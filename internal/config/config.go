// Package config provides configuration types and defaults for drapto.
package config

import (
	"fmt"
	"runtime"
)

// Default constants
const (
	// DefaultCRF is the default CRF quality setting.
	DefaultCRF uint8 = 27

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

	// DefaultEncodeCooldownSecs is the cooldown period between encodes.
	DefaultEncodeCooldownSecs uint64 = 3

	// ProgressLogIntervalPercent is the progress logging interval.
	ProgressLogIntervalPercent uint8 = 5

	// DefaultChunkDuration is the default chunk duration in seconds for non-4K content.
	DefaultChunkDuration float64 = 10.0

	// DefaultChunkDuration4K is the default chunk duration in seconds for 4K content.
	DefaultChunkDuration4K float64 = 20.0
)

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

	// Quality setting (CRF value 0-63)
	CRF uint8

	// Processing options
	CropMode           string // "auto" or "none"
	ResponsiveEncoding bool   // Reserve CPU threads for responsiveness
	EncodeCooldownSecs uint64 // Cooldown between batch encodes

	// Parallel encoding options
	Workers     int // Number of parallel encoder workers
	ChunkBuffer int // Extra chunks to buffer in memory

	// Chunk duration (set automatically based on resolution)
	ChunkDuration float64 // Chunk duration in seconds

	// Debug options
	Verbose bool // Enable verbose output
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
		CRF: DefaultCRF,
		CropMode:                    DefaultCropMode,
		ResponsiveEncoding:          false,
		EncodeCooldownSecs:          DefaultEncodeCooldownSecs,
		Workers:       workers,
		ChunkBuffer:   buffer,
		ChunkDuration: DefaultChunkDuration,
	}
}

// Validate checks the configuration for errors.
func (c *Config) Validate() error {
	if c.SVTAV1Preset > 13 {
		return fmt.Errorf("svt_av1_preset must be 0-13, got %d", c.SVTAV1Preset)
	}

	if c.CRF > 63 {
		return fmt.Errorf("crf must be 0-63, got %d", c.CRF)
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

	if c.ChunkDuration < 1 || c.ChunkDuration > 120 {
		return fmt.Errorf("chunk_duration must be between 1 and 120 seconds, got %g", c.ChunkDuration)
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
