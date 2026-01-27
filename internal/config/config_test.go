package config

import (
	"errors"
	"testing"
)

func TestNewConfig(t *testing.T) {
	cfg := NewConfig("/input", "/output", "/log")

	if cfg.InputDir != "/input" {
		t.Errorf("expected InputDir=/input, got %s", cfg.InputDir)
	}
	if cfg.OutputDir != "/output" {
		t.Errorf("expected OutputDir=/output, got %s", cfg.OutputDir)
	}
	if cfg.LogDir != "/log" {
		t.Errorf("expected LogDir=/log, got %s", cfg.LogDir)
	}

	// Check defaults
	if cfg.SVTAV1Preset != DefaultSVTAV1Preset {
		t.Errorf("expected SVTAV1Preset=%d, got %d", DefaultSVTAV1Preset, cfg.SVTAV1Preset)
	}
	if cfg.CRFSD != DefaultCRFSD {
		t.Errorf("expected CRFSD=%d, got %d", DefaultCRFSD, cfg.CRFSD)
	}
}

func TestConfigValidate(t *testing.T) {
	tests := []struct {
		name        string
		modify      func(*Config)
		wantErr     bool
		wantSentinel error
	}{
		{
			name:    "default config is valid",
			modify:  func(c *Config) {},
			wantErr: false,
		},
		{
			name:         "preset 14 is invalid",
			modify:       func(c *Config) { c.SVTAV1Preset = 14 },
			wantErr:      true,
			wantSentinel: ErrInvalidSVTPreset,
		},
		{
			name:    "preset 13 is valid",
			modify:  func(c *Config) { c.SVTAV1Preset = 13 },
			wantErr: false,
		},
		{
			name:         "crf_sd 64 is invalid",
			modify:       func(c *Config) { c.CRFSD = 64 },
			wantErr:      true,
			wantSentinel: ErrInvalidCRF,
		},
		{
			name:         "crf_hd 64 is invalid",
			modify:       func(c *Config) { c.CRFHD = 64 },
			wantErr:      true,
			wantSentinel: ErrInvalidCRF,
		},
		{
			name:         "crf_uhd 64 is invalid",
			modify:       func(c *Config) { c.CRFUHD = 64 },
			wantErr:      true,
			wantSentinel: ErrInvalidCRF,
		},
		{
			name: "film_grain_denoise without film_grain is invalid",
			modify: func(c *Config) {
				b := true
				c.SVTAV1FilmGrainDenoise = &b
			},
			wantErr:      true,
			wantSentinel: ErrInvalidFilmGrain,
		},
		{
			name: "film_grain with denoise is valid",
			modify: func(c *Config) {
				fg := uint8(6)
				b := true
				c.SVTAV1FilmGrain = &fg
				c.SVTAV1FilmGrainDenoise = &b
			},
			wantErr: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			cfg := NewConfig("/input", "/output", "/log")
			tt.modify(cfg)
			err := cfg.Validate()
			if (err != nil) != tt.wantErr {
				t.Errorf("Validate() error = %v, wantErr %v", err, tt.wantErr)
			}
			if tt.wantSentinel != nil && !errors.Is(err, tt.wantSentinel) {
				t.Errorf("Validate() error = %v, want sentinel %v", err, tt.wantSentinel)
			}
		})
	}
}

func TestParsePreset(t *testing.T) {
	tests := []struct {
		input        string
		want         Preset
		wantErr      bool
		wantSentinel error
	}{
		{"grain", PresetGrain, false, nil},
		{"GRAIN", PresetGrain, false, nil},
		{"Grain", PresetGrain, false, nil},
		{"clean", PresetClean, false, nil},
		{"CLEAN", PresetClean, false, nil},
		{"quick", PresetQuick, false, nil},
		{"QUICK", PresetQuick, false, nil},
		{"invalid", "", true, ErrInvalidPreset},
		{"", "", true, ErrInvalidPreset},
	}

	for _, tt := range tests {
		t.Run(tt.input, func(t *testing.T) {
			got, err := ParsePreset(tt.input)
			if (err != nil) != tt.wantErr {
				t.Errorf("ParsePreset(%q) error = %v, wantErr %v", tt.input, err, tt.wantErr)
				return
			}
			if tt.wantSentinel != nil && !errors.Is(err, tt.wantSentinel) {
				t.Errorf("ParsePreset(%q) error = %v, want sentinel %v", tt.input, err, tt.wantSentinel)
			}
			if got != tt.want {
				t.Errorf("ParsePreset(%q) = %v, want %v", tt.input, got, tt.want)
			}
		})
	}
}

func TestApplyPreset(t *testing.T) {
	cfg := NewConfig("/input", "/output", "/log")

	// Modify some values
	cfg.CRFSD = 1
	cfg.SVTAV1Preset = 13

	// Apply preset
	cfg.ApplyPreset(PresetGrain)

	// Check that preset was applied
	if cfg.DraptoPreset == nil || *cfg.DraptoPreset != PresetGrain {
		t.Error("expected DraptoPreset to be set to Grain")
	}

	grainValues := GetPresetValues(PresetGrain)
	if cfg.CRFSD != grainValues.CRFSD {
		t.Errorf("expected CRFSD=%d, got %d", grainValues.CRFSD, cfg.CRFSD)
	}
	if cfg.SVTAV1Preset != grainValues.SVTAV1Preset {
		t.Errorf("expected SVTAV1Preset=%d, got %d", grainValues.SVTAV1Preset, cfg.SVTAV1Preset)
	}
}

func TestGetPresetValues(t *testing.T) {
	// Test that Quick preset has higher CRF values (higher CRF = lower quality)
	quickValues := GetPresetValues(PresetQuick)
	grainValues := GetPresetValues(PresetGrain)

	if quickValues.CRFSD <= grainValues.CRFSD {
		t.Error("expected Quick preset to have higher CRF than Grain")
	}
	if quickValues.SVTAV1Preset <= grainValues.SVTAV1Preset {
		t.Error("expected Quick preset to have higher (faster) preset than Grain")
	}
}

func TestCRFForWidth(t *testing.T) {
	cfg := NewConfig("/input", "/output", "/log")
	cfg.CRFSD = 25
	cfg.CRFHD = 27
	cfg.CRFUHD = 29

	tests := []struct {
		width    uint32
		expected uint8
	}{
		{1280, 25},  // SD
		{1919, 25},  // SD (below HD threshold)
		{1920, 27},  // HD
		{2560, 27},  // HD
		{3839, 27},  // HD (below UHD threshold)
		{3840, 29},  // UHD
		{7680, 29},  // UHD (8K)
	}

	for _, tt := range tests {
		got := cfg.CRFForWidth(tt.width)
		if got != tt.expected {
			t.Errorf("CRFForWidth(%d) = %d, want %d", tt.width, got, tt.expected)
		}
	}
}
