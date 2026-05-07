// Package processing provides video processing orchestration.
package processing

import (
	"bufio"
	"fmt"
	"os/exec"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"sync"

	"github.com/five82/drapto/internal/ffprobe"
)

// Crop detection constants
const (
	// cropDetectionConcurrency is the maximum number of concurrent crop detection samples.
	cropDetectionConcurrency = 8

	// cropSampleStart is the start position for sampling (15% of video = 30/200).
	cropSampleStart = 30

	// cropSampleEnd is the end position for sampling (85% of video = 170/200).
	cropSampleEnd = 170

	// cropSampleDivisor converts sample positions to percentages.
	cropSampleDivisor = 200.0

	// cropThresholdSDR is the black bar detection threshold for SDR content.
	cropThresholdSDR = 16

	// cropThresholdHDR is the black bar detection threshold for HDR content.
	cropThresholdHDR = 100

	// cropSampleFrames is the number of frames to sample at each position.
	cropSampleFrames = 10

	// cropRound is the rounding value for cropdetect filter.
	cropRound = 2

	// cropReset is the reset value for cropdetect filter.
	cropReset = 1
)

// CropCandidate represents a detected crop value and its frequency.
type CropCandidate struct {
	Crop    string  // The crop value (e.g., "3840:1632:0:264")
	Count   int     // Number of samples with this crop
	Percent float64 // Percentage of total samples
}

// CropResult contains the result of crop detection.
type CropResult struct {
	CropFilter     string          // The crop filter string (e.g., "crop=1920:800:0:140")
	Required       bool            // Whether cropping is required
	MultipleRatios bool            // Whether multiple aspect ratios were detected
	Message        string          // Human-readable message about the crop result
	Candidates     []CropCandidate // All detected crop values with frequencies (for debugging)
	TotalSamples   int             // Total number of samples analyzed
}

// cropRegex matches FFmpeg cropdetect output.
var cropRegex = regexp.MustCompile(`crop=(\d+:\d+:\d+:\d+)`)

// DetectCrop performs crop detection on a video file.
// It samples 141 points from 15-85% of the video to detect black bars.
func DetectCrop(inputPath string, props *ffprobe.VideoProperties, disableCrop bool) CropResult {
	if disableCrop {
		return CropResult{
			Required: false,
			Message:  "Skipped",
		}
	}

	// Set threshold based on HDR status
	threshold := uint32(cropThresholdSDR)
	if props.HDRInfo.IsHDR {
		threshold = cropThresholdHDR
	}

	// Sample every 0.5% from 15% to 85% (141 points total)
	var samplePoints []float64
	for i := cropSampleStart; i <= cropSampleEnd; i++ {
		samplePoints = append(samplePoints, float64(i)/cropSampleDivisor)
	}
	numSamples := len(samplePoints)

	// Process samples in parallel
	cropCounts := make(map[string]int)
	var mu sync.Mutex
	var wg sync.WaitGroup

	// Use a semaphore to limit concurrency
	sem := make(chan struct{}, cropDetectionConcurrency)

	for _, position := range samplePoints {
		wg.Add(1)
		go func(pos float64) {
			defer wg.Done()
			sem <- struct{}{}
			defer func() { <-sem }()

			startTime := props.DurationSecs * pos
			crop := sampleCropAtPosition(inputPath, startTime, threshold)
			if crop != "" {
				mu.Lock()
				cropCounts[crop]++
				mu.Unlock()
			}
		}(position)
	}

	wg.Wait()

	sampleMsg := fmt.Sprintf("Analyzed %d samples", numSamples)

	return analyzeCropCounts(cropCounts, props.Width, props.Height, sampleMsg, numSamples)
}

type cropCount struct {
	crop  string
	count int
}

type cropMargins struct {
	top    uint32
	bottom uint32
	left   uint32
	right  uint32
}

