package validation

import (
	"github.com/five82/drapto/internal/ffprobe"
	"github.com/five82/drapto/internal/mediainfo"
)

// DefaultAnalyzer implements MediaAnalyzer using ffprobe and mediainfo.
type DefaultAnalyzer struct{}

// NewDefaultAnalyzer creates a new DefaultAnalyzer instance.
func NewDefaultAnalyzer() *DefaultAnalyzer {
	return &DefaultAnalyzer{}
}

// GetVideoProperties returns video stream properties using ffprobe.
func (a *DefaultAnalyzer) GetVideoProperties(path string) (*AnalyzerVideoProperties, error) {
	props, err := ffprobe.GetVideoProperties(path)
	if err != nil {
		return nil, err
	}
	return &AnalyzerVideoProperties{
		Width:        props.Width,
		Height:       props.Height,
		DurationSecs: props.DurationSecs,
		BitDepth:     props.HDRInfo.BitDepth,
	}, nil
}

// GetAudioStreams returns audio stream information using ffprobe.
func (a *DefaultAnalyzer) GetAudioStreams(path string) ([]AnalyzerAudioStream, error) {
	streams, err := ffprobe.GetAudioStreamInfo(path)
	if err != nil {
		return nil, err
	}

	result := make([]AnalyzerAudioStream, len(streams))
	for i, s := range streams {
		result[i] = AnalyzerAudioStream{
			Codec:    s.CodecName,
			Channels: int(s.Channels),
		}
	}
	return result, nil
}

// GetVideoCodec returns the video codec name using ffprobe.
func (a *DefaultAnalyzer) GetVideoCodec(path string) (string, error) {
	return ffprobe.GetVideoCodecName(path)
}

// GetHDRInfo returns HDR detection information using mediainfo.
func (a *DefaultAnalyzer) GetHDRInfo(path string) (*AnalyzerHDRInfo, error) {
	info, err := mediainfo.GetMediaInfo(path)
	if err != nil {
		return nil, err
	}

	hdr := mediainfo.DetectHDR(info)
	return &AnalyzerHDRInfo{
		IsHDR:    hdr.IsHDR,
		BitDepth: hdr.BitDepth,
	}, nil
}

// IsHDRDetectionAvailable returns whether mediainfo is available.
func (a *DefaultAnalyzer) IsHDRDetectionAvailable() bool {
	return mediainfo.IsAvailable()
}
