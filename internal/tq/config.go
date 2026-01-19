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

// parseRange parses a "min-max" range string and returns the values.
func parseRange(s, name, example string) (min, max float64, err error) {
	parts := strings.Split(s, "-")
	if len(parts) != 2 {
		return 0, 0, fmt.Errorf("invalid %s format %q, expected 'min-max' (e.g., '%s')", name, s, example)
	}

	min, err = strconv.ParseFloat(strings.TrimSpace(parts[0]), 64)
	if err != nil {
		return 0, 0, fmt.Errorf("invalid %s min %q: %w", name, parts[0], err)
	}

	max, err = strconv.ParseFloat(strings.TrimSpace(parts[1]), 64)
	if err != nil {
		return 0, 0, fmt.Errorf("invalid %s max %q: %w", name, parts[1], err)
	}

	if min >= max {
		return 0, 0, fmt.Errorf("%s min (%v) must be less than max (%v)", name, min, max)
	}

	return min, max, nil
}

// ParseTargetRange parses a target quality range string (e.g., "70-75").
// Returns a Config with TargetMin, TargetMax, Target, and Tolerance set.
func ParseTargetRange(s string) (*Config, error) {
	minVal, maxVal, err := parseRange(s, "target quality", "70-75")
	if err != nil {
		return nil, err
	}

	cfg := DefaultConfig()
	cfg.TargetMin = minVal
	cfg.TargetMax = maxVal
	cfg.Target = (minVal + maxVal) / 2.0
	cfg.Tolerance = (maxVal - minVal) / 2.0

	return cfg, nil
}

// ParseQPRange parses a CRF search range string (e.g., "8-48").
func ParseQPRange(s string) (min, max float64, err error) {
	return parseRange(s, "QP range", "8-48")
}
