// Package config provides configuration types and defaults for drapto.
package config

import (
	"fmt"
	"strings"
)

// Default constants
const (
	// DefaultQualitySD is the CRF for Standard Definition videos (<1920 width).
	DefaultQualitySD uint8 = 25

	// DefaultQualityHD is the CRF for High Definition videos (>=1920 width, <3840 width).
	DefaultQualityHD uint8 = 27

	// DefaultQualityUHD is the CRF for Ultra High Definition videos (>=3840 width).
	DefaultQualityUHD uint8 = 29

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
	QualitySD                   uint8
	QualityHD                   uint8
	QualityUHD                  uint8
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
			QualitySD:                   DefaultQualitySD,
			QualityHD:                   DefaultQualityHD,
			QualityUHD:                  DefaultQualityUHD,
			SVTAV1Preset:                DefaultSVTAV1Preset,
			SVTAV1Tune:                  DefaultSVTAV1Tune,
			SVTAV1ACBias:                DefaultSVTAV1ACBias,
			SVTAV1EnableVarianceBoost:   DefaultSVTAV1EnableVarianceBoost,
			SVTAV1VarianceBoostStrength: DefaultSVTAV1VarianceBoostStrength,
			SVTAV1VarianceOctile:        DefaultSVTAV1VarianceOctile,
		}
	case PresetClean:
		return PresetValues{
			QualitySD:                   27,
			QualityHD:                   29,
			QualityUHD:                  31,
			SVTAV1Preset:                DefaultSVTAV1Preset,
			SVTAV1Tune:                  DefaultSVTAV1Tune,
			SVTAV1ACBias:                0.05,
			SVTAV1EnableVarianceBoost:   false,
			SVTAV1VarianceBoostStrength: 0,
			SVTAV1VarianceOctile:        0,
		}
	case PresetQuick:
		return PresetValues{
			QualitySD:                   32,
			QualityHD:                   35,
			QualityUHD:                  36,
			SVTAV1Preset:                8,
			SVTAV1Tune:                  DefaultSVTAV1Tune,
			SVTAV1ACBias:                0.0,
			SVTAV1EnableVarianceBoost:   false,
			SVTAV1VarianceBoostStrength: 0,
			SVTAV1VarianceOctile:        0,
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

	// Quality settings (CRF values 0-63)
	QualitySD  uint8
	QualityHD  uint8
	QualityUHD uint8

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
		QualitySD:                   DefaultQualitySD,
		QualityHD:                   DefaultQualityHD,
		QualityUHD:                  DefaultQualityUHD,
		CropMode:                    DefaultCropMode,
		ResponsiveEncoding:          false,
		EncodeCooldownSecs:          DefaultEncodeCooldownSecs,
	}
}

// ApplyPreset applies the given preset to the config.
func (c *Config) ApplyPreset(p Preset) {
	values := GetPresetValues(p)
	c.DraptoPreset = &p
	c.QualitySD = values.QualitySD
	c.QualityHD = values.QualityHD
	c.QualityUHD = values.QualityUHD
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
	if c.SVTAV1Preset > 13 {
		return fmt.Errorf("%w: must be 0-13, got %d", ErrInvalidSVTPreset, c.SVTAV1Preset)
	}

	if c.QualitySD > 63 {
		return fmt.Errorf("%w: quality_sd must be 0-63, got %d", ErrInvalidCRF, c.QualitySD)
	}

	if c.QualityHD > 63 {
		return fmt.Errorf("%w: quality_hd must be 0-63, got %d", ErrInvalidCRF, c.QualityHD)
	}

	if c.QualityUHD > 63 {
		return fmt.Errorf("%w: quality_uhd must be 0-63, got %d", ErrInvalidCRF, c.QualityUHD)
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
