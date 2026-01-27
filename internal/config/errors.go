// Package config provides configuration types and defaults for drapto.
package config

import "errors"

// Sentinel errors for configuration validation.
var (
	// ErrInvalidPreset indicates an unknown preset name was provided.
	ErrInvalidPreset = errors.New("invalid preset")

	// ErrInvalidCRF indicates a CRF value outside the valid 0-63 range.
	ErrInvalidCRF = errors.New("CRF value out of range")

	// ErrInvalidSVTPreset indicates an SVT-AV1 preset outside the valid 0-13 range.
	ErrInvalidSVTPreset = errors.New("SVT-AV1 preset out of range")

	// ErrInvalidFilmGrain indicates film grain denoise was set without film grain.
	ErrInvalidFilmGrain = errors.New("film grain configuration invalid")
)
