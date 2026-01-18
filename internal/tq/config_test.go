package tq

import (
	"testing"
)

func TestParseTargetRange(t *testing.T) {
	tests := []struct {
		name      string
		input     string
		wantMin   float64
		wantMax   float64
		wantErr   bool
	}{
		{
			name:    "valid range",
			input:   "70-75",
			wantMin: 70,
			wantMax: 75,
		},
		{
			name:    "valid range with spaces",
			input:   " 70 - 75 ",
			wantMin: 70,
			wantMax: 75,
		},
		{
			name:    "valid decimal range",
			input:   "70.5-75.5",
			wantMin: 70.5,
			wantMax: 75.5,
		},
		{
			name:    "invalid - no separator",
			input:   "7075",
			wantErr: true,
		},
		{
			name:    "invalid - min >= max",
			input:   "75-70",
			wantErr: true,
		},
		{
			name:    "invalid - equal values",
			input:   "70-70",
			wantErr: true,
		},
		{
			name:    "invalid - non-numeric min",
			input:   "abc-75",
			wantErr: true,
		},
		{
			name:    "invalid - non-numeric max",
			input:   "70-xyz",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			cfg, err := ParseTargetRange(tt.input)

			if tt.wantErr {
				if err == nil {
					t.Errorf("ParseTargetRange(%q) error = nil, want error", tt.input)
				}
				return
			}

			if err != nil {
				t.Errorf("ParseTargetRange(%q) error = %v, want nil", tt.input, err)
				return
			}

			if cfg.TargetMin != tt.wantMin {
				t.Errorf("ParseTargetRange(%q).TargetMin = %v, want %v", tt.input, cfg.TargetMin, tt.wantMin)
			}

			if cfg.TargetMax != tt.wantMax {
				t.Errorf("ParseTargetRange(%q).TargetMax = %v, want %v", tt.input, cfg.TargetMax, tt.wantMax)
			}

			expectedTarget := (tt.wantMin + tt.wantMax) / 2
			if cfg.Target != expectedTarget {
				t.Errorf("ParseTargetRange(%q).Target = %v, want %v", tt.input, cfg.Target, expectedTarget)
			}

			expectedTolerance := (tt.wantMax - tt.wantMin) / 2
			if cfg.Tolerance != expectedTolerance {
				t.Errorf("ParseTargetRange(%q).Tolerance = %v, want %v", tt.input, cfg.Tolerance, expectedTolerance)
			}
		})
	}
}

func TestParseQPRange(t *testing.T) {
	tests := []struct {
		name    string
		input   string
		wantMin float64
		wantMax float64
		wantErr bool
	}{
		{
			name:    "default range",
			input:   "8-48",
			wantMin: 8,
			wantMax: 48,
		},
		{
			name:    "narrow range",
			input:   "20-30",
			wantMin: 20,
			wantMax: 30,
		},
		{
			name:    "decimal range",
			input:   "15.5-35.5",
			wantMin: 15.5,
			wantMax: 35.5,
		},
		{
			name:    "invalid - no separator",
			input:   "848",
			wantErr: true,
		},
		{
			name:    "invalid - min >= max",
			input:   "48-8",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			min, max, err := ParseQPRange(tt.input)

			if tt.wantErr {
				if err == nil {
					t.Errorf("ParseQPRange(%q) error = nil, want error", tt.input)
				}
				return
			}

			if err != nil {
				t.Errorf("ParseQPRange(%q) error = %v, want nil", tt.input, err)
				return
			}

			if min != tt.wantMin {
				t.Errorf("ParseQPRange(%q) min = %v, want %v", tt.input, min, tt.wantMin)
			}

			if max != tt.wantMax {
				t.Errorf("ParseQPRange(%q) max = %v, want %v", tt.input, max, tt.wantMax)
			}
		})
	}
}

func TestDefaultConfig(t *testing.T) {
	cfg := DefaultConfig()

	if cfg.QPMin != 8 {
		t.Errorf("DefaultConfig().QPMin = %v, want 8", cfg.QPMin)
	}

	if cfg.QPMax != 48 {
		t.Errorf("DefaultConfig().QPMax = %v, want 48", cfg.QPMax)
	}

	if cfg.MaxRounds != 10 {
		t.Errorf("DefaultConfig().MaxRounds = %v, want 10", cfg.MaxRounds)
	}

	if cfg.MetricMode != "mean" {
		t.Errorf("DefaultConfig().MetricMode = %v, want mean", cfg.MetricMode)
	}
}
