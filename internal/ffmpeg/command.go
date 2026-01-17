package ffmpeg

import (
	"fmt"

	"github.com/five82/drapto/internal/ffprobe"
)

// EncodeParams contains all parameters for an FFmpeg encode operation.
type EncodeParams struct {
	InputPath             string
	OutputPath            string
	Quality               uint32
	Preset                uint8
	Tune                  uint8
	ACBias                float32
	EnableVarianceBoost   bool
	VarianceBoostStrength uint8
	VarianceOctile        uint8
	VideoDenoiseFilter string // Optional denoise filter
	FilmGrain          *uint8 // Optional film grain synthesis strength
	FilmGrainDenoise   *bool  // Optional film grain denoise toggle
	CropFilter         string // Optional crop filter
	AudioChannels         []uint32
	AudioStreams          []ffprobe.AudioStreamInfo
	Duration              float64
	VideoCodec            string
	PixelFormat           string
	MatrixCoefficients    string
	AudioCodec            string
}

// SVTAV1CLIParams builds the -svtav1-params string.
func (p *EncodeParams) SVTAV1CLIParams() string {
	builder := NewSvtAv1ParamsBuilder().
		WithACBias(p.ACBias).
		WithEnableVarianceBoost(p.EnableVarianceBoost)

	if p.EnableVarianceBoost {
		builder = builder.
			WithVarianceBoostStrength(p.VarianceBoostStrength).
			WithVarianceOctile(p.VarianceOctile)
	}

	builder = builder.WithTune(p.Tune).
		AddParam("keyint", "10s").
		AddParam("scd", "1").
		AddParam("scm", "0")

	if p.FilmGrain != nil {
		builder = builder.AddParam("film-grain", fmt.Sprintf("%d", *p.FilmGrain))
		if p.FilmGrainDenoise != nil {
			val := "0"
			if *p.FilmGrainDenoise {
				val = "1"
			}
			builder = builder.AddParam("film-grain-denoise", val)
		}
	}

	return builder.Build()
}

// calculateAudioBitrate returns audio bitrate in kbps based on channel count.
func calculateAudioBitrate(channels uint32) uint32 {
	switch channels {
	case 1:
		return 64 // Mono
	case 2:
		return 128 // Stereo
	case 6:
		return 256 // 5.1 surround
	case 8:
		return 384 // 7.1 surround
	default:
		return channels * 48 // ~48 kbps per channel for non-standard configs
	}
}

// CalculateAudioBitrate is exported for use by other packages.
func CalculateAudioBitrate(channels uint32) uint32 {
	return calculateAudioBitrate(channels)
}
