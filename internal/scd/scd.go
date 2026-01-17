// Package scd provides scene change detection functionality using the drapto-scd helper binary.
package scd

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
)

const scdBinaryName = "drapto-scd"

// DetectScenes runs scene change detection on a video file.
// It writes the detected scene boundaries to the specified output file.
func DetectScenes(videoPath, sceneFile string, fpsNum, fpsDen uint32, totalFrames int, showProgress bool) error {
	// Check if drapto-scd is available
	scdPath, err := exec.LookPath(scdBinaryName)
	if err != nil {
		return fmt.Errorf("%s not found in PATH: %w", scdBinaryName, err)
	}

	args := []string{
		"--input", videoPath,
		"--output", sceneFile,
		"--fps-num", fmt.Sprintf("%d", fpsNum),
		"--fps-den", fmt.Sprintf("%d", fpsDen),
		"--total-frames", fmt.Sprintf("%d", totalFrames),
	}

	if showProgress {
		args = append(args, "--progress")
	}

	cmd := exec.Command(scdPath, args...)
	cmd.Stderr = os.Stderr

	if err := cmd.Run(); err != nil {
		return fmt.Errorf("scene detection failed: %w", err)
	}

	return nil
}

// DetectScenesIfNeeded runs scene detection only if the scene file doesn't exist.
func DetectScenesIfNeeded(videoPath, workDir string, fpsNum, fpsDen uint32, totalFrames int, showProgress bool) (string, error) {
	sceneFile := filepath.Join(workDir, "scenes.txt")

	// Check if scene file already exists
	if _, err := os.Stat(sceneFile); err == nil {
		return sceneFile, nil
	}

	// Run scene detection
	if err := DetectScenes(videoPath, sceneFile, fpsNum, fpsDen, totalFrames, showProgress); err != nil {
		return "", err
	}

	return sceneFile, nil
}

// IsSCDBinaryAvailable checks if the drapto-scd binary is available in PATH.
func IsSCDBinaryAvailable() bool {
	_, err := exec.LookPath(scdBinaryName)
	return err == nil
}

// GetSCDBinaryPath returns the path to the drapto-scd binary if available.
func GetSCDBinaryPath() (string, error) {
	return exec.LookPath(scdBinaryName)
}
