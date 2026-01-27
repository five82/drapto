package validation

import (
	"errors"
	"testing"
)

// mockAnalyzer implements MediaAnalyzer for testing.
type mockAnalyzer struct {
	videoProps        *AnalyzerVideoProperties
	videoPropsErr     error
	audioStreams      []AnalyzerAudioStream
	audioStreamsErr   error
	videoCodec        string
	videoCodecErr     error
	hdrInfo           *AnalyzerHDRInfo
	hdrInfoErr        error
	hdrDetectionAvail bool
}

func (m *mockAnalyzer) GetVideoProperties(path string) (*AnalyzerVideoProperties, error) {
	return m.videoProps, m.videoPropsErr
}

func (m *mockAnalyzer) GetAudioStreams(path string) ([]AnalyzerAudioStream, error) {
	return m.audioStreams, m.audioStreamsErr
}

func (m *mockAnalyzer) GetVideoCodec(path string) (string, error) {
	return m.videoCodec, m.videoCodecErr
}

func (m *mockAnalyzer) GetHDRInfo(path string) (*AnalyzerHDRInfo, error) {
	return m.hdrInfo, m.hdrInfoErr
}

func (m *mockAnalyzer) IsHDRDetectionAvailable() bool {
	return m.hdrDetectionAvail
}

func TestValidateWithAnalyzer_ValidAV1SDR(t *testing.T) {
	bitDepth := uint8(10)
	mock := &mockAnalyzer{
		videoProps: &AnalyzerVideoProperties{
			Width:        1920,
			Height:       800,
			DurationSecs: 120.5,
			BitDepth:     &bitDepth,
		},
		audioStreams: []AnalyzerAudioStream{
			{Codec: "opus", Channels: 2},
		},
		videoCodec:        "av1",
		hdrInfo:           &AnalyzerHDRInfo{IsHDR: false, BitDepth: &bitDepth},
		hdrDetectionAvail: true,
	}

	expectedDims := [2]uint32{1920, 800}
	expectedDuration := 120.5
	expectedHDR := false
	expectedTracks := 1

	result, err := ValidateWithAnalyzer(mock, "/fake/path.mp4", Options{
		ExpectedDimensions:  &expectedDims,
		ExpectedDuration:    &expectedDuration,
		ExpectedHDR:         &expectedHDR,
		ExpectedAudioTracks: &expectedTracks,
	})

	if err != nil {
		t.Fatalf("ValidateWithAnalyzer() error = %v", err)
	}

	if !result.IsValid() {
		t.Errorf("IsValid() = false, want true. Failures: %v", result.GetFailures())
	}

	if !result.IsAV1 {
		t.Error("IsAV1 = false, want true")
	}
	if !result.Is10Bit {
		t.Error("Is10Bit = false, want true")
	}
	if !result.IsCropCorrect {
		t.Error("IsCropCorrect = false, want true")
	}
	if !result.IsDurationCorrect {
		t.Error("IsDurationCorrect = false, want true")
	}
	if !result.IsHDRCorrect {
		t.Error("IsHDRCorrect = false, want true")
	}
	if !result.IsAudioOpus {
		t.Error("IsAudioOpus = false, want true")
	}
}

func TestValidateWithAnalyzer_ValidAV1HDR(t *testing.T) {
	bitDepth := uint8(10)
	mock := &mockAnalyzer{
		videoProps: &AnalyzerVideoProperties{
			Width:        3840,
			Height:       2160,
			DurationSecs: 7200.0,
			BitDepth:     &bitDepth,
		},
		audioStreams: []AnalyzerAudioStream{
			{Codec: "opus", Channels: 8},
			{Codec: "opus", Channels: 6},
		},
		videoCodec:        "av1",
		hdrInfo:           &AnalyzerHDRInfo{IsHDR: true, BitDepth: &bitDepth},
		hdrDetectionAvail: true,
	}

	expectedDims := [2]uint32{3840, 2160}
	expectedDuration := 7200.0
	expectedHDR := true
	expectedTracks := 2

	result, err := ValidateWithAnalyzer(mock, "/fake/path.mp4", Options{
		ExpectedDimensions:  &expectedDims,
		ExpectedDuration:    &expectedDuration,
		ExpectedHDR:         &expectedHDR,
		ExpectedAudioTracks: &expectedTracks,
	})

	if err != nil {
		t.Fatalf("ValidateWithAnalyzer() error = %v", err)
	}

	if !result.IsValid() {
		t.Errorf("IsValid() = false, want true. Failures: %v", result.GetFailures())
	}

	if result.HDRMessage != "HDR preserved" {
		t.Errorf("HDRMessage = %q, want %q", result.HDRMessage, "HDR preserved")
	}
}

