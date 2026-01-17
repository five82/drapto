// Package ffmpeg provides FFmpeg command building and execution.
package ffmpeg

import (
	"fmt"
	"strings"
)

// SvtAv1ParamsBuilder builds SVT-AV1 parameters with method chaining.
type SvtAv1ParamsBuilder struct {
	params []paramKV
}

type paramKV struct {
	key   string
	value string
}

// NewSvtAv1ParamsBuilder creates a new SVT-AV1 parameters builder.
func NewSvtAv1ParamsBuilder() *SvtAv1ParamsBuilder {
	return &SvtAv1ParamsBuilder{}
}

// WithTune sets the tune parameter.
func (b *SvtAv1ParamsBuilder) WithTune(tune uint8) *SvtAv1ParamsBuilder {
	b.params = append(b.params, paramKV{"tune", fmt.Sprintf("%d", tune)})
	return b
}

// WithACBias sets the ac-bias parameter.
func (b *SvtAv1ParamsBuilder) WithACBias(acBias float32) *SvtAv1ParamsBuilder {
	b.params = append(b.params, paramKV{"ac-bias", fmt.Sprintf("%g", acBias)})
	return b
}

// WithEnableVarianceBoost enables or disables variance boost.
func (b *SvtAv1ParamsBuilder) WithEnableVarianceBoost(enabled bool) *SvtAv1ParamsBuilder {
	val := "0"
	if enabled {
		val = "1"
	}
	b.params = append(b.params, paramKV{"enable-variance-boost", val})
	return b
}

// WithVarianceBoostStrength sets variance boost strength.
func (b *SvtAv1ParamsBuilder) WithVarianceBoostStrength(strength uint8) *SvtAv1ParamsBuilder {
	b.params = append(b.params, paramKV{"variance-boost-strength", fmt.Sprintf("%d", strength)})
	return b
}

// WithVarianceOctile sets variance octile.
func (b *SvtAv1ParamsBuilder) WithVarianceOctile(octile uint8) *SvtAv1ParamsBuilder {
	b.params = append(b.params, paramKV{"variance-octile", fmt.Sprintf("%d", octile)})
	return b
}

// AddParam adds a custom parameter.
func (b *SvtAv1ParamsBuilder) AddParam(key, value string) *SvtAv1ParamsBuilder {
	b.params = append(b.params, paramKV{key, value})
	return b
}

// Build builds the parameters into a colon-separated string.
func (b *SvtAv1ParamsBuilder) Build() string {
	var parts []string
	for _, p := range b.params {
		parts = append(parts, fmt.Sprintf("%s=%s", p.key, p.value))
	}
	return strings.Join(parts, ":")
}
