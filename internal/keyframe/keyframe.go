// Package keyframe provides scene detection from video files using ffmpeg's scene filter.
// This detects actual scene changes rather than just I-frame positions.
package keyframe

import (
	"bufio"
	"fmt"
	"math"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
)

// DefaultSceneThreshold is the default threshold for scene change detection.
// Higher values = fewer scene changes detected. Range is 0.0 to 1.0.
const DefaultSceneThreshold = 0.5

// DetectScenes runs ffmpeg's scene detection filter on a video file.
// Returns a sorted slice of frame numbers where scene changes occur.
func DetectScenes(videoPath string, fpsNum, fpsDen uint32, threshold float64) ([]int, error) {
	if threshold <= 0 {
		threshold = DefaultSceneThreshold
	}

	// Use ffmpeg scene filter to detect scene changes
	// The showinfo filter outputs pts_time which we convert to frame numbers
	cmd := exec.Command("ffmpeg",
		"-i", videoPath,
		"-vf", fmt.Sprintf("select='gt(scene,%g)',showinfo", threshold),
		"-an",
		"-f", "null",
		"-",
	)

	// Scene detection output goes to stderr
	stderr, err := cmd.StderrPipe()
	if err != nil {
		return nil, fmt.Errorf("failed to create stderr pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return nil, fmt.Errorf("failed to start ffmpeg: %w", err)
	}

	// Parse showinfo output for pts_time values
	// Example: [Parsed_showinfo_1 @ 0x...] n:   0 pts: 135052 pts_time:135.052 ...
	ptsTimeRegex := regexp.MustCompile(`pts_time:(\d+\.?\d*)`)
	fps := float64(fpsNum) / float64(fpsDen)

	var sceneFrames []int
	scanner := bufio.NewScanner(stderr)
	for scanner.Scan() {
		line := scanner.Text()
		matches := ptsTimeRegex.FindStringSubmatch(line)
		if len(matches) >= 2 {
			ptsTime, err := strconv.ParseFloat(matches[1], 64)
			if err != nil {
				continue
			}
			// Convert pts_time to frame number
			frameNum := int(math.Round(ptsTime * fps))
			sceneFrames = append(sceneFrames, frameNum)
		}
	}

	if err := scanner.Err(); err != nil {
		_ = cmd.Wait()
		return nil, fmt.Errorf("error reading ffmpeg output: %w", err)
	}

	// Wait for ffmpeg to finish (ignore exit code since it outputs to null)
	_ = cmd.Wait()

	// Ensure we always start at frame 0
	if len(sceneFrames) == 0 || sceneFrames[0] != 0 {
		sceneFrames = append([]int{0}, sceneFrames...)
	}

	// Sort and deduplicate
	sort.Ints(sceneFrames)
	sceneFrames = dedupe(sceneFrames)

	return sceneFrames, nil
}

// ExtractKeyframesIfNeeded detects scenes and writes them to scenes.txt if not already present.
// Returns the path to the scenes.txt file.
// minDuration specifies the minimum chunk duration in seconds (0 to disable merging).
func ExtractKeyframesIfNeeded(videoPath, workDir string, fpsNum, fpsDen uint32, totalFrames int, threshold, minDuration float64) (string, error) {
	sceneFile := filepath.Join(workDir, "scenes.txt")

	// Check if scene file already exists
	if _, err := os.Stat(sceneFile); err == nil {
		return sceneFile, nil
	}

	// Detect scenes using ffmpeg scene filter
	scenes, err := DetectScenes(videoPath, fpsNum, fpsDen, threshold)
	if err != nil {
		return "", fmt.Errorf("scene detection failed: %w", err)
	}

	// Calculate max frames per scene
	maxFrames := CalculateMaxFrames(fpsNum, fpsDen)

	// Split long scenes
	finalFrames := SplitLongScenes(scenes, totalFrames, maxFrames)

	// Merge short scenes if minDuration is set
	if minDuration > 0 {
		minFrames := CalculateMinFrames(fpsNum, fpsDen, minDuration)
		finalFrames = MergeShortScenes(finalFrames, totalFrames, minFrames)
	}

	// Write to scenes.txt
	if err := writeSceneFile(sceneFile, finalFrames); err != nil {
		return "", err
	}

	return sceneFile, nil
}

// CalculateMaxFrames calculates the maximum scene length in frames.
// Returns min(fps * 30, 1000).
func CalculateMaxFrames(fpsNum, fpsDen uint32) int {
	if fpsDen == 0 {
		return 1000 // Safe default
	}

	fps := float64(fpsNum) / float64(fpsDen)
	maxFromFPS := int(fps * 30)

	if maxFromFPS < 1000 {
		return maxFromFPS
	}
	return 1000
}

// CalculateMinFrames calculates the minimum scene length in frames from duration.
func CalculateMinFrames(fpsNum, fpsDen uint32, minDurationSecs float64) int {
	if fpsDen == 0 || minDurationSecs <= 0 {
		return 0
	}

	fps := float64(fpsNum) / float64(fpsDen)
	return int(fps * minDurationSecs)
}

// SplitLongScenes splits scenes that exceed maxFrames into smaller chunks.
// When a scene is longer than maxFrames, it is split evenly into chunks
// that are as close to equal length as possible while staying under maxFrames.
func SplitLongScenes(keyframes []int, totalFrames, maxFrames int) []int {
	if len(keyframes) == 0 {
		return []int{0}
	}

	result := make([]int, 0, len(keyframes))

	for i := 0; i < len(keyframes); i++ {
		start := keyframes[i]
		end := totalFrames
		if i+1 < len(keyframes) {
			end = keyframes[i+1]
		}

		result = append(result, start)

		sceneLen := end - start
		if sceneLen > maxFrames {
			// Calculate how many chunks we need
			numChunks := (sceneLen + maxFrames - 1) / maxFrames
			chunkSize := sceneLen / numChunks

			// Add intermediate split points
			for j := 1; j < numChunks; j++ {
				split := start + j*chunkSize
				if split < end {
					result = append(result, split)
				}
			}
		}
	}

	// Sort and deduplicate
	sort.Ints(result)
	result = dedupe(result)

	return result
}

// MergeShortScenes merges scenes shorter than minFrames with adjacent scenes.
// Short scenes are merged with whichever neighbor results in a smaller combined chunk,
// keeping chunk sizes more balanced.
func MergeShortScenes(keyframes []int, totalFrames, minFrames int) []int {
	if len(keyframes) <= 1 || minFrames <= 0 {
		return keyframes
	}

	// Work with a copy to allow iterative merging
	result := make([]int, len(keyframes))
	copy(result, keyframes)

	// Keep merging until no short scenes remain
	for {
		merged := false

		for i := 0; i < len(result); i++ {
			start := result[i]
			end := totalFrames
			if i+1 < len(result) {
				end = result[i+1]
			}

			sceneLen := end - start
			if sceneLen >= minFrames {
				continue
			}

			// Scene is too short, need to merge
			// First scene (i=0): can only merge with next by removing keyframe at index 1
			if i == 0 {
				if len(result) > 1 {
					result = append(result[:1], result[2:]...)
					merged = true
					break
				}
				continue
			}

			// Calculate neighbor sizes
			prevStart := 0
			if i > 1 {
				prevStart = result[i-1]
			}
			prevLen := start - prevStart

			nextEnd := totalFrames
			if i+2 < len(result) {
				nextEnd = result[i+2]
			}
			nextLen := nextEnd - end

			// Decide which neighbor to merge with
			// Merge with smaller neighbor to keep chunks balanced
			// If this is the last scene, must merge with previous
			if i+1 >= len(result) || prevLen <= nextLen {
				// Merge with previous: remove this keyframe
				result = append(result[:i], result[i+1:]...)
			} else {
				// Merge with next: remove next keyframe
				if i+1 < len(result) {
					result = append(result[:i+1], result[i+2:]...)
				}
			}

			merged = true
			break // Restart scanning after a merge
		}

		if !merged {
			break
		}
	}

	return result
}

// dedupe removes duplicate values from a sorted slice.
func dedupe(sorted []int) []int {
	if len(sorted) <= 1 {
		return sorted
	}

	result := make([]int, 0, len(sorted))
	result = append(result, sorted[0])

	for i := 1; i < len(sorted); i++ {
		if sorted[i] != sorted[i-1] {
			result = append(result, sorted[i])
		}
	}

	return result
}

// writeSceneFile writes frame numbers to a scenes.txt file.
func writeSceneFile(path string, frames []int) error {
	file, err := os.Create(path)
	if err != nil {
		return fmt.Errorf("failed to create scene file: %w", err)
	}
	defer func() { _ = file.Close() }()

	for _, frame := range frames {
		if _, err := fmt.Fprintf(file, "%d\n", frame); err != nil {
			return fmt.Errorf("failed to write scene file: %w", err)
		}
	}

	return nil
}
