package processing

import (
	"fmt"
	"log"
	"path/filepath"
	"strings"

	"github.com/five82/drapto/internal/ffmpeg"
	"github.com/five82/drapto/internal/ffprobe"
)

// GetAudioChannels returns audio channel counts for a file.
func GetAudioChannels(inputPath string) []uint32 {
	channels, err := ffprobe.GetAudioChannels(inputPath)
	if err != nil {
		return nil
	}
	return channels
}

// GetAudioStreamInfo returns detailed audio stream information.
func GetAudioStreamInfo(inputPath string) []ffprobe.AudioStreamInfo {
	streams, err := ffprobe.GetAudioStreamInfo(inputPath)
	if err != nil {
		return nil
	}
	return streams
}

// FormatAudioDescription formats a basic audio description.
func FormatAudioDescription(channels []uint32) string {
	if len(channels) == 0 {
		return "No audio"
	}

	if len(channels) == 1 {
		return fmt.Sprintf("%d channels", channels[0])
	}

	var parts []string
	for i, ch := range channels {
		parts = append(parts, fmt.Sprintf("Stream %d (%dch)", i, ch))
	}
	return fmt.Sprintf("%d streams: %s", len(channels), strings.Join(parts, ", "))
}

// FormatAudioDescriptionConfig formats audio description for config display.
func FormatAudioDescriptionConfig(channels []uint32, streams []ffprobe.AudioStreamInfo) string {
	if streams == nil {
		return FormatAudioDescription(channels)
	}

	if len(streams) == 0 {
		return "No audio"
	}

	if len(streams) == 1 {
		stream := streams[0]
		bitrate := ffmpeg.CalculateAudioBitrate(stream.Channels)
		return fmt.Sprintf("%d channels @ %dkbps Opus", stream.Channels, bitrate)
	}

	var parts []string
	for _, stream := range streams {
		bitrate := ffmpeg.CalculateAudioBitrate(stream.Channels)
		parts = append(parts, fmt.Sprintf("Stream %d: %dch [%dkbps Opus]", stream.Index, stream.Channels, bitrate))
	}
	return strings.Join(parts, ", ")
}

// GenerateAudioResultsDescription generates audio description for results.
func GenerateAudioResultsDescription(channels []uint32, streams []ffprobe.AudioStreamInfo) string {
	if len(streams) > 0 {
		if len(streams) == 1 {
			bitrate := ffmpeg.CalculateAudioBitrate(streams[0].Channels)
			return fmt.Sprintf("Opus %dch @ %dkbps", streams[0].Channels, bitrate)
		}

		var parts []string
		for _, stream := range streams {
			bitrate := ffmpeg.CalculateAudioBitrate(stream.Channels)
			parts = append(parts, fmt.Sprintf("%dch@%dk", stream.Channels, bitrate))
		}
		return fmt.Sprintf("Opus (%s)", strings.Join(parts, ", "))
	}

	if len(channels) == 0 {
		return "No audio"
	}

	if len(channels) == 1 {
		bitrate := ffmpeg.CalculateAudioBitrate(channels[0])
		return fmt.Sprintf("Opus %dch @ %dkbps", channels[0], bitrate)
	}

	var parts []string
	for _, ch := range channels {
		bitrate := ffmpeg.CalculateAudioBitrate(ch)
		parts = append(parts, fmt.Sprintf("%dch@%dk", ch, bitrate))
	}
	return fmt.Sprintf("Opus (%s)", strings.Join(parts, ", "))
}

// Logger defines the interface for audio analysis logging.
type Logger interface {
	Info(format string, args ...any)
	Warn(format string, args ...any)
}

// DefaultLogger implements Logger using the standard log package.
type DefaultLogger struct{}

func (d DefaultLogger) Info(format string, args ...any) {
	log.Printf("[INFO] "+format, args...)
}

func (d DefaultLogger) Warn(format string, args ...any) {
	log.Printf("[WARN] "+format, args...)
}

