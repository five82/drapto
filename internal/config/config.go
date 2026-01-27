// Package config provides configuration types and defaults for drapto.
package config

import (
	"fmt"
	"strings"
)

// Default constants
const (
	// DefaultCRFSD is the CRF for Standard Definition videos (<1920 width).
	DefaultCRFSD uint8 = 25

	// DefaultCRFHD is the CRF for High Definition videos (>=1920 width, <3840 width).
	DefaultCRFHD uint8 = 27

	// DefaultCRFUHD is the CRF for Ultra High Definition videos (>=3840 width).
	DefaultCRFUHD uint8 = 29

	// DefaultSVTAV1Preset is the SVT-AV1 preset (0-13, lower is slower/better).
	DefaultSVTAV1Preset uint8 = 6

	// DefaultSVTAV1Tune is the SVT-AV1 tune parameter.
	DefaultSVTAV1Tune uint8 = 0

	// DefaultSVTAV1ACBias is the SVT-AV1 ac-bias parameter.
	DefaultSVTAV1ACBias float32 = 0.1

	// DefaultSVTAV1EnableVarianceBoost is whether variance boost is enabled.
	DefaultSVTAV1EnableVarianceBoost bool = false

	// DefaultSVTAV1VarianceBoostStrength is the variance boost strength.
	DefaultSVTAV1VarianceBoostStrength uint8 = 0

	// DefaultSVTAV1VarianceOctile is the variance octile parameter.
	DefaultSVTAV1VarianceOctile uint8 = 0

	// DefaultCropMode is the crop mode for the main encode.
	DefaultCropMode string = "auto"

	// UHDWidthThreshold is the width threshold for Ultra High Definition (4K).
	UHDWidthThreshold uint32 = 3840

	// HDWidthThreshold is the width threshold for High Definition.
	HDWidthThreshold uint32 = 1920

	// DefaultEncodeCooldownSecs is the cooldown period between encodes.
	DefaultEncodeCooldownSecs uint64 = 3

	// ProgressLogIntervalPercent is the progress logging interval.
	ProgressLogIntervalPercent uint8 = 5

	// MaxSVTPreset is the maximum valid SVT-AV1 preset value.
	MaxSVTPreset uint8 = 13

	// MaxCRF is the maximum valid CRF value.
	MaxCRF uint8 = 63

	// CleanPresetCRFSD is the CRF for SD videos in Clean preset.
	CleanPresetCRFSD uint8 = 27

	// CleanPresetCRFHD is the CRF for HD videos in Clean preset.
	CleanPresetCRFHD uint8 = 29

	// CleanPresetCRFUHD is the CRF for UHD videos in Clean preset.
	CleanPresetCRFUHD uint8 = 31

	// CleanPresetACBias is the AC bias for Clean preset.
	CleanPresetACBias float32 = 0.05

	// QuickPresetCRFSD is the CRF for SD videos in Quick preset.
	QuickPresetCRFSD uint8 = 32

	// QuickPresetCRFHD is the CRF for HD videos in Quick preset.
	QuickPresetCRFHD uint8 = 35

	// QuickPresetCRFUHD is the CRF for UHD videos in Quick preset.
	QuickPresetCRFUHD uint8 = 36

	// QuickPresetSVTAV1Preset is the SVT-AV1 preset for Quick mode.
	QuickPresetSVTAV1Preset uint8 = 8
)

// Preset represents a Drapto preset grouping.
type Preset string

const (
	PresetGrain Preset = "grain"
	PresetClean Preset = "clean"
	PresetQuick Preset = "quick"
)

// ParsePreset parses a string into a Preset.
func ParsePreset(s string) (Preset, error) {
	switch strings.ToLower(s) {
	case "grain":
		return PresetGrain, nil
	case "clean":
		return PresetClean, nil
	case "quick":
		return PresetQuick, nil
	default:
		return "", fmt.Errorf("%w: '%s', valid options: grain, clean, quick", ErrInvalidPreset, s)
	}
}

// String returns the string representation of the preset.
func (p Preset) String() string {
	return string(p)
}

