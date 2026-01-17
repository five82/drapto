// Package discovery provides file discovery for video processing.
package discovery

import (
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/five82/drapto/internal/util"
)

// DiscoveryLogger defines the interface for discovery logging.
type DiscoveryLogger interface {
	Info(format string, args ...any)
	Debug(format string, args ...any)
}

// DiscoveryResult contains the results of file discovery with metadata.
type DiscoveryResult struct {
	Files        []string
	SkippedCount int
	Errors       []error
}

// FindVideoFiles finds video files in the given directory.
// Returns files sorted alphabetically by filename.
func FindVideoFiles(inputDir string) ([]string, error) {
	// Validate input directory
	info, err := os.Stat(inputDir)
	if err != nil {
		return nil, fmt.Errorf("directory does not exist: %s", inputDir)
	}
	if !info.IsDir() {
		return nil, fmt.Errorf("%s is not a directory", inputDir)
	}

	entries, err := os.ReadDir(inputDir)
	if err != nil {
		return nil, fmt.Errorf("cannot read directory %s: %w", inputDir, err)
	}

	var files []string
	skippedCount := 0

	for _, entry := range entries {
		if entry.IsDir() {
			continue
		}

		name := entry.Name()

		// Skip hidden files
		if strings.HasPrefix(name, ".") {
			continue
		}

		fullPath := filepath.Join(inputDir, name)
		if util.IsVideoFile(fullPath) {
			files = append(files, fullPath)
		} else {
			skippedCount++
		}
	}

	if len(files) == 0 {
		return nil, fmt.Errorf("no video files found in %s", inputDir)
	}

	// Sort alphabetically
	sort.Slice(files, func(i, j int) bool {
		return strings.ToLower(filepath.Base(files[i])) < strings.ToLower(filepath.Base(files[j]))
	})

	return files, nil
}

// FindVideoFilesWithLogging finds video files and logs discovery progress.
// Logs the first 5 files found plus a count summary.
func FindVideoFilesWithLogging(inputDir string, logger DiscoveryLogger) (*DiscoveryResult, error) {
	result := &DiscoveryResult{}

	// Validate input directory
	info, err := os.Stat(inputDir)
	if err != nil {
		return nil, fmt.Errorf("directory does not exist: %s", inputDir)
	}
	if !info.IsDir() {
		return nil, fmt.Errorf("%s is not a directory", inputDir)
	}

	entries, err := os.ReadDir(inputDir)
	if err != nil {
		return nil, fmt.Errorf("cannot read directory %s: %w", inputDir, err)
	}

	for _, entry := range entries {
		if entry.IsDir() {
			continue
		}

		name := entry.Name()

		// Skip hidden files
		if strings.HasPrefix(name, ".") {
			continue
		}

		fullPath := filepath.Join(inputDir, name)
		if util.IsVideoFile(fullPath) {
			result.Files = append(result.Files, fullPath)
		} else {
			result.SkippedCount++
		}
	}

	if len(result.Files) == 0 {
		return nil, fmt.Errorf("no video files found in %s", inputDir)
	}

	// Sort alphabetically
	sort.Slice(result.Files, func(i, j int) bool {
		return strings.ToLower(filepath.Base(result.Files[i])) < strings.ToLower(filepath.Base(result.Files[j]))
	})

	// Log discovery results
	if logger != nil {
		logDiscoveredFiles(result.Files, logger)
	}

	return result, nil
}

// logDiscoveredFiles logs the first 5 discovered files plus a count.
func logDiscoveredFiles(files []string, logger DiscoveryLogger) {
	if len(files) == 0 {
		logger.Info("No video files found")
		return
	}

	logger.Info("Found %d video file(s)", len(files))

	// Log first 5 files
	maxToLog := min(5, len(files))

	for i := range maxToLog {
		logger.Debug("  %s", filepath.Base(files[i]))
	}

	if len(files) > 5 {
		logger.Debug("  ... and %d more", len(files)-5)
	}
}

