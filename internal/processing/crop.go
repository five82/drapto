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

	// cropDominantRatio is the minimum ratio for a crop to be considered dominant.
	cropDominantRatio = 0.8

	// cropClearWinnerRatio is the minimum ratio for a "clear winner with noise" scenario.
	// Used when the top candidate has this ratio AND second-best is below cropNoiseThreshold.
	cropClearWinnerRatio = 0.6

	// cropNoiseThreshold is the maximum ratio for the second-best candidate to be
	// considered noise rather than a genuine alternative aspect ratio.
	cropNoiseThreshold = 0.05

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

	// Analyze results
	if len(cropCounts) == 0 {
		return CropResult{
			Required:     false,
			Message:      sampleMsg,
			TotalSamples: numSamples,
		}
	}

	// Build sorted candidate list for all cases
	type cropCount struct {
		crop  string
		count int
	}
	var sorted []cropCount
	totalSamples := 0
	for crop, count := range cropCounts {
		sorted = append(sorted, cropCount{crop, count})
		totalSamples += count
	}
	sort.Slice(sorted, func(i, j int) bool {
		return sorted[i].count > sorted[j].count
	})

	// Build candidates slice for debugging
	buildCandidates := func() []CropCandidate {
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

	if len(cropCounts) == 1 {
		// Single crop detected
		crop := sorted[0].crop
		if !isEffectiveCrop(crop, props.Width, props.Height) {
			return CropResult{
				Required:     false,
				Message:      sampleMsg,
				Candidates:   buildCandidates(),
				TotalSamples: totalSamples,
			}
		}
		return CropResult{
			CropFilter:   "crop=" + crop,
			Required:     true,
			Message:      "Black bars detected",
			Candidates:   buildCandidates(),
			TotalSamples: totalSamples,
		}
	}

	// Multiple crops detected - find the most common
	mostCommon := sorted[0]
	ratio := float64(mostCommon.count) / float64(totalSamples)

	// If one crop is dominant (>80% of samples), use it
	if ratio > cropDominantRatio {
		if !isEffectiveCrop(mostCommon.crop, props.Width, props.Height) {
			return CropResult{
				Required:     false,
				Message:      sampleMsg,
				Candidates:   buildCandidates(),
				TotalSamples: totalSamples,
			}
		}
		return CropResult{
			CropFilter:   "crop=" + mostCommon.crop,
			Required:     true,
			Message:      "Black bars detected",
			Candidates:   buildCandidates(),
			TotalSamples: totalSamples,
		}
	}

	// Check for "clear winner with noise" scenario:
	// Top candidate has >60% AND second-best has <5% (noise from HDR dark scenes, etc.)
	if ratio > cropClearWinnerRatio && len(sorted) > 1 {
		secondRatio := float64(sorted[1].count) / float64(totalSamples)
		if secondRatio < cropNoiseThreshold {
			if !isEffectiveCrop(mostCommon.crop, props.Width, props.Height) {
				return CropResult{
					Required:     false,
					Message:      sampleMsg,
					Candidates:   buildCandidates(),
					TotalSamples: totalSamples,
				}
			}
			return CropResult{
				CropFilter:   "crop=" + mostCommon.crop,
				Required:     true,
				Message:      "Black bars detected (clear winner with noise)",
				Candidates:   buildCandidates(),
				TotalSamples: totalSamples,
			}
		}
	}

	// Multiple significant aspect ratios - don't crop
	return CropResult{
		Required:       false,
		MultipleRatios: true,
		Message:        "Multiple aspect ratios detected",
		Candidates:     buildCandidates(),
		TotalSamples:   totalSamples,
	}
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
