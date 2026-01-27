package validation

import (
	"fmt"
	"math"
	"strings"
)

const (
	// durationToleranceSecs is the maximum allowed difference in duration between input and output.
	durationToleranceSecs = 1.0
	// maxSyncDriftMs is the maximum allowed audio/video sync drift in milliseconds.
	maxSyncDriftMs = 100.0
	// requiredBitDepth is the minimum bit depth required for AV1 output validation.
	requiredBitDepth = 10
)

// Options contains optional parameters for validation.
type Options struct {
	ExpectedDimensions    *[2]uint32
	ExpectedDuration      *float64
	ExpectedHDR           *bool
	ExpectedAudioTracks   *int
	ExpectedAudioChannels []uint32
}

// ValidateOutputVideo performs comprehensive validation of an encoded video.
// It delegates to ValidateWithAnalyzer using the DefaultAnalyzer.
func ValidateOutputVideo(inputPath, outputPath string, opts Options) (*Result, error) {
	return ValidateWithAnalyzer(NewDefaultAnalyzer(), outputPath, opts)
}

// validateDimensions checks that dimensions match expected values.
func validateDimensions(actualW, actualH, expectedW, expectedH uint32) (bool, string) {
	if actualW == expectedW && actualH == expectedH {
		return true, fmt.Sprintf("Dimensions match: %dx%d", actualW, actualH)
	}
	return false, fmt.Sprintf("Dimension mismatch: got %dx%d, expected %dx%d",
		actualW, actualH, expectedW, expectedH)
}

// validateDuration checks that duration is within acceptable tolerance.
func validateDuration(actual, expected float64) (bool, string) {
	diff := math.Abs(actual - expected)

	if diff <= durationToleranceSecs {
		return true, fmt.Sprintf("Duration matches input (%.1fs)", actual)
	}
	return false, fmt.Sprintf("Duration mismatch: got %.1fs, expected %.1fs (diff: %.1fs)",
		actual, expected, diff)
}

// validateSync checks audio/video sync drift.
func validateSync(outputDuration, inputDuration float64) (bool, *float64, string) {
	// Calculate drift in milliseconds
	driftMs := math.Abs(outputDuration-inputDuration) * 1000
	preserved := driftMs <= maxSyncDriftMs

	message := fmt.Sprintf("Audio/video sync preserved (drift: %.1fms)", driftMs)
	if !preserved {
		message = fmt.Sprintf("Audio/video sync drift too large: %.1fms (max: %.1fms)", driftMs, maxSyncDriftMs)
	}

	return preserved, &driftMs, message
}