func analyzeCropCounts(cropCounts map[string]int, sourceWidth, sourceHeight uint32, sampleMsg string, expectedSamples int) CropResult {
	if len(cropCounts) == 0 {
		return CropResult{
			Required:     false,
			Message:      sampleMsg,
			TotalSamples: expectedSamples,
		}
	}

	sorted := sortedCropCounts(cropCounts)
	totalSamples := 0
	for _, cc := range sorted {
		totalSamples += cc.count
	}
	candidates := cropCandidates(sorted, totalSamples)

	crop, ok := leastAggressiveCrop(sorted, sourceWidth, sourceHeight)
	if !ok {
		return CropResult{
			Required:     false,
			Message:      sampleMsg,
			Candidates:   candidates,
			TotalSamples: totalSamples,
		}
	}

	if !isEffectiveCrop(crop, sourceWidth, sourceHeight) {
		result := CropResult{
			Required:     false,
			Message:      sampleMsg,
			Candidates:   candidates,
			TotalSamples: totalSamples,
		}
		if len(cropCounts) > 1 {
			result.MultipleRatios = true
			result.Message = "Multiple aspect ratios detected"
		}
		return result
	}

	return CropResult{
		CropFilter:   "crop=" + crop,
		Required:     true,
		Message:      "Black bars detected",
		Candidates:   candidates,
		TotalSamples: totalSamples,
	}
}

func sortedCropCounts(cropCounts map[string]int) []cropCount {
	sorted := make([]cropCount, 0, len(cropCounts))
	for crop, count := range cropCounts {
		sorted = append(sorted, cropCount{crop: crop, count: count})
	}
	sort.Slice(sorted, func(i, j int) bool {
		if sorted[i].count == sorted[j].count {
			return sorted[i].crop < sorted[j].crop
		}
		return sorted[i].count > sorted[j].count
	})
	return sorted
}

func cropCandidates(sorted []cropCount, totalSamples int) []CropCandidate {
	candidates := make([]CropCandidate, 0, len(sorted))
	for _, cc := range sorted {
		candidates = append(candidates, CropCandidate{
			Crop:    cc.crop,
			Count:   cc.count,
			Percent: float64(cc.count) / float64(totalSamples) * 100,
		})
	}
	return candidates
}

func leastAggressiveCrop(sorted []cropCount, sourceWidth, sourceHeight uint32) (string, bool) {
	var best cropMargins
	haveBest := false
	for _, cc := range sorted {
		margins, ok := parseCropMargins(cc.crop, sourceWidth, sourceHeight)
		if !ok {
			continue
		}
		margins = margins.even()
		if !haveBest {
			best = margins
			haveBest = true
			continue
		}
		best = minCropMargins(best, margins)
	}
	if !haveBest {
		return "", false
	}
	return best.crop(sourceWidth, sourceHeight)
}

func parseCropMargins(crop string, sourceWidth, sourceHeight uint32) (cropMargins, bool) {
	parts := strings.Split(crop, ":")
	if len(parts) != 4 {
		return cropMargins{}, false
	}

	width, err := strconv.ParseUint(parts[0], 10, 32)
	if err != nil {
		return cropMargins{}, false
	}
	height, err := strconv.ParseUint(parts[1], 10, 32)
	if err != nil {
		return cropMargins{}, false
	}
	left, err := strconv.ParseUint(parts[2], 10, 32)
	if err != nil {
		return cropMargins{}, false
	}
	top, err := strconv.ParseUint(parts[3], 10, 32)
	if err != nil {
		return cropMargins{}, false
	}

	w, h, x, y := uint32(width), uint32(height), uint32(left), uint32(top)
	if x > sourceWidth || w > sourceWidth-x || y > sourceHeight || h > sourceHeight-y {
		return cropMargins{}, false
	}
	return cropMargins{
		top:    y,
		bottom: sourceHeight - y - h,
		left:   x,
		right:  sourceWidth - x - w,
	}, true
}

func minCropMargins(a, b cropMargins) cropMargins {
	return cropMargins{
		top:    min(a.top, b.top),
		bottom: min(a.bottom, b.bottom),
		left:   min(a.left, b.left),
		right:  min(a.right, b.right),
	}
}

