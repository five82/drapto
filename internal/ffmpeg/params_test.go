package ffmpeg

import (
	"strings"
	"testing"
)

func TestSvtAv1ParamsBuilder(t *testing.T) {
	tests := []struct {
		name     string
		build    func() string
		contains []string
	}{
		{
			name: "basic params",
			build: func() string {
				return NewSvtAv1ParamsBuilder().
					WithACBias(0.1).
					WithEnableVarianceBoost(true).
					WithVarianceBoostStrength(1).
					WithVarianceOctile(7).
					Build()
			},
			contains: []string{"ac-bias=0.1", "enable-variance-boost=1", "variance-boost-strength=1", "variance-octile=7"},
		},
		{
			name: "variance disabled",
			build: func() string {
				return NewSvtAv1ParamsBuilder().
					WithACBias(0.0).
					WithEnableVarianceBoost(false).
					WithTune(3).
					Build()
			},
			contains: []string{"ac-bias=0", "enable-variance-boost=0", "tune=3"},
		},
		{
			name: "custom params",
			build: func() string {
				return NewSvtAv1ParamsBuilder().
					AddParam("keyint", "10s").
					AddParam("scd", "1").
					Build()
			},
			contains: []string{"keyint=10s", "scd=1"},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := tt.build()
			for _, want := range tt.contains {
				if !strings.Contains(result, want) {
					t.Errorf("result %q does not contain %q", result, want)
				}
			}
		})
	}
}

func TestVideoFilterChain(t *testing.T) {
	tests := []struct {
		name  string
		build func() string
		want  string
	}{
		{
			name: "empty chain",
			build: func() string {
				return NewVideoFilterChain().Build()
			},
			want: "",
		},
		{
			name: "single crop",
			build: func() string {
				return NewVideoFilterChain().AddCrop("crop=1920:800:0:140").Build()
			},
			want: "crop=1920:800:0:140",
		},
		{
			name: "crop and filter",
			build: func() string {
				return NewVideoFilterChain().
					AddCrop("crop=1920:800:0:140").
					AddFilter("scale=1920:1080").
					Build()
			},
			want: "crop=1920:800:0:140,scale=1920:1080",
		},
		{
			name: "empty filters ignored",
			build: func() string {
				return NewVideoFilterChain().
					AddCrop("").
					AddFilter("").
					AddCrop("crop=1920:1080:0:0").
					Build()
			},
			want: "crop=1920:1080:0:0",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := tt.build()
			if got != tt.want {
				t.Errorf("got %q, want %q", got, tt.want)
			}
		})
	}
}