// PresetValues contains bundled parameter values for a preset.
type PresetValues struct {
	CRFSD                       uint8
	CRFHD                       uint8
	CRFUHD                      uint8
	SVTAV1Preset                uint8
	SVTAV1Tune                  uint8
	SVTAV1ACBias                float32
	SVTAV1EnableVarianceBoost   bool
	SVTAV1VarianceBoostStrength uint8
	SVTAV1VarianceOctile        uint8
	VideoDenoiseFilter          string // Empty means none
	SVTAV1FilmGrain             *uint8 // nil means none
	SVTAV1FilmGrainDenoise      *bool  // nil means none
}

// GetPresetValues returns the values for a given preset.
func GetPresetValues(p Preset) PresetValues {
	switch p {
	case PresetGrain:
		return PresetValues{
			CRFSD:                       DefaultCRFSD,
			CRFHD:                       DefaultCRFHD,
			CRFUHD:                      DefaultCRFUHD,
			SVTAV1Preset:                DefaultSVTAV1Preset,
			SVTAV1Tune:                  DefaultSVTAV1Tune,
			SVTAV1ACBias:                DefaultSVTAV1ACBias,
			SVTAV1EnableVarianceBoost:   DefaultSVTAV1EnableVarianceBoost,
			SVTAV1VarianceBoostStrength: DefaultSVTAV1VarianceBoostStrength,
			SVTAV1VarianceOctile:        DefaultSVTAV1VarianceOctile,
		}
	case PresetClean:
		return PresetValues{
			CRFSD:                       CleanPresetCRFSD,
			CRFHD:                       CleanPresetCRFHD,
			CRFUHD:                      CleanPresetCRFUHD,
			SVTAV1Preset:                DefaultSVTAV1Preset,
			SVTAV1Tune:                  DefaultSVTAV1Tune,
			SVTAV1ACBias:                CleanPresetACBias,
			SVTAV1EnableVarianceBoost:   DefaultSVTAV1EnableVarianceBoost,
			SVTAV1VarianceBoostStrength: DefaultSVTAV1VarianceBoostStrength,
			SVTAV1VarianceOctile:        DefaultSVTAV1VarianceOctile,
		}
	case PresetQuick:
		return PresetValues{
			CRFSD:                       QuickPresetCRFSD,
			CRFHD:                       QuickPresetCRFHD,
			CRFUHD:                      QuickPresetCRFUHD,
			SVTAV1Preset:                QuickPresetSVTAV1Preset,
			SVTAV1Tune:                  DefaultSVTAV1Tune,
			SVTAV1ACBias:                DefaultSVTAV1ACBias,
			SVTAV1EnableVarianceBoost:   DefaultSVTAV1EnableVarianceBoost,
			SVTAV1VarianceBoostStrength: DefaultSVTAV1VarianceBoostStrength,
			SVTAV1VarianceOctile:        DefaultSVTAV1VarianceOctile,
		}
	default:
		// Return grain preset as default
		return GetPresetValues(PresetGrain)
	}
}

// Config holds all configuration for video processing.
type Config struct {
	// Input/output paths
	InputDir  string
	OutputDir string
	LogDir    string
	TempDir   string // Optional, defaults to OutputDir

	// SVT-AV1 parameters
	SVTAV1Preset                uint8
	SVTAV1Tune                  uint8
	SVTAV1ACBias                float32
	SVTAV1EnableVarianceBoost   bool
	SVTAV1VarianceBoostStrength uint8
	SVTAV1VarianceOctile        uint8

	// Optional filters and film grain
	VideoDenoiseFilter     string // Optional denoise filter (e.g., "hqdn3d=1.5:1.5:3:3")
	SVTAV1FilmGrain        *uint8 // Optional film grain synthesis strength
	SVTAV1FilmGrainDenoise *bool  // Optional film grain denoise toggle

	// CRF settings (values 0-63)
	CRFSD  uint8
	CRFHD  uint8
	CRFUHD uint8

	// Processing options
	CropMode           string // "auto" or "none"
	ResponsiveEncoding bool   // Reserve CPU threads for responsiveness
	EncodeCooldownSecs uint64 // Cooldown between batch encodes

	// Selected preset (optional)
	DraptoPreset *Preset
}

