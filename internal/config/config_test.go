package config

import (
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
	if cfg.QualitySD != DefaultQualitySD {
		t.Errorf("expected QualitySD=%d, got %d", DefaultQualitySD, cfg.QualitySD)
	}
}

func TestConfigValidate(t *testing.T) {
	tests := []struct {
		name    string
		modify  func(*Config)
		wantErr bool
	}{
		{
			name:    "default config is valid",
			modify:  func(c *Config) {},
			wantErr: false,
		},
		{
			name:    "preset 14 is invalid",
			modify:  func(c *Config) { c.SVTAV1Preset = 14 },
			wantErr: true,
		},
		{
			name:    "preset 13 is valid",
			modify:  func(c *Config) { c.SVTAV1Preset = 13 },
			wantErr: false,
		},
		{
			name:    "quality_sd 64 is invalid",
			modify:  func(c *Config) { c.QualitySD = 64 },
			wantErr: true,
		},
		{
			name:    "quality_hd 64 is invalid",
			modify:  func(c *Config) { c.QualityHD = 64 },
			wantErr: true,
		},
		{
			name:    "quality_uhd 64 is invalid",
			modify:  func(c *Config) { c.QualityUHD = 64 },
			wantErr: true,
		},
		{
			name: "film_grain_denoise without film_grain is invalid",
			modify: func(c *Config) {
				b := true
				c.SVTAV1FilmGrainDenoise = &b
			},
			wantErr: true,
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
		})
	}
}

