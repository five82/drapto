// Package validation provides post-encode validation checks.
package validation

// Result contains the overall validation result.
type Result struct {
	IsAV1                    bool
	Is10Bit                  bool
	IsCropCorrect            bool
	IsDurationCorrect        bool
	IsHDRCorrect             bool
	IsAudioOpus              bool
	IsAudioTrackCountCorrect bool
	IsSyncPreserved          bool

	// Details
	CodecName          string
	PixelFormat        string
	BitDepth           *uint8
	ActualDimensions   *[2]uint32
	ExpectedDimensions *[2]uint32
	CropMessage        string
	ActualDuration     *float64
	ExpectedDuration   *float64
	DurationMessage    string
	ExpectedHDR        *bool
	ActualHDR          *bool
	HDRMessage         string
	AudioCodecs        []string
	AudioMessage       string
	SyncDriftMs        *float64
	SyncMessage        string
}

// ValidationStep represents a single validation check.
type ValidationStep struct {
	Name    string
	Passed  bool
	Details string
}

// IsValid returns true if all validation checks passed.
func (r *Result) IsValid() bool {
	return r.IsAV1 &&
		r.Is10Bit &&
		r.IsCropCorrect &&
		r.IsDurationCorrect &&
		r.IsHDRCorrect &&
		r.IsAudioOpus &&
		r.IsAudioTrackCountCorrect &&
		r.IsSyncPreserved
}

// GetValidationSteps returns all validation steps with results.
func (r *Result) GetValidationSteps() []ValidationStep {
	steps := []ValidationStep{
		{
			Name:    "Video codec",
			Passed:  r.IsAV1,
			Details: formatCodecDetails(r.CodecName, r.IsAV1),
		},
		{
			Name:    "Bit depth",
			Passed:  r.Is10Bit,
			Details: formatBitDepthDetails(r.BitDepth, r.PixelFormat),
		},
		{
			Name:    "Crop detection",
			Passed:  r.IsCropCorrect,
			Details: r.CropMessage,
		},
		{
			Name:    "Video duration",
			Passed:  r.IsDurationCorrect,
			Details: r.DurationMessage,
		},
		{
			Name:    "HDR/SDR status",
			Passed:  r.IsHDRCorrect,
			Details: r.HDRMessage,
		},
		{
			Name:    "Audio tracks",
			Passed:  r.IsAudioOpus && r.IsAudioTrackCountCorrect,
			Details: r.AudioMessage,
		},
		{
			Name:    "Audio/video sync",
			Passed:  r.IsSyncPreserved,
			Details: r.SyncMessage,
		},
	}
	return steps
}

// GetFailures returns descriptions of failed validation checks.
func (r *Result) GetFailures() []string {
	var failures []string
	for _, step := range r.GetValidationSteps() {
		if !step.Passed {
			failures = append(failures, step.Name+": "+step.Details)
		}
	}
	return failures
}

func formatCodecDetails(codecName string, passed bool) string {
	if passed {
		return "AV1 (" + codecName + ")"
	}
	if codecName != "" {
		return "Expected AV1, got " + codecName
	}
	return "Unknown codec"
}

func formatBitDepthDetails(bitDepth *uint8, pixelFormat string) string {
	if bitDepth != nil {
		return formatWithPixFmt(*bitDepth, pixelFormat)
	}
	if pixelFormat != "" {
		return "Pixel format: " + pixelFormat
	}
	return "Unknown bit depth"
}

func formatWithPixFmt(depth uint8, pxfmt string) string {
	if pxfmt != "" {
		return formatDepth(depth) + " (" + pxfmt + ")"
	}
	return formatDepth(depth)
}

func formatDepth(depth uint8) string {
	switch depth {
	case 8:
		return "8-bit"
	case 10:
		return "10-bit"
	case 12:
		return "12-bit"
	default:
		return ""
	}
}