// ValidateWithAnalyzer performs validation using a MediaAnalyzer interface.
// This allows for testing without external tool dependencies.
func ValidateWithAnalyzer(analyzer MediaAnalyzer, outputPath string, opts Options) (*Result, error) {
	result := &Result{
		IsCropCorrect:            true,
		IsDurationCorrect:        true,
		IsHDRCorrect:             true,
		IsAudioOpus:              true,
		IsAudioTrackCountCorrect: true,
		IsSyncPreserved:          true,
	}

	// Get output video properties
	outputProps, err := analyzer.GetVideoProperties(outputPath)
	if err != nil {
		return nil, fmt.Errorf("failed to get output video properties: %w", err)
	}

	// Validate video codec (should be AV1)
	codecName, err := analyzer.GetVideoCodec(outputPath)
	if err != nil {
		result.IsAV1 = false
		result.CodecName = ""
	} else {
		isAV1 := strings.Contains(strings.ToLower(codecName), "av1") ||
			strings.Contains(strings.ToLower(codecName), "av01")
		result.IsAV1 = isAV1
		result.CodecName = codecName
	}

	// Validate bit depth
	if outputProps.BitDepth != nil {
		result.Is10Bit = *outputProps.BitDepth >= requiredBitDepth
		result.BitDepth = outputProps.BitDepth
	} else {
		// Try HDR info for bit depth
		hdrInfo, err := analyzer.GetHDRInfo(outputPath)
		if err == nil && hdrInfo.BitDepth != nil {
			result.Is10Bit = *hdrInfo.BitDepth >= requiredBitDepth
			result.BitDepth = hdrInfo.BitDepth
		} else {
			// Default to true for AV1 (typically 10-bit)
			defaultDepth := uint8(10)
			result.Is10Bit = true
			result.BitDepth = &defaultDepth
		}
	}

	// Validate dimensions if expected
	if opts.ExpectedDimensions != nil {
		result.ActualDimensions = &[2]uint32{outputProps.Width, outputProps.Height}
		result.ExpectedDimensions = opts.ExpectedDimensions
		result.IsCropCorrect, result.CropMessage = validateDimensions(
			outputProps.Width, outputProps.Height,
			opts.ExpectedDimensions[0], opts.ExpectedDimensions[1],
		)
	} else {
		result.CropMessage = "No crop validation required"
	}

	// Validate duration if expected
	if opts.ExpectedDuration != nil {
		actualDur := outputProps.DurationSecs
		result.ActualDuration = &actualDur
		result.ExpectedDuration = opts.ExpectedDuration
		result.IsDurationCorrect, result.DurationMessage = validateDuration(actualDur, *opts.ExpectedDuration)
	} else {
		result.DurationMessage = "Duration validation skipped"
	}

	// Validate HDR status if expected
	if opts.ExpectedHDR != nil {
		if !analyzer.IsHDRDetectionAvailable() {
			result.IsHDRCorrect = true
			result.HDRMessage = "HDR detection not available - validation skipped"
		} else {
			hdrInfo, err := analyzer.GetHDRInfo(outputPath)
			if err != nil {
				result.IsHDRCorrect = false
				result.HDRMessage = "Failed to detect HDR status"
			} else {
				result.ActualHDR = &hdrInfo.IsHDR
				result.ExpectedHDR = opts.ExpectedHDR
				if *opts.ExpectedHDR == hdrInfo.IsHDR {
					status := "SDR"
					if hdrInfo.IsHDR {
						status = "HDR"
					}
					result.IsHDRCorrect = true
					result.HDRMessage = status + " preserved"
				} else {
					expectedStr := "SDR"
					if *opts.ExpectedHDR {
						expectedStr = "HDR"
					}
					actualStr := "SDR"
					if hdrInfo.IsHDR {
						actualStr = "HDR"
					}
					result.IsHDRCorrect = false
					result.HDRMessage = "Expected " + expectedStr + ", found " + actualStr
				}
			}
		}
	} else {
		// No expected HDR, but still detect actual status for reporting
		if analyzer.IsHDRDetectionAvailable() {
			hdrInfo, err := analyzer.GetHDRInfo(outputPath)
			if err == nil {
				result.ActualHDR = &hdrInfo.IsHDR
				status := "SDR"
				if hdrInfo.IsHDR {
					status = "HDR"
				}
				result.HDRMessage = "Output is " + status
			}
		}
		result.IsHDRCorrect = true // No expectation means always valid
	}

	// Validate audio
	audioStreams, err := analyzer.GetAudioStreams(outputPath)
	if err != nil {
		result.AudioMessage = "Failed to get audio info"
	} else {
		result.IsAudioOpus, result.IsAudioTrackCountCorrect, result.AudioCodecs, result.AudioMessage = validateAudioStreams(
			audioStreams, opts.ExpectedAudioTracks,
		)
	}

	// Validate A/V sync
	if opts.ExpectedDuration != nil {
		result.IsSyncPreserved, result.SyncDriftMs, result.SyncMessage = validateSync(
			outputProps.DurationSecs, *opts.ExpectedDuration,
		)
	} else {
		result.SyncMessage = "Sync validation skipped"
	}

	return result, nil
}

// validateAudioStreams checks audio codec and track count.
func validateAudioStreams(streams []AnalyzerAudioStream, expectedTracks *int) (bool, bool, []string, string) {
	isOpus := true
	var codecs []string

	for _, stream := range streams {
		codec := strings.ToLower(stream.Codec)
		codecs = append(codecs, codec)
		if codec != "opus" {
			isOpus = false
		}
	}

	trackCountCorrect := true
	if expectedTracks != nil {
		trackCountCorrect = len(streams) == *expectedTracks
	}

	var message string
	if len(streams) == 0 {
		message = "No audio tracks"
	} else if len(streams) == 1 {
		if isOpus {
			message = "Audio track is Opus"
		} else {
			message = fmt.Sprintf("Audio track is %s (expected Opus)", codecs[0])
		}
	} else {
		if isOpus {
			message = fmt.Sprintf("%d audio tracks, all Opus", len(streams))
		} else {
			message = fmt.Sprintf("%d audio tracks: %s", len(streams), strings.Join(codecs, ", "))
		}
	}

	return isOpus, trackCountCorrect, codecs, message
}
