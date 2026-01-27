// Package mediainfo provides functions for HDR detection using MediaInfo.
package mediainfo

import (
	"encoding/json"
	"fmt"
	"os/exec"
	"strconv"
	"strings"
)

// VideoTrack contains video track information from MediaInfo.
type VideoTrack struct {
	Format                  string `json:"Format"`
	Width                   string `json:"Width"`
	Height                  string `json:"Height"`
	Duration                string `json:"Duration"`
	BitDepth                string `json:"BitDepth"`
	ColorSpace              string `json:"ColorSpace"`
	ChromaSubsampling       string `json:"ChromaSubsampling"`
	ColourRange             string `json:"colour_range"`
	ColourPrimaries         string `json:"colour_primaries"`
	TransferCharacteristics string `json:"transfer_characteristics"`
	MatrixCoefficients      string `json:"matrix_coefficients"`
}

// AudioTrack contains audio track information from MediaInfo.
type AudioTrack struct {
	Format       string `json:"Format"`
	Channels     string `json:"Channels"`
	SamplingRate string `json:"SamplingRate"`
	BitRate      string `json:"BitRate"`
}

// Track represents a MediaInfo track with type information.
type Track struct {
	Type  string `json:"@type"`
	Video VideoTrack
	Audio AudioTrack
}

// UnmarshalJSON implements custom JSON unmarshaling for Track.
func (t *Track) UnmarshalJSON(data []byte) error {
	// First, get the track type
	var typeOnly struct {
		Type string `json:"@type"`
	}
	if err := json.Unmarshal(data, &typeOnly); err != nil {
		return err
	}
	t.Type = typeOnly.Type

	// Then unmarshal based on type
	switch t.Type {
	case "Video":
		return json.Unmarshal(data, &t.Video)
	case "Audio":
		return json.Unmarshal(data, &t.Audio)
	}
	return nil
}

// Media contains the track array.
type Media struct {
	Track []Track `json:"track"`
}

// Response is the root MediaInfo response structure.
type Response struct {
	Media Media `json:"media"`
}

// HDRInfo contains HDR detection results.
type HDRInfo struct {
	IsHDR                   bool
	ColourPrimaries         string
	TransferCharacteristics string
	MatrixCoefficients      string
	BitDepth                *uint8
}

// IsAvailable checks if MediaInfo is available on the system.
func IsAvailable() bool {
	cmd := exec.Command("mediainfo", "--Version")
	err := cmd.Run()
	return err == nil
}

// GetMediaInfo runs MediaInfo and returns parsed output.
func GetMediaInfo(inputPath string) (*Response, error) {
	cmd := exec.Command("mediainfo", "--Output=JSON", inputPath)

	output, err := cmd.Output()
	if err != nil {
		return nil, fmt.Errorf("mediainfo failed: %w", err)
	}

	return parseMediaInfoOutput(output)
}

// parseMediaInfoOutput parses MediaInfo JSON output into the Response structure.
// This is exported for testing purposes.
func parseMediaInfoOutput(data []byte) (*Response, error) {
	var result Response
	if err := json.Unmarshal(data, &result); err != nil {
		return nil, fmt.Errorf("failed to parse mediainfo output: %w", err)
	}

	return &result, nil
}

// DetectHDR detects HDR content from MediaInfo data.
func DetectHDR(info *Response) HDRInfo {
	// Find the video track
	var videoTrack *VideoTrack
	for i := range info.Media.Track {
		if info.Media.Track[i].Type == "Video" {
			videoTrack = &info.Media.Track[i].Video
			break
		}
	}

	if videoTrack == nil {
		return HDRInfo{IsHDR: false}
	}

	// Extract values
	primaries := videoTrack.ColourPrimaries
	transfer := videoTrack.TransferCharacteristics
	matrix := videoTrack.MatrixCoefficients
	bitDepthStr := videoTrack.BitDepth

	var bitDepth *uint8
	if bitDepthStr != "" {
		if bd, err := strconv.ParseUint(bitDepthStr, 10, 8); err == nil {
			bdVal := uint8(bd)
			bitDepth = &bdVal
		}
	}

	isHDR := detectHDRFromMetadata(primaries, transfer, matrix)

	return HDRInfo{
		IsHDR:                   isHDR,
		ColourPrimaries:         primaries,
		TransferCharacteristics: transfer,
		MatrixCoefficients:      matrix,
		BitDepth:                bitDepth,
	}
}

// detectHDRFromMetadata determines if content is HDR based on color metadata.
func detectHDRFromMetadata(primaries, transfer, matrix string) bool {
	// Check for HDR primaries (BT.2020 color gamut)
	if containsAny(primaries, "BT.2020", "BT.2100") {
		return true
	}

	// Check for HDR transfer characteristics
	if containsAny(transfer, "PQ", "HLG", "SMPTE 2084") {
		return true
	}

	// Check for HDR matrix coefficients
	if containsAny(matrix, "BT.2020") {
		return true
	}

	return false
}

// containsAny checks if s contains any of the substrings.
func containsAny(s string, substrs ...string) bool {
	sLower := strings.ToLower(s)
	for _, substr := range substrs {
		if strings.Contains(sLower, strings.ToLower(substr)) {
			return true
		}
	}
	return false
}

// GetAudioChannels extracts audio channel counts from MediaInfo data.
func GetAudioChannels(info *Response) []uint32 {
	var channels []uint32
	for _, track := range info.Media.Track {
		if track.Type == "Audio" && track.Audio.Channels != "" {
			if ch, err := strconv.ParseUint(track.Audio.Channels, 10, 32); err == nil {
				channels = append(channels, uint32(ch))
			}
		}
	}
	return channels
}
