package processing

import "testing"

func TestAnalyzeCropCountsSingleCrop(t *testing.T) {
	result := analyzeCropCounts(map[string]int{
		"1920:800:0:140": 141,
	}, 1920, 1080, "Analyzed 141 samples", 141)

	if !result.Required {
		t.Fatal("expected crop to be required")
	}
	if result.CropFilter != "crop=1920:800:0:140" {
		t.Fatalf("crop filter = %q, want crop=1920:800:0:140", result.CropFilter)
	}
	if result.MultipleRatios {
		t.Fatal("did not expect multiple ratios")
	}
}

func TestAnalyzeCropCountsJitterUsesLeastAggressiveCrop(t *testing.T) {
	result := analyzeCropCounts(map[string]int{
		"1920:1044:0:18":  104,
		"1920:1046:0:16":  16,
		"1920:1042:0:20":  9,
		"1920:1042:0:18":  7,
		"298:146:984:308": 1,
		"1920:1036:0:22":  1,
		"1920:1040:0:18":  1,
		"1918:1042:2:16":  1,
		"1920:1038:0:22":  1,
	}, 1920, 1080, "Analyzed 141 samples", 141)

	if !result.Required {
		t.Fatal("expected jitter around one matte to produce a crop")
	}
	if result.MultipleRatios {
		t.Fatal("jitter around one matte should not be reported as multiple aspect ratios")
	}
	if result.CropFilter != "crop=1920:1046:0:16" {
		t.Fatalf("crop filter = %q, want least aggressive safe crop", result.CropFilter)
	}
}

func TestAnalyzeCropCountsFullFrameSamplePreventsCrop(t *testing.T) {
	result := analyzeCropCounts(map[string]int{
		"1920:800:0:140": 100,
		"1920:1080:0:0":  41,
	}, 1920, 1080, "Analyzed 141 samples", 141)

	if result.Required {
		t.Fatalf("expected no crop for mixed full-frame content, got %q", result.CropFilter)
	}
	if !result.MultipleRatios {
		t.Fatal("expected mixed full-frame and letterboxed samples to report multiple ratios")
	}
}

func TestAnalyzeCropCountsNoCrop(t *testing.T) {
	result := analyzeCropCounts(map[string]int{
		"1920:1080:0:0": 141,
	}, 1920, 1080, "Analyzed 141 samples", 141)

	if result.Required {
		t.Fatalf("expected no crop, got %q", result.CropFilter)
	}
	if result.MultipleRatios {
		t.Fatal("did not expect multiple ratios")
	}
}
