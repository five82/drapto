// Package keyframe provides fixed-length chunk generation for video encoding.
package keyframe

import (
	"fmt"
	"os"
	"path/filepath"
)

// ChunkDurationForResolution returns the appropriate chunk duration based on resolution.
// Longer chunks provide better encoder efficiency and reduce concatenation overhead.
// 4K: 45s (slower encode, needs longer warmup)
// 1080p: 30s (balanced)
// SD/720p: 20s (faster encode, can use shorter chunks)
func ChunkDurationForResolution(width, height uint32) float64 {
	if width > 2560 || height > 1440 {
		return 45.0 // 4K
	}
	if width >= 1920 || height >= 1080 {
		return 30.0 // 1080p
	}
	return 20.0 // SD, 720p
}

// GenerateFixedChunks creates chunk boundaries at fixed time intervals.
// Returns a sorted slice of frame numbers where chunks start.
func GenerateFixedChunks(totalFrames int, fpsNum, fpsDen uint32, chunkDurationSecs float64) []int {
	if fpsDen == 0 || totalFrames <= 0 {
		return []int{0}
	}

	fps := float64(fpsNum) / float64(fpsDen)
	framesPerChunk := int(fps * chunkDurationSecs)
	if framesPerChunk < 1 {
		framesPerChunk = 1
	}

	var keyframes []int
	for frame := 0; frame < totalFrames; frame += framesPerChunk {
		keyframes = append(keyframes, frame)
	}

	// Ensure we have at least frame 0
	if len(keyframes) == 0 {
		keyframes = []int{0}
	}

	return keyframes
}

// ExtractKeyframesIfNeeded generates fixed-length chunks and writes them to scenes.txt if not already present.
// Returns the path to the scenes.txt file.
func ExtractKeyframesIfNeeded(videoPath, workDir string, fpsNum, fpsDen uint32, totalFrames int, width, height uint32) (string, error) {
	sceneFile := filepath.Join(workDir, "scenes.txt")

	// Check if scene file already exists
	if _, err := os.Stat(sceneFile); err == nil {
		return sceneFile, nil
	}

	// Determine chunk duration based on resolution
	chunkDuration := ChunkDurationForResolution(width, height)

	// Generate fixed-length chunks
	keyframes := GenerateFixedChunks(totalFrames, fpsNum, fpsDen, chunkDuration)

	// Write to scenes.txt
	if err := writeSceneFile(sceneFile, keyframes); err != nil {
		return "", err
	}

	return sceneFile, nil
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