func TestValidateWithAnalyzer_DimensionMismatch(t *testing.T) {
	bitDepth := uint8(10)
	mock := &mockAnalyzer{
		videoProps: &AnalyzerVideoProperties{
			Width:        1920,
			Height:       1080, // Not cropped
			DurationSecs: 120.5,
			BitDepth:     &bitDepth,
		},
		audioStreams:      []AnalyzerAudioStream{{Codec: "opus", Channels: 2}},
		videoCodec:        "av1",
		hdrInfo:           &AnalyzerHDRInfo{IsHDR: false, BitDepth: &bitDepth},
		hdrDetectionAvail: true,
	}

	expectedDims := [2]uint32{1920, 800} // Expected cropped height

	result, err := ValidateWithAnalyzer(mock, "/fake/path.mp4", Options{
		ExpectedDimensions: &expectedDims,
	})

	if err != nil {
		t.Fatalf("ValidateWithAnalyzer() error = %v", err)
	}

	if result.IsCropCorrect {
		t.Error("IsCropCorrect = true, want false for dimension mismatch")
	}
}

func TestValidateWithAnalyzer_WrongCodec(t *testing.T) {
	bitDepth := uint8(10)
	mock := &mockAnalyzer{
		videoProps: &AnalyzerVideoProperties{
			Width:        1920,
			Height:       1080,
			DurationSecs: 120.5,
			BitDepth:     &bitDepth,
		},
		audioStreams:      []AnalyzerAudioStream{{Codec: "opus", Channels: 2}},
		videoCodec:        "hevc", // Not AV1
		hdrInfo:           &AnalyzerHDRInfo{IsHDR: false, BitDepth: &bitDepth},
		hdrDetectionAvail: true,
	}

	result, err := ValidateWithAnalyzer(mock, "/fake/path.mp4", Options{})

	if err != nil {
		t.Fatalf("ValidateWithAnalyzer() error = %v", err)
	}

	if result.IsAV1 {
		t.Error("IsAV1 = true, want false for HEVC codec")
	}
	if result.CodecName != "hevc" {
		t.Errorf("CodecName = %q, want %q", result.CodecName, "hevc")
	}
}

func TestValidateWithAnalyzer_NonOpusAudio(t *testing.T) {
	bitDepth := uint8(10)
	mock := &mockAnalyzer{
		videoProps: &AnalyzerVideoProperties{
			Width:        1920,
			Height:       1080,
			DurationSecs: 120.5,
			BitDepth:     &bitDepth,
		},
		audioStreams: []AnalyzerAudioStream{
			{Codec: "aac", Channels: 2}, // Not Opus
		},
		videoCodec:        "av1",
		hdrInfo:           &AnalyzerHDRInfo{IsHDR: false, BitDepth: &bitDepth},
		hdrDetectionAvail: true,
	}

	result, err := ValidateWithAnalyzer(mock, "/fake/path.mp4", Options{})

	if err != nil {
		t.Fatalf("ValidateWithAnalyzer() error = %v", err)
	}

	if result.IsAudioOpus {
		t.Error("IsAudioOpus = true, want false for AAC audio")
	}
}

func TestValidateWithAnalyzer_HDRMismatch(t *testing.T) {
	bitDepth := uint8(10)
	mock := &mockAnalyzer{
		videoProps: &AnalyzerVideoProperties{
			Width:        3840,
			Height:       2160,
			DurationSecs: 7200.0,
			BitDepth:     &bitDepth,
		},
		audioStreams:      []AnalyzerAudioStream{{Codec: "opus", Channels: 6}},
		videoCodec:        "av1",
		hdrInfo:           &AnalyzerHDRInfo{IsHDR: false, BitDepth: &bitDepth}, // Actually SDR
		hdrDetectionAvail: true,
	}

	expectedHDR := true // Expected HDR

	result, err := ValidateWithAnalyzer(mock, "/fake/path.mp4", Options{
		ExpectedHDR: &expectedHDR,
	})

	if err != nil {
		t.Fatalf("ValidateWithAnalyzer() error = %v", err)
	}

	if result.IsHDRCorrect {
		t.Error("IsHDRCorrect = true, want false for HDR mismatch")
	}
	if result.HDRMessage != "Expected HDR, found SDR" {
		t.Errorf("HDRMessage = %q, want %q", result.HDRMessage, "Expected HDR, found SDR")
	}
}

func TestValidateWithAnalyzer_HDRDetectionUnavailable(t *testing.T) {
	bitDepth := uint8(10)
	mock := &mockAnalyzer{
		videoProps: &AnalyzerVideoProperties{
			Width:        1920,
			Height:       1080,
			DurationSecs: 120.5,
			BitDepth:     &bitDepth,
		},
		audioStreams:      []AnalyzerAudioStream{{Codec: "opus", Channels: 2}},
		videoCodec:        "av1",
		hdrDetectionAvail: false, // HDR detection not available
	}

	expectedHDR := true

	result, err := ValidateWithAnalyzer(mock, "/fake/path.mp4", Options{
		ExpectedHDR: &expectedHDR,
	})

	if err != nil {
		t.Fatalf("ValidateWithAnalyzer() error = %v", err)
	}

	// Should still pass when HDR detection is unavailable
	if !result.IsHDRCorrect {
		t.Error("IsHDRCorrect = false, want true when HDR detection unavailable")
	}
	if result.HDRMessage != "HDR detection not available - validation skipped" {
		t.Errorf("HDRMessage = %q, want %q", result.HDRMessage, "HDR detection not available - validation skipped")
	}
}