// AnalyzeAndLogAudio analyzes audio streams and logs channel information.
// Returns channel counts for encoding. Returns empty slice on error (non-critical operation).
func AnalyzeAndLogAudio(inputPath string, logger Logger) []uint32 {
	if logger == nil {
		logger = DefaultLogger{}
	}

	filename := filepath.Base(inputPath)

	audioChannels, err := ffprobe.GetAudioChannels(inputPath)
	if err != nil {
		logger.Warn("Error getting audio channels for %s: %v. Using empty list.", filename, err)
		logger.Info("Audio streams: Error detecting audio")
		return nil
	}

	if len(audioChannels) == 0 {
		logger.Info("Audio streams: None detected")
		return nil
	}

	// Log channel summary
	var channelSummary string
	if len(audioChannels) == 1 {
		channelSummary = fmt.Sprintf("%d channels", audioChannels[0])
	} else {
		var parts []string
		for i, ch := range audioChannels {
			parts = append(parts, fmt.Sprintf("Stream %d (%dch)", i, ch))
		}
		channelSummary = fmt.Sprintf("%d streams: %s", len(audioChannels), strings.Join(parts, ", "))
	}
	logger.Info("Audio: %s", channelSummary)

	// Log bitrate information
	var bitrateParts []string
	for i, numChannels := range audioChannels {
		bitrate := ffmpeg.CalculateAudioBitrate(numChannels)
		if len(audioChannels) == 1 {
			logger.Info("Bitrate: %dkbps", bitrate)
		} else {
			bitrateParts = append(bitrateParts, fmt.Sprintf("Stream %d: %dkbps", i, bitrate))
		}
	}

	if len(audioChannels) > 1 {
		logger.Info("Bitrates: %s", strings.Join(bitrateParts, ", "))
	}

	return audioChannels
}

// AnalyzeAndLogAudioDetailed analyzes audio streams and returns detailed stream information.
// Also logs audio stream details. Returns nil on error (non-critical operation).
func AnalyzeAndLogAudioDetailed(inputPath string, logger Logger) []ffprobe.AudioStreamInfo {
	if logger == nil {
		logger = DefaultLogger{}
	}

	filename := filepath.Base(inputPath)

	audioStreams, err := ffprobe.GetAudioStreamInfo(inputPath)
	if err != nil {
		logger.Warn("Error getting audio stream info for %s: %v. Using fallback.", filename, err)
		logger.Info("Audio streams: Error detecting audio details")
		return nil
	}

	logger.Info("Detected %d audio streams", len(audioStreams))
	for _, stream := range audioStreams {
		logger.Info("Stream %d: codec=%s, profile=%s, spatial=%v",
			stream.Index, stream.CodecName, stream.Profile, stream.IsSpatial)
	}

	if len(audioStreams) == 0 {
		logger.Info("Audio streams: None detected")
		return audioStreams
	}

	// Log stream information (all streams will be transcoded to Opus)
	if len(audioStreams) == 1 {
		stream := audioStreams[0]
		bitrate := ffmpeg.CalculateAudioBitrate(stream.Channels)
		logger.Info("Audio: %d channels @ %dkbps Opus", stream.Channels, bitrate)
	} else {
		logger.Info("Audio: %d streams detected", len(audioStreams))
		for _, stream := range audioStreams {
			bitrate := ffmpeg.CalculateAudioBitrate(stream.Channels)
			logger.Info("  Stream %d: %d channels [%dkbps Opus]", stream.Index, stream.Channels, bitrate)
		}
	}

	return audioStreams
}

// GetAudioChannelsQuiet analyzes audio streams and returns channel information without logging.
// Returns empty slice on error (non-critical operation).
func GetAudioChannelsQuiet(inputPath string) []uint32 {
	channels, err := ffprobe.GetAudioChannels(inputPath)
	if err != nil {
		return nil
	}
	return channels
}
