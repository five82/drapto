// Package tq provides target quality encoding support with iterative CRF search.
package tq

import (
	"fmt"
	"strconv"
	"strings"
)

// Config holds target quality configuration.
type Config struct {
	// TargetMin and TargetMax define the acceptable SSIMULACRA2 score range.
	TargetMin float64
	TargetMax float64

	// Target is the midpoint of the range.
	Target float64

	// Tolerance is half the range width.
	Tolerance float64

	// QPMin and QPMax define the CRF search bounds.
	QPMin float64
	QPMax float64

	// MaxRounds is the maximum number of iterations before accepting best result.
	MaxRounds int

	// MetricMode specifies how to aggregate frame scores ("mean" or "pN").
	MetricMode string
}

// DefaultConfig returns a Config with sensible defaults.
func DefaultConfig() *Config {
	return &Config{
		QPMin:      8.0,
		QPMax:      48.0,
		MaxRounds:  10,
		MetricMode: "mean",
	}
}

// ParseTargetRange parses a target quality range string (e.g., "70-75").
// Returns a Config with TargetMin, TargetMax, Target, and Tolerance set.
func ParseTargetRange(s string) (*Config, error) {
	cfg := DefaultConfig()

	parts := strings.Split(s, "-")
	if len(parts) != 2 {
		return nil, fmt.Errorf("invalid target quality format %q, expected 'min-max' (e.g., '70-75')", s)
	}

	minVal, err := strconv.ParseFloat(strings.TrimSpace(parts[0]), 64)
	if err != nil {
		return nil, fmt.Errorf("invalid target quality min %q: %w", parts[0], err)
	}

	maxVal, err := strconv.ParseFloat(strings.TrimSpace(parts[1]), 64)
	if err != nil {
		return nil, fmt.Errorf("invalid target quality max %q: %w", parts[1], err)
	}

	if minVal >= maxVal {
		return nil, fmt.Errorf("target quality min (%v) must be less than max (%v)", minVal, maxVal)
	}

	cfg.TargetMin = minVal
	cfg.TargetMax = maxVal
	cfg.Target = (minVal + maxVal) / 2.0
	cfg.Tolerance = (maxVal - minVal) / 2.0

	return cfg, nil
}

// ParseQPRange parses a CRF search range string (e.g., "8-48").
func ParseQPRange(s string) (min, max float64, err error) {
	parts := strings.Split(s, "-")
	if len(parts) != 2 {
		return 0, 0, fmt.Errorf("invalid QP range format %q, expected 'min-max' (e.g., '8-48')", s)
	}

	min, err = strconv.ParseFloat(strings.TrimSpace(parts[0]), 64)
	if err != nil {
		return 0, 0, fmt.Errorf("invalid QP range min %q: %w", parts[0], err)
	}

	max, err = strconv.ParseFloat(strings.TrimSpace(parts[1]), 64)
	if err != nil {
		return 0, 0, fmt.Errorf("invalid QP range max %q: %w", parts[1], err)
	}

	if min >= max {
		return 0, 0, fmt.Errorf("QP range min (%v) must be less than max (%v)", min, max)
	}

	return min, max, nil
}
