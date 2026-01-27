package ffprobe

import (
	"os"
	"path/filepath"
	"testing"
)

// loadTestData loads a JSON fixture from the testdata directory.
func loadTestData(t *testing.T, filename string) []byte {
	t.Helper()
	data, err := os.ReadFile(filepath.Join("testdata", filename))
	if err != nil {
		t.Fatalf("failed to load test data %s: %v", filename, err)
	}
	return data
}

func TestParseFFprobeOutput_Valid1080pSDR(t *testing.T) {
	data := loadTestData(t, "video_1080p_sdr.json")

	probe, err := parseFFprobeOutput(data)
	if err != nil {
		t.Fatalf("parseFFprobeOutput() error = %v", err)
	}

	if probe.Format.Duration != "120.500000" {
		t.Errorf("Duration = %q, want %q", probe.Format.Duration, "120.500000")
	}

	if len(probe.Streams) != 2 {
		t.Fatalf("len(Streams) = %d, want 2", len(probe.Streams))
	}

	// Check video stream
	video := probe.Streams[0]
	if video.CodecType != "video" {
		t.Errorf("video.CodecType = %q, want %q", video.CodecType, "video")
	}
	if video.Width != 1920 {
		t.Errorf("video.Width = %d, want 1920", video.Width)
	}
	if video.Height != 1080 {
		t.Errorf("video.Height = %d, want 1080", video.Height)
	}
	if video.BitsPerRawSample != "8" {
		t.Errorf("video.BitsPerRawSample = %q, want %q", video.BitsPerRawSample, "8")
	}

	// Check audio stream
	audio := probe.Streams[1]
	if audio.CodecType != "audio" {
		t.Errorf("audio.CodecType = %q, want %q", audio.CodecType, "audio")
	}
	if audio.Channels != 2 {
		t.Errorf("audio.Channels = %d, want 2", audio.Channels)
	}
}

func TestParseFFprobeOutput_Valid4KHDRPQ(t *testing.T) {
	data := loadTestData(t, "video_4k_hdr_pq.json")

	probe, err := parseFFprobeOutput(data)
	if err != nil {
		t.Fatalf("parseFFprobeOutput() error = %v", err)
	}

	// Check video stream
	video := probe.Streams[0]
	if video.Width != 3840 {
		t.Errorf("video.Width = %d, want 3840", video.Width)
	}
	if video.Height != 2160 {
		t.Errorf("video.Height = %d, want 2160", video.Height)
	}
	if video.ColorPrimaries != "bt2020" {
		t.Errorf("video.ColorPrimaries = %q, want %q", video.ColorPrimaries, "bt2020")
	}
	if video.ColorTransfer != "smpte2084" {
		t.Errorf("video.ColorTransfer = %q, want %q", video.ColorTransfer, "smpte2084")
	}

	// Check multiple audio streams
	if len(probe.Streams) != 3 {
		t.Fatalf("len(Streams) = %d, want 3", len(probe.Streams))
	}
}

func TestParseFFprobeOutput_MalformedJSON(t *testing.T) {
	data := []byte(`{"format": {"duration": "120.5"}, "streams": [}`)

	_, err := parseFFprobeOutput(data)
	if err == nil {
		t.Error("parseFFprobeOutput() expected error for malformed JSON, got nil")
	}
}

func TestExtractVideoProperties_SDR(t *testing.T) {
	data := loadTestData(t, "video_1080p_sdr.json")
	probe, err := parseFFprobeOutput(data)
	if err != nil {
		t.Fatalf("parseFFprobeOutput() error = %v", err)
	}

	props, err := extractVideoProperties(probe, "test.mp4")
	if err != nil {
		t.Fatalf("extractVideoProperties() error = %v", err)
	}

	if props.Width != 1920 {
		t.Errorf("Width = %d, want 1920", props.Width)
	}
	if props.Height != 1080 {
		t.Errorf("Height = %d, want 1080", props.Height)
	}
	if props.DurationSecs != 120.5 {
		t.Errorf("DurationSecs = %f, want 120.5", props.DurationSecs)
	}
	if props.HDRInfo.IsHDR {
		t.Error("HDRInfo.IsHDR = true, want false for SDR content")
	}
	if props.HDRInfo.BitDepth == nil || *props.HDRInfo.BitDepth != 8 {
		t.Errorf("HDRInfo.BitDepth = %v, want 8", props.HDRInfo.BitDepth)
	}
}