func (m cropMargins) even() cropMargins {
	return cropMargins{
		top:    m.top &^ 1,
		bottom: m.bottom &^ 1,
		left:   m.left &^ 1,
		right:  m.right &^ 1,
	}
}

func (m cropMargins) crop(sourceWidth, sourceHeight uint32) (string, bool) {
	if m.left >= sourceWidth || m.right >= sourceWidth-m.left || m.top >= sourceHeight || m.bottom >= sourceHeight-m.top {
		return "", false
	}
	width := sourceWidth - m.left - m.right
	height := sourceHeight - m.top - m.bottom
	if width == 0 || height == 0 || width%2 != 0 || height%2 != 0 {
		return "", false
	}
	return fmt.Sprintf("%d:%d:%d:%d", width, height, m.left, m.top), true
}

// sampleCropAtPosition samples crop detection at a specific position.
func sampleCropAtPosition(inputPath string, startTime float64, threshold uint32) string {
	cmd := exec.Command("ffmpeg",
		"-hide_banner",
		"-ss", fmt.Sprintf("%.2f", startTime),
		"-i", inputPath,
		"-vframes", fmt.Sprintf("%d", cropSampleFrames),
		"-vf", fmt.Sprintf("cropdetect=limit=%d:round=%d:reset=%d", threshold, cropRound, cropReset),
		"-f", "null",
		"-",
	)

	stderr, err := cmd.StderrPipe()
	if err != nil {
		return ""
	}

	if err := cmd.Start(); err != nil {
		return ""
	}

	// Parse cropdetect output
	cropCounts := make(map[string]int)
	scanner := bufio.NewScanner(stderr)
	for scanner.Scan() {
		line := scanner.Text()
		if matches := cropRegex.FindStringSubmatch(line); len(matches) >= 2 {
			cropValue := matches[1]
			if isValidCropFormat(cropValue) {
				cropCounts[cropValue]++
			}
		}
	}

	_ = cmd.Wait()

	// Return the most common crop value
	if len(cropCounts) == 0 {
		return ""
	}

	var bestCrop string
	bestCount := 0
	for crop, count := range cropCounts {
		if count > bestCount {
			bestCrop = crop
			bestCount = count
		}
	}

	return bestCrop
}

// isValidCropFormat validates that a crop string is in format w:h:x:y.
func isValidCropFormat(crop string) bool {
	parts := strings.Split(crop, ":")
	if len(parts) != 4 {
		return false
	}

	for _, part := range parts {
		if _, err := strconv.ParseUint(part, 10, 32); err != nil {
			return false
		}
	}

	return true
}

// isEffectiveCrop checks if a crop filter actually removes pixels.
func isEffectiveCrop(crop string, sourceWidth, sourceHeight uint32) bool {
	parts := strings.Split(crop, ":")
	if len(parts) < 2 {
		return true // Can't parse, assume effective
	}

	cropWidth, err := strconv.ParseUint(parts[0], 10, 32)
	if err != nil {
		return true
	}

	cropHeight, err := strconv.ParseUint(parts[1], 10, 32)
	if err != nil {
		return true
	}

	// If crop dimensions match source, no pixels are removed
	return uint32(cropWidth) != sourceWidth || uint32(cropHeight) != sourceHeight
}

// GetOutputDimensions calculates final output dimensions after crop.
func GetOutputDimensions(originalWidth, originalHeight uint32, cropFilter string) (uint32, uint32) {
	if cropFilter == "" {
		return originalWidth, originalHeight
	}

	// Strip "crop=" prefix if present
	params := strings.TrimPrefix(cropFilter, "crop=")
	parts := strings.Split(params, ":")

	if len(parts) >= 2 {
		if width, err := strconv.ParseUint(parts[0], 10, 32); err == nil {
			if height, err := strconv.ParseUint(parts[1], 10, 32); err == nil {
				return uint32(width), uint32(height)
			}
		}
	}

	return originalWidth, originalHeight
}
