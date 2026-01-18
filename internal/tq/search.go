package tq

import "math"

// BinarySearch returns the midpoint between min and max, rounded to the nearest integer.
func BinarySearch(min, max float64) float64 {
	mid := (min + max) / 2
	return roundCRF(mid)
}

// NextCRF determines the next CRF value to try based on the current state.
// For rounds 1-2, uses binary search.
// For round 3+, uses interpolation from collected probes.
func NextCRF(state *State) float64 {
	state.Round++

	var crf float64

	if state.Round <= 2 {
		crf = BinarySearch(state.SearchMin, state.SearchMax)
	} else {
		interpolated := InterpolateCRF(state.Probes, state.Target, state.Round)
		if interpolated != nil {
			crf = *interpolated
		} else {
			crf = BinarySearch(state.SearchMin, state.SearchMax)
		}
	}

	// Clamp to search bounds
	crf = clamp(crf, state.SearchMin, state.SearchMax)
	state.LastCRF = crf

	return crf
}

// Converged checks if the score is within tolerance of the target.
func Converged(score, target, tolerance float64) bool {
	return math.Abs(score-target) <= tolerance
}

// UpdateBounds updates the search bounds based on the score result.
// For SSIMULACRA2:
// - If score is too low (quality too low), decrease CRF (search lower)
// - If score is too high (quality too high), increase CRF (search higher)
// Returns true if bounds have crossed (no valid CRF in remaining range).
func UpdateBounds(state *State, score, target, tolerance float64) bool {
	if score < target-tolerance {
		// Score too low, need lower CRF for higher quality
		state.SearchMax = state.LastCRF - 1.0
	} else if score > target+tolerance {
		// Score too high, need higher CRF for lower quality
		state.SearchMin = state.LastCRF + 1.0
	}

	// Return true if bounds have crossed
	return state.SearchMin > state.SearchMax
}

// ShouldComplete determines if the TQ search should complete.
// Returns true if any termination condition is met.
func ShouldComplete(state *State, score float64, cfg *Config) bool {
	// Check convergence
	if Converged(score, cfg.Target, cfg.Tolerance) {
		return true
	}

	// Check max rounds
	if state.Round >= cfg.MaxRounds {
		return true
	}

	// Check if bounds have crossed
	if UpdateBounds(state, score, cfg.Target, cfg.Tolerance) {
		return true
	}

	return false
}

// clamp restricts a value to the range [min, max].
func clamp(v, min, max float64) float64 {
	if v < min {
		return min
	}
	if v > max {
		return max
	}
	return v
}