func TestExtractVideoProperties_HDRPQ(t *testing.T) {
	data := loadTestData(t, "video_4k_hdr_pq.json")
	probe, err := parseFFprobeOutput(data)
	if err != nil {
		t.Fatalf("parseFFprobeOutput() error = %v", err)
	}

	props, err := extractVideoProperties(probe, "test.mp4")
	if err != nil {
		t.Fatalf("extractVideoProperties() error = %v", err)
	}

	if props.Width != 3840 {
		t.Errorf("Width = %d, want 3840", props.Width)
	}
	if props.Height != 2160 {
		t.Errorf("Height = %d, want 2160", props.Height)
	}
	if !props.HDRInfo.IsHDR {
		t.Error("HDRInfo.IsHDR = false, want true for HDR PQ content")
	}
	if props.HDRInfo.ColourPrimaries != "bt2020" {
		t.Errorf("HDRInfo.ColourPrimaries = %q, want %q", props.HDRInfo.ColourPrimaries, "bt2020")
	}
	if props.HDRInfo.TransferCharacteristics != "smpte2084" {
		t.Errorf("HDRInfo.TransferCharacteristics = %q, want %q", props.HDRInfo.TransferCharacteristics, "smpte2084")
	}
}

func TestExtractVideoProperties_HDRHLG(t *testing.T) {
	data := loadTestData(t, "video_4k_hdr_hlg.json")
	probe, err := parseFFprobeOutput(data)
	if err != nil {
		t.Fatalf("parseFFprobeOutput() error = %v", err)
	}

	props, err := extractVideoProperties(probe, "test.mp4")
	if err != nil {
		t.Fatalf("extractVideoProperties() error = %v", err)
	}

	if !props.HDRInfo.IsHDR {
		t.Error("HDRInfo.IsHDR = false, want true for HDR HLG content")
	}
	if props.HDRInfo.TransferCharacteristics != "arib-std-b67" {
		t.Errorf("HDRInfo.TransferCharacteristics = %q, want %q", props.HDRInfo.TransferCharacteristics, "arib-std-b67")
	}
}

func TestExtractVideoProperties_NoVideoStream(t *testing.T) {
	data := loadTestData(t, "video_no_video_stream.json")
	probe, err := parseFFprobeOutput(data)
	if err != nil {
		t.Fatalf("parseFFprobeOutput() error = %v", err)
	}

	_, err = extractVideoProperties(probe, "test.mp4")
	if err == nil {
		t.Error("extractVideoProperties() expected error for missing video stream, got nil")
	}
}

func TestExtractAudioChannels(t *testing.T) {
	data := loadTestData(t, "video_4k_hdr_pq.json")
	probe, err := parseFFprobeOutput(data)
	if err != nil {
		t.Fatalf("parseFFprobeOutput() error = %v", err)
	}

	channels := extractAudioChannels(probe)
	if len(channels) != 2 {
		t.Fatalf("len(channels) = %d, want 2", len(channels))
	}
	if channels[0] != 8 {
		t.Errorf("channels[0] = %d, want 8", channels[0])
	}
	if channels[1] != 6 {
		t.Errorf("channels[1] = %d, want 6", channels[1])
	}
}

