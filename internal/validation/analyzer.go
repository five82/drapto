// Package validation provides post-encode validation checks.
package validation

// MediaAnalyzer provides media analysis capabilities for validation.
// This interface allows validation logic to be tested without external tools.
type MediaAnalyzer interface {
	// GetVideoProperties returns video stream properties for the given file.
	GetVideoProperties(path string) (*AnalyzerVideoProperties, error)

	// GetAudioStreams returns audio stream information for the given file.
	GetAudioStreams(path string) ([]AnalyzerAudioStream, error)

	// GetVideoCodec returns the video codec name for the given file.
	GetVideoCodec(path string) (string, error)

	// GetHDRInfo returns HDR detection information for the given file.
	GetHDRInfo(path string) (*AnalyzerHDRInfo, error)

	// IsHDRDetectionAvailable returns whether HDR detection is available.
	IsHDRDetectionAvailable() bool
}

// AnalyzerVideoProperties contains video stream information needed for validation.
type AnalyzerVideoProperties struct {
	Width        uint32
	Height       uint32
	DurationSecs float64
	BitDepth     *uint8
}

// AnalyzerAudioStream contains audio stream information.
type AnalyzerAudioStream struct {
	Codec    string
	Channels int
}

// AnalyzerHDRInfo contains HDR detection results.
type AnalyzerHDRInfo struct {
	IsHDR    bool
	BitDepth *uint8
}
