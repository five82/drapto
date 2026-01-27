// Package util provides utility functions for formatting and common operations.
package util

import (
	"fmt"
	"strconv"
	"strings"
)

const (
	KiB = 1024
	MiB = KiB * 1024
	GiB = MiB * 1024

	// SecondsPerMinute is the number of seconds in a minute.
	SecondsPerMinute = 60
	// SecondsPerHour is the number of seconds in an hour.
	SecondsPerHour = 3600
)

// FormatBytes formats bytes with appropriate binary units (B, KiB, MiB, GiB).
func FormatBytes(bytes uint64) string {
	bf := float64(bytes)
	switch {
	case bf >= GiB:
		return fmt.Sprintf("%.2f GiB", bf/GiB)
	case bf >= MiB:
		return fmt.Sprintf("%.2f MiB", bf/MiB)
	case bf >= KiB:
		return fmt.Sprintf("%.2f KiB", bf/KiB)
	default:
		return fmt.Sprintf("%d B", bytes)
	}
}

// FormatBytesReadable formats bytes showing both MB and GB values.
func FormatBytesReadable(bytes uint64) string {
	bf := float64(bytes)
	mb := bf / float64(MiB)
	gb := bf / float64(GiB)
	return fmt.Sprintf("%.2f MB (%.2f GB)", mb, gb)
}

// FormatDuration formats seconds as HH:MM:SS.
func FormatDuration(seconds float64) string {
	if seconds < 0 || seconds != seconds { // NaN check
		return "??:??:??"
	}

	totalSecs := int64(seconds)
	hours := totalSecs / SecondsPerHour
	minutes := (totalSecs % SecondsPerHour) / SecondsPerMinute
	secs := totalSecs % SecondsPerMinute
	return fmt.Sprintf("%02d:%02d:%02d", hours, minutes, secs)
}

// FormatDurationFromSecs formats seconds as HH:MM:SS from an int64.
func FormatDurationFromSecs(secs int64) string {
	hours := secs / SecondsPerHour
	minutes := (secs % SecondsPerHour) / SecondsPerMinute
	seconds := secs % SecondsPerMinute
	return fmt.Sprintf("%02d:%02d:%02d", hours, minutes, seconds)
}

// ParseFFmpegTime parses FFmpeg time string (HH:MM:SS.MS) to seconds.
func ParseFFmpegTime(timeStr string) (float64, bool) {
	parts := strings.Split(timeStr, ":")
	if len(parts) != 3 {
		return 0, false
	}

	hours, err := strconv.ParseFloat(parts[0], 64)
	if err != nil {
		return 0, false
	}

	minutes, err := strconv.ParseFloat(parts[1], 64)
	if err != nil {
		return 0, false
	}

	seconds, err := strconv.ParseFloat(parts[2], 64)
	if err != nil {
		return 0, false
	}

	return hours*SecondsPerHour + minutes*SecondsPerMinute + seconds, true
}

// CalculateSizeReduction calculates the percentage size reduction.
// Returns positive values for size reduction, negative for size increase.
func CalculateSizeReduction(inputSize, outputSize uint64) float64 {
	if inputSize == 0 {
		return 0
	}
	return (float64(inputSize) - float64(outputSize)) / float64(inputSize) * 100
}