func TestValidateWithAnalyzer_VideoPropsError(t *testing.T) {
	mock := &mockAnalyzer{
		videoPropsErr: errors.New("ffprobe failed"),
	}

	_, err := ValidateWithAnalyzer(mock, "/fake/path.mp4", Options{})

	if err == nil {
		t.Error("ValidateWithAnalyzer() expected error, got nil")
	}
}

func TestValidateWithAnalyzer_DurationTolerance(t *testing.T) {
	bitDepth := uint8(10)
	mock := &mockAnalyzer{
		videoProps: &AnalyzerVideoProperties{
			Width:        1920,
			Height:       1080,
			DurationSecs: 120.8, // 0.3s difference (within 1s tolerance)
			BitDepth:     &bitDepth,
		},
		audioStreams:      []AnalyzerAudioStream{{Codec: "opus", Channels: 2}},
		videoCodec:        "av1",
		hdrInfo:           &AnalyzerHDRInfo{IsHDR: false, BitDepth: &bitDepth},
		hdrDetectionAvail: true,
	}

	expectedDuration := 120.5

	result, err := ValidateWithAnalyzer(mock, "/fake/path.mp4", Options{
		ExpectedDuration: &expectedDuration,
	})

	if err != nil {
		t.Fatalf("ValidateWithAnalyzer() error = %v", err)
	}

	if !result.IsDurationCorrect {
		t.Error("IsDurationCorrect = false, want true for small duration difference")
	}
}

func TestValidateWithAnalyzer_DurationExceedsTolerance(t *testing.T) {
	bitDepth := uint8(10)
	mock := &mockAnalyzer{
		videoProps: &AnalyzerVideoProperties{
			Width:        1920,
			Height:       1080,
			DurationSecs: 122.0, // 1.5s difference (exceeds 1s tolerance)
			BitDepth:     &bitDepth,
		},
		audioStreams:      []AnalyzerAudioStream{{Codec: "opus", Channels: 2}},
		videoCodec:        "av1",
		hdrInfo:           &AnalyzerHDRInfo{IsHDR: false, BitDepth: &bitDepth},
		hdrDetectionAvail: true,
	}

	expectedDuration := 120.5

	result, err := ValidateWithAnalyzer(mock, "/fake/path.mp4", Options{
		ExpectedDuration: &expectedDuration,
	})

	if err != nil {
		t.Fatalf("ValidateWithAnalyzer() error = %v", err)
	}

	if result.IsDurationCorrect {
		t.Error("IsDurationCorrect = true, want false for large duration difference")
	}
}

func TestValidateWithAnalyzer_AudioTrackCountMismatch(t *testing.T) {
	bitDepth := uint8(10)
	mock := &mockAnalyzer{
		videoProps: &AnalyzerVideoProperties{
			Width:        1920,
			Height:       1080,
			DurationSecs: 120.5,
			BitDepth:     &bitDepth,
		},
		audioStreams: []AnalyzerAudioStream{
			{Codec: "opus", Channels: 2},
		},
		videoCodec:        "av1",
		hdrInfo:           &AnalyzerHDRInfo{IsHDR: false, BitDepth: &bitDepth},
		hdrDetectionAvail: true,
	}

	expectedTracks := 2 // Expected 2 tracks but got 1

	result, err := ValidateWithAnalyzer(mock, "/fake/path.mp4", Options{
		ExpectedAudioTracks: &expectedTracks,
	})

	if err != nil {
		t.Fatalf("ValidateWithAnalyzer() error = %v", err)
	}

	if result.IsAudioTrackCountCorrect {
		t.Error("IsAudioTrackCountCorrect = true, want false for track count mismatch")
	}
}

func TestValidateWithAnalyzer_NoOptions(t *testing.T) {
	bitDepth := uint8(10)
	mock := &mockAnalyzer{
		videoProps: &AnalyzerVideoProperties{
			Width:        1920,
			Height:       1080,
			DurationSecs: 120.5,
			BitDepth:     &bitDepth,
		},
		audioStreams:      []AnalyzerAudioStream{{Codec: "opus", Channels: 2}},
		videoCodec:        "av1",
		hdrInfo:           &AnalyzerHDRInfo{IsHDR: false, BitDepth: &bitDepth},
		hdrDetectionAvail: true,
	}

	result, err := ValidateWithAnalyzer(mock, "/fake/path.mp4", Options{})

	if err != nil {
		t.Fatalf("ValidateWithAnalyzer() error = %v", err)
	}

	// With no expectations, all dimension/duration/HDR checks should pass
	if !result.IsCropCorrect {
		t.Error("IsCropCorrect = false, want true when no dimensions expected")
	}
	if !result.IsDurationCorrect {
		t.Error("IsDurationCorrect = false, want true when no duration expected")
	}
	if !result.IsHDRCorrect {
		t.Error("IsHDRCorrect = false, want true when no HDR expected")
	}
}
