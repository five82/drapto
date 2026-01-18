// Package config provides configuration types and defaults for drapto.
package config

import (
	"fmt"
	"runtime"
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

	// DefaultSceneThreshold is the threshold for scene change detection.
	// Higher values = fewer scene changes detected. Range is 0.0 to 1.0.
	DefaultSceneThreshold float64 = 0.4
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
		return "", fmt.Errorf("unknown preset '%s', valid options: grain, clean, quick", s)
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

// AutoParallelConfig returns optimal workers and buffer settings based on CPU cores.
// Workers: 1 per 8 cores, min 1, max 4
// Buffer: matches workers (ensures next chunk is always ready)
func AutoParallelConfig() (workers, buffer int) {
	numCPU := runtime.NumCPU()

	// 1 worker per 8 cores, minimum 1, maximum 4
	workers = numCPU / 8
	if workers < 1 {
		workers = 1
	}
	if workers > 4 {
		workers = 4
	}

	// Buffer matches workers
	buffer = workers

	return workers, buffer
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

	// Parallel encoding options
	Workers     int // Number of parallel encoder workers
	ChunkBuffer int // Extra chunks to buffer in memory

	// Target quality options
	TargetQuality string // Target quality range (e.g., "70-75" for SSIMULACRA2 score)
	QPRange       string // CRF search range (default "8-48")
	MetricWorkers int    // Number of GPU metric workers (default 1)
	MetricMode    string // Metric aggregation mode ("mean" or "pN")

	// Scene detection options
	SceneThreshold float64 // Scene change detection threshold (0.0-1.0, higher = fewer scenes)

	// Selected preset (optional)
	DraptoPreset *Preset
}

// NewConfig creates a new Config with default values.
func NewConfig(inputDir, outputDir, logDir string) *Config {
	workers, buffer := AutoParallelConfig()

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
		Workers:                     workers,
		ChunkBuffer:                 buffer,
		// Target quality defaults
		QPRange:       "8-48",
		MetricWorkers: 1,
		MetricMode:    "mean",
		// Scene detection defaults
		SceneThreshold: DefaultSceneThreshold,
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
		return fmt.Errorf("svt_av1_preset must be 0-13, got %d", c.SVTAV1Preset)
	}

	if c.QualitySD > 63 {
		return fmt.Errorf("quality_sd must be 0-63, got %d", c.QualitySD)
	}

	if c.QualityHD > 63 {
		return fmt.Errorf("quality_hd must be 0-63, got %d", c.QualityHD)
	}

	if c.QualityUHD > 63 {
		return fmt.Errorf("quality_uhd must be 0-63, got %d", c.QualityUHD)
	}

	if c.SVTAV1FilmGrain == nil && c.SVTAV1FilmGrainDenoise != nil {
		return fmt.Errorf("svt_av1_film_grain_denoise set without svt_av1_film_grain")
	}

	if c.Workers < 1 {
		return fmt.Errorf("workers must be at least 1, got %d", c.Workers)
	}

	if c.ChunkBuffer < 0 {
		return fmt.Errorf("chunk_buffer must be non-negative, got %d", c.ChunkBuffer)
	}

	if c.SceneThreshold < 0 || c.SceneThreshold > 1 {
		return fmt.Errorf("scene_threshold must be between 0.0 and 1.0, got %g", c.SceneThreshold)
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