func TestExtractAudioStreamInfo(t *testing.T) {
	data := loadTestData(t, "video_4k_hdr_pq.json")
	probe, err := parseFFprobeOutput(data)
	if err != nil {
		t.Fatalf("parseFFprobeOutput() error = %v", err)
	}

	streams := extractAudioStreamInfo(probe)
	if len(streams) != 2 {
		t.Fatalf("len(streams) = %d, want 2", len(streams))
	}

	// Check first audio stream
	if streams[0].CodecName != "truehd" {
		t.Errorf("streams[0].CodecName = %q, want %q", streams[0].CodecName, "truehd")
	}
	if streams[0].Channels != 8 {
		t.Errorf("streams[0].Channels = %d, want 8", streams[0].Channels)
	}
	if streams[0].Index != 0 {
		t.Errorf("streams[0].Index = %d, want 0", streams[0].Index)
	}
	if streams[0].Disposition.Default != 1 {
		t.Errorf("streams[0].Disposition.Default = %d, want 1", streams[0].Disposition.Default)
	}
	if streams[0].Disposition.Original != 1 {
		t.Errorf("streams[0].Disposition.Original = %d, want 1", streams[0].Disposition.Original)
	}

	// Check second audio stream
	if streams[1].CodecName != "ac3" {
		t.Errorf("streams[1].CodecName = %q, want %q", streams[1].CodecName, "ac3")
	}
	if streams[1].Channels != 6 {
		t.Errorf("streams[1].Channels = %d, want 6", streams[1].Channels)
	}
	if streams[1].Index != 1 {
		t.Errorf("streams[1].Index = %d, want 1", streams[1].Index)
	}
	if streams[1].Disposition.Dub != 1 {
		t.Errorf("streams[1].Disposition.Dub = %d, want 1", streams[1].Disposition.Dub)
	}
}

func TestExtractMediaInfo(t *testing.T) {
	data := loadTestData(t, "video_1080p_sdr.json")
	probe, err := parseFFprobeOutput(data)
	if err != nil {
		t.Fatalf("parseFFprobeOutput() error = %v", err)
	}

	info := extractMediaInfo(probe)
	if info.Duration != 120.5 {
		t.Errorf("Duration = %f, want 120.5", info.Duration)
	}
	if info.Width != 1920 {
		t.Errorf("Width = %d, want 1920", info.Width)
	}
	if info.Height != 1080 {
		t.Errorf("Height = %d, want 1080", info.Height)
	}
	if info.TotalFrames != 2892 {
		t.Errorf("TotalFrames = %d, want 2892", info.TotalFrames)
	}
}

func TestDetectHDR(t *testing.T) {
	tests := []struct {
		name     string
		primaries string
		transfer  string
		matrix    string
		wantHDR  bool
	}{
		{
			name:     "SDR BT709",
			primaries: "bt709",
			transfer:  "bt709",
			matrix:    "bt709",
			wantHDR:  false,
		},
		{
			name:     "HDR PQ with BT2020",
			primaries: "bt2020",
			transfer:  "smpte2084",
			matrix:    "bt2020nc",
			wantHDR:  true,
		},
		{
			name:     "HDR HLG",
			primaries: "bt2020",
			transfer:  "arib-std-b67",
			matrix:    "bt2020nc",
			wantHDR:  true,
		},
		{
			name:     "BT2020 primaries only",
			primaries: "bt2020",
			transfer:  "bt709",
			matrix:    "bt709",
			wantHDR:  true,
		},
		{
			name:     "PQ transfer only",
			primaries: "bt709",
			transfer:  "smpte2084",
			matrix:    "bt709",
			wantHDR:  true,
		},
		{
			name:     "Empty values",
			primaries: "",
			transfer:  "",
			matrix:    "",
			wantHDR:  false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := detectHDR(tt.primaries, tt.transfer, tt.matrix)
			if got != tt.wantHDR {
				t.Errorf("detectHDR(%q, %q, %q) = %v, want %v",
					tt.primaries, tt.transfer, tt.matrix, got, tt.wantHDR)
			}
		})
	}
}