// NewConfig creates a new Config with default values.
func NewConfig(inputDir, outputDir, logDir string) *Config {
	return &Config{
		InputDir:                    inputDir,
		OutputDir:                   outputDir,
		LogDir:                      logDir,
		SVTAV1Preset:                DefaultSVTAV1Preset,
		SVTAV1Tune:                  DefaultSVTAV1Tune,
		SVTAV1ACBias:                DefaultSVTAV1ACBias,
		SVTAV1EnableVarianceBoost:   DefaultSVTAV1EnableVarianceBoost,
		SVTAV1VarianceBoostStrength: DefaultSVTAV1VarianceBoostStrength,
		SVTAV1VarianceOctile:        DefaultSVTAV1VarianceOctile,
		CRFSD:                       DefaultCRFSD,
		CRFHD:                       DefaultCRFHD,
		CRFUHD:                      DefaultCRFUHD,
		CropMode:                    DefaultCropMode,
		ResponsiveEncoding:          false,
		EncodeCooldownSecs:          DefaultEncodeCooldownSecs,
	}
}

// ApplyPreset applies the given preset to the config.
func (c *Config) ApplyPreset(p Preset) {
	values := GetPresetValues(p)
	c.DraptoPreset = &p
	c.CRFSD = values.CRFSD
	c.CRFHD = values.CRFHD
	c.CRFUHD = values.CRFUHD
	c.SVTAV1Preset = values.SVTAV1Preset
	c.SVTAV1Tune = values.SVTAV1Tune
	c.SVTAV1ACBias = values.SVTAV1ACBias
	c.SVTAV1EnableVarianceBoost = values.SVTAV1EnableVarianceBoost
	c.SVTAV1VarianceBoostStrength = values.SVTAV1VarianceBoostStrength
	c.SVTAV1VarianceOctile = values.SVTAV1VarianceOctile
	c.VideoDenoiseFilter = values.VideoDenoiseFilter
	c.SVTAV1FilmGrain = values.SVTAV1FilmGrain
	c.SVTAV1FilmGrainDenoise = values.SVTAV1FilmGrainDenoise
}

// Validate checks the configuration for errors.
func (c *Config) Validate() error {
	if c.SVTAV1Preset > MaxSVTPreset {
		return fmt.Errorf("%w: must be 0-%d, got %d", ErrInvalidSVTPreset, MaxSVTPreset, c.SVTAV1Preset)
	}

	if c.CRFSD > MaxCRF {
		return fmt.Errorf("%w: crf_sd must be 0-%d, got %d", ErrInvalidCRF, MaxCRF, c.CRFSD)
	}

	if c.CRFHD > MaxCRF {
		return fmt.Errorf("%w: crf_hd must be 0-%d, got %d", ErrInvalidCRF, MaxCRF, c.CRFHD)
	}

	if c.CRFUHD > MaxCRF {
		return fmt.Errorf("%w: crf_uhd must be 0-%d, got %d", ErrInvalidCRF, MaxCRF, c.CRFUHD)
	}

	if c.SVTAV1FilmGrain == nil && c.SVTAV1FilmGrainDenoise != nil {
		return fmt.Errorf("%w: denoise set without film grain", ErrInvalidFilmGrain)
	}

	return nil
}

// GetTempDir returns the temp directory, falling back to OutputDir if not set.
func (c *Config) GetTempDir() string {
	if c.TempDir != "" {
		return c.TempDir
	}
	return c.OutputDir
}

// CRFForWidth returns the appropriate CRF value based on video width.
func (c *Config) CRFForWidth(width uint32) uint8 {
	if width >= UHDWidthThreshold {
		return c.CRFUHD
	}
	if width >= HDWidthThreshold {
		return c.CRFHD
	}
	return c.CRFSD
}
