package mediainfo

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

func TestParseMediaInfoOutput_ValidSDR(t *testing.T) {
	data := loadTestData(t, "video_sdr.json")

	resp, err := parseMediaInfoOutput(data)
	if err != nil {
		t.Fatalf("parseMediaInfoOutput() error = %v", err)
	}

	if len(resp.Media.Track) != 3 {
		t.Fatalf("len(Track) = %d, want 3", len(resp.Media.Track))
	}

	// Check video track
	var videoTrack *Track
	for i := range resp.Media.Track {
		if resp.Media.Track[i].Type == "Video" {
			videoTrack = &resp.Media.Track[i]
			break
		}
	}

	if videoTrack == nil {
		t.Fatal("no video track found")
	}

	if videoTrack.Video.Format != "AVC" {
		t.Errorf("Video.Format = %q, want %q", videoTrack.Video.Format, "AVC")
	}
	if videoTrack.Video.Width != "1920" {
		t.Errorf("Video.Width = %q, want %q", videoTrack.Video.Width, "1920")
	}
	if videoTrack.Video.Height != "1080" {
		t.Errorf("Video.Height = %q, want %q", videoTrack.Video.Height, "1080")
	}
	if videoTrack.Video.BitDepth != "8" {
		t.Errorf("Video.BitDepth = %q, want %q", videoTrack.Video.BitDepth, "8")
	}
}

func TestParseMediaInfoOutput_ValidHDRPQ(t *testing.T) {
	data := loadTestData(t, "video_hdr_pq.json")

	resp, err := parseMediaInfoOutput(data)
	if err != nil {
		t.Fatalf("parseMediaInfoOutput() error = %v", err)
	}

	// Check video track
	var videoTrack *Track
	for i := range resp.Media.Track {
		if resp.Media.Track[i].Type == "Video" {
			videoTrack = &resp.Media.Track[i]
			break
		}
	}

	if videoTrack == nil {
		t.Fatal("no video track found")
	}

	if videoTrack.Video.ColourPrimaries != "BT.2020" {
		t.Errorf("Video.ColourPrimaries = %q, want %q", videoTrack.Video.ColourPrimaries, "BT.2020")
	}
	if videoTrack.Video.TransferCharacteristics != "PQ" {
		t.Errorf("Video.TransferCharacteristics = %q, want %q", videoTrack.Video.TransferCharacteristics, "PQ")
	}
	if videoTrack.Video.BitDepth != "10" {
		t.Errorf("Video.BitDepth = %q, want %q", videoTrack.Video.BitDepth, "10")
	}
}

func TestParseMediaInfoOutput_MalformedJSON(t *testing.T) {
	data := []byte(`{"media": {"track": [}`)

	_, err := parseMediaInfoOutput(data)
	if err == nil {
		t.Error("parseMediaInfoOutput() expected error for malformed JSON, got nil")
	}
}

func TestDetectHDR_SDR(t *testing.T) {
	data := loadTestData(t, "video_sdr.json")
	resp, err := parseMediaInfoOutput(data)
	if err != nil {
		t.Fatalf("parseMediaInfoOutput() error = %v", err)
	}

	hdr := DetectHDR(resp)
	if hdr.IsHDR {
		t.Error("IsHDR = true, want false for SDR content")
	}
	if hdr.BitDepth == nil || *hdr.BitDepth != 8 {
		t.Errorf("BitDepth = %v, want 8", hdr.BitDepth)
	}
	if hdr.ColourPrimaries != "BT.709" {
		t.Errorf("ColourPrimaries = %q, want %q", hdr.ColourPrimaries, "BT.709")
	}
}

func TestDetectHDR_HDRPQ(t *testing.T) {
	data := loadTestData(t, "video_hdr_pq.json")
	resp, err := parseMediaInfoOutput(data)
	if err != nil {
		t.Fatalf("parseMediaInfoOutput() error = %v", err)
	}

	hdr := DetectHDR(resp)
	if !hdr.IsHDR {
		t.Error("IsHDR = false, want true for HDR PQ content")
	}
	if hdr.BitDepth == nil || *hdr.BitDepth != 10 {
		t.Errorf("BitDepth = %v, want 10", hdr.BitDepth)
	}
	if hdr.ColourPrimaries != "BT.2020" {
		t.Errorf("ColourPrimaries = %q, want %q", hdr.ColourPrimaries, "BT.2020")
	}
	if hdr.TransferCharacteristics != "PQ" {
		t.Errorf("TransferCharacteristics = %q, want %q", hdr.TransferCharacteristics, "PQ")
	}
}

func TestDetectHDR_HDRHLG(t *testing.T) {
	data := loadTestData(t, "video_hdr_hlg.json")
	resp, err := parseMediaInfoOutput(data)
	if err != nil {
		t.Fatalf("parseMediaInfoOutput() error = %v", err)
	}

	hdr := DetectHDR(resp)
	if !hdr.IsHDR {
		t.Error("IsHDR = false, want true for HDR HLG content")
	}
	if hdr.TransferCharacteristics != "HLG" {
		t.Errorf("TransferCharacteristics = %q, want %q", hdr.TransferCharacteristics, "HLG")
	}
}

func TestDetectHDR_NoVideoTrack(t *testing.T) {
	data := loadTestData(t, "video_no_video_track.json")
	resp, err := parseMediaInfoOutput(data)
	if err != nil {
		t.Fatalf("parseMediaInfoOutput() error = %v", err)
	}

	hdr := DetectHDR(resp)
	if hdr.IsHDR {
		t.Error("IsHDR = true, want false when no video track")
	}
}

func TestGetAudioChannels(t *testing.T) {
	data := loadTestData(t, "video_hdr_pq.json")
	resp, err := parseMediaInfoOutput(data)
	if err != nil {
		t.Fatalf("parseMediaInfoOutput() error = %v", err)
	}

	channels := GetAudioChannels(resp)
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

func TestDetectHDRFromMetadata(t *testing.T) {
	tests := []struct {
		name     string
		primaries string
		transfer  string
		matrix    string
		wantHDR  bool
	}{
		{
			name:     "SDR BT.709",
			primaries: "BT.709",
			transfer:  "BT.709",
			matrix:    "BT.709",
			wantHDR:  false,
		},
		{
			name:     "HDR PQ with BT.2020",
			primaries: "BT.2020",
			transfer:  "PQ",
			matrix:    "BT.2020 non-constant",
			wantHDR:  true,
		},
		{
			name:     "HDR HLG",
			primaries: "BT.2020",
			transfer:  "HLG",
			matrix:    "BT.2020 non-constant",
			wantHDR:  true,
		},
		{
			name:     "BT.2020 primaries only",
			primaries: "BT.2020",
			transfer:  "BT.709",
			matrix:    "BT.709",
			wantHDR:  true,
		},
		{
			name:     "SMPTE 2084 transfer",
			primaries: "BT.709",
			transfer:  "SMPTE 2084",
			matrix:    "BT.709",
			wantHDR:  true,
		},
		{
			name:     "BT.2100 primaries",
			primaries: "BT.2100",
			transfer:  "BT.709",
			matrix:    "BT.709",
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
			got := detectHDRFromMetadata(tt.primaries, tt.transfer, tt.matrix)
			if got != tt.wantHDR {
				t.Errorf("detectHDRFromMetadata(%q, %q, %q) = %v, want %v",
					tt.primaries, tt.transfer, tt.matrix, got, tt.wantHDR)
			}
		})
	}
}
