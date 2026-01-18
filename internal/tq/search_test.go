package tq

import (
	"testing"
)

func TestBinarySearch(t *testing.T) {
	tests := []struct {
		min      float64
		max      float64
		expected float64
	}{
		{8, 48, 28},
		{20, 30, 25},
		{20, 25, 22.5},
		{20, 22.5, 21.25},
	}

	for _, tt := range tests {
		result := BinarySearch(tt.min, tt.max)
		if result != tt.expected {
			t.Errorf("BinarySearch(%v, %v) = %v, want %v", tt.min, tt.max, result, tt.expected)
		}
	}
}

func TestConverged(t *testing.T) {
	tests := []struct {
		score     float64
		target    float64
		tolerance float64
		expected  bool
	}{
		{70, 70, 2.5, true},   // Exact match
		{72.5, 70, 2.5, true}, // At upper bound
		{67.5, 70, 2.5, true}, // At lower bound
		{71, 70, 2.5, true},   // Within range
		{73, 70, 2.5, false},  // Above range
		{66, 70, 2.5, false},  // Below range
	}

	for _, tt := range tests {
		result := Converged(tt.score, tt.target, tt.tolerance)
		if result != tt.expected {
			t.Errorf("Converged(%v, %v, %v) = %v, want %v",
				tt.score, tt.target, tt.tolerance, result, tt.expected)
		}
	}
}

func TestUpdateBounds(t *testing.T) {
	tests := []struct {
		name           string
		initialMin     float64
		initialMax     float64
		lastCRF        float64
		score          float64
		target         float64
		tolerance      float64
		expectedMin    float64
		expectedMax    float64
		expectedCross  bool
	}{
		{
			name:          "score too low - decrease CRF",
			initialMin:    8,
			initialMax:    48,
			lastCRF:       28,
			score:         65,
			target:        72.5,
			tolerance:     2.5,
			expectedMin:   8,
			expectedMax:   27.75,
			expectedCross: false,
		},
		{
			name:          "score too high - increase CRF",
			initialMin:    8,
			initialMax:    48,
			lastCRF:       28,
			score:         80,
			target:        72.5,
			tolerance:     2.5,
			expectedMin:   28.25,
			expectedMax:   48,
			expectedCross: false,
		},
		{
			name:          "score in range - no change",
			initialMin:    8,
			initialMax:    48,
			lastCRF:       28,
			score:         73,
			target:        72.5,
			tolerance:     2.5,
			expectedMin:   8,
			expectedMax:   48,
			expectedCross: false,
		},
		{
			name:          "bounds crossed",
			initialMin:    28,
			initialMax:    28.25,
			lastCRF:       28.25,
			score:         65,
			target:        72.5,
			tolerance:     2.5,
			expectedMin:   28,
			expectedMax:   28, // 28.25 - 0.25 = 28, so min (28) is not > max (28), still valid
			expectedCross: false,
		},
		{
			name:          "bounds truly crossed",
			initialMin:    28.25,
			initialMax:    28.25,
			lastCRF:       28.25,
			score:         65,
			target:        72.5,
			tolerance:     2.5,
			expectedMin:   28.25,
			expectedMax:   28, // 28.25 - 0.25 = 28, so min (28.25) > max (28)
			expectedCross: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			state := &State{
				SearchMin: tt.initialMin,
				SearchMax: tt.initialMax,
				LastCRF:   tt.lastCRF,
			}

			crossed := UpdateBounds(state, tt.score, tt.target, tt.tolerance)

			if crossed != tt.expectedCross {
				t.Errorf("UpdateBounds() crossed = %v, want %v", crossed, tt.expectedCross)
			}
			if state.SearchMin != tt.expectedMin {
				t.Errorf("UpdateBounds() SearchMin = %v, want %v", state.SearchMin, tt.expectedMin)
			}
			if state.SearchMax != tt.expectedMax {
				t.Errorf("UpdateBounds() SearchMax = %v, want %v", state.SearchMax, tt.expectedMax)
			}
		})
	}
}

func TestNextCRF(t *testing.T) {
	// Test binary search for first two rounds
	state := NewState(72.5, 8, 48)

	crf1 := NextCRF(state)
	if crf1 != 28 {
		t.Errorf("NextCRF() round 1 = %v, want 28", crf1)
	}
	if state.Round != 1 {
		t.Errorf("State round = %v, want 1", state.Round)
	}

	// Simulate a probe result
	state.AddProbe(28, 65, nil, 1000000)
	UpdateBounds(state, 65, 72.5, 2.5)

	crf2 := NextCRF(state)
	if state.Round != 2 {
		t.Errorf("State round = %v, want 2", state.Round)
	}
	// Should still be binary search
	if crf2 == 28 {
		t.Errorf("NextCRF() round 2 should not return same CRF as round 1")
	}
}

func TestShouldComplete(t *testing.T) {
	cfg := &Config{
		Target:    72.5,
		Tolerance: 2.5,
		MaxRounds: 10,
	}

	tests := []struct {
		name     string
		round    int
		score    float64
		expected bool
	}{
		{
			name:     "converged",
			round:    3,
			score:    72,
			expected: true,
		},
		{
			name:     "max rounds reached",
			round:    10,
			score:    65,
			expected: true,
		},
		{
			name:     "not converged, more rounds available",
			round:    3,
			score:    65,
			expected: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			state := NewState(72.5, 8, 48)
			state.Round = tt.round
			state.LastCRF = 28

			result := ShouldComplete(state, tt.score, cfg)
			if result != tt.expected {
				t.Errorf("ShouldComplete() = %v, want %v", result, tt.expected)
			}
		})
	}
}

func TestStateAddProbe(t *testing.T) {
	state := NewState(72.5, 8, 48)

	state.AddProbe(28, 65, []float64{64, 65, 66}, 1000000)
	state.AddProbe(22, 75, []float64{74, 75, 76}, 800000)

	if len(state.Probes) != 2 {
		t.Errorf("State has %d probes, want 2", len(state.Probes))
	}

	if state.Probes[0].CRF != 28 {
		t.Errorf("First probe CRF = %v, want 28", state.Probes[0].CRF)
	}

	if state.Probes[1].Score != 75 {
		t.Errorf("Second probe score = %v, want 75", state.Probes[1].Score)
	}
}

func TestStateBestProbe(t *testing.T) {
	state := NewState(72.5, 8, 48)

	// No probes
	best := state.BestProbe()
	if best != nil {
		t.Errorf("BestProbe() with no probes = %v, want nil", best)
	}

	// Add probes
	state.AddProbe(35, 65, nil, 1200000)
	state.AddProbe(28, 72, nil, 1000000)  // Closest to target 72.5
	state.AddProbe(22, 78, nil, 800000)

	best = state.BestProbe()
	if best == nil {
		t.Fatal("BestProbe() = nil, want non-nil")
	}

	if best.CRF != 28 {
		t.Errorf("BestProbe().CRF = %v, want 28 (closest to target 72.5)", best.CRF)
	}
}
