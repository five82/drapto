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
		{20, 25, 23},  // (20+25)/2 = 22.5 → 23
		{20, 23, 22},  // (20+23)/2 = 21.5 → 22
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
		qpMin          float64
		qpMax          float64
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
			qpMin:         8,
			qpMax:         48,
			lastCRF:       28,
			score:         65,
			target:        72.5,
			tolerance:     2.5,
			expectedMin:   8,
			expectedMax:   27, // 28 - 1.0
			expectedCross: false,
		},
		{
			name:          "score too high - increase CRF",
			initialMin:    8,
			initialMax:    48,
			qpMin:         8,
			qpMax:         48,
			lastCRF:       28,
			score:         80,
			target:        72.5,
			tolerance:     2.5,
			expectedMin:   29, // 28 + 1.0
			expectedMax:   48,
			expectedCross: false,
		},
		{
			name:          "score in range - no change",
			initialMin:    8,
			initialMax:    48,
			qpMin:         8,
			qpMax:         48,
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
			initialMax:    29,
			qpMin:         8,
			qpMax:         48,
			lastCRF:       29,
			score:         65,
			target:        72.5,
			tolerance:     2.5,
			expectedMin:   28,
			expectedMax:   28, // 29 - 1.0 = 28, so min (28) is not > max (28), still valid
			expectedCross: false,
		},
		{
			name:          "bounds crossed with expansion possible",
			initialMin:    29,
			initialMax:    29,
			qpMin:         8,
			qpMax:         48,
			lastCRF:       29,
			score:         65,
			target:        72.5,
			tolerance:     2.5,
			expectedMin:   24, // expanded: max(8, 29-5) = 24
			expectedMax:   28, // 29 - 1.0
			expectedCross: false,
		},
		{
			name:          "bounds crossed at hard limit - no expansion",
			initialMin:    9,
			initialMax:    9,
			qpMin:         8,
			qpMax:         48,
			lastCRF:       9,
			score:         65,
			target:        72.5,
			tolerance:     2.5,
			expectedMin:   8,  // expanded: max(8, 9-5) = 8
			expectedMax:   8,  // 9 - 1.0
			expectedCross: false,
		},
		{
			name:          "bounds crossed at absolute min - truly crossed",
			initialMin:    8,
			initialMax:    8,
			qpMin:         8,
			qpMax:         48,
			lastCRF:       8,
			score:         65,
			target:        72.5,
			tolerance:     2.5,
			expectedMin:   8,
			expectedMax:   7, // 8 - 1.0, can't expand further
			expectedCross: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			state := &State{
				SearchMin: tt.initialMin,
				SearchMax: tt.initialMax,
				QPMin:     tt.qpMin,
				QPMax:     tt.qpMax,
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

func TestUpdateBoundsExpansion(t *testing.T) {
	tests := []struct {
		name          string
		state         *State
		score         float64
		target        float64
		tolerance     float64
		expectedMin   float64
		expectedMax   float64
		expectedCross bool
	}{
		{
			name: "expand downward when score too low",
			state: &State{
				SearchMin: 25,
				SearchMax: 25,
				QPMin:     8,
				QPMax:     48,
				LastCRF:   25,
			},
			score:         65,
			target:        72.5,
			tolerance:     2.5,
			expectedMin:   20, // max(8, 25-5)
			expectedMax:   24, // 25 - 1
			expectedCross: false,
		},
		{
			name: "expand upward when score too high",
			state: &State{
				SearchMin: 25,
				SearchMax: 25,
				QPMin:     8,
				QPMax:     48,
				LastCRF:   25,
			},
			score:         80,
			target:        72.5,
			tolerance:     2.5,
			expectedMin:   26, // 25 + 1
			expectedMax:   30, // min(48, 25+5)
			expectedCross: false,
		},
		{
			name: "expansion limited by qpMax",
			state: &State{
				SearchMin: 46,
				SearchMax: 46,
				QPMin:     8,
				QPMax:     48,
				LastCRF:   46,
			},
			score:         80,
			target:        72.5,
			tolerance:     2.5,
			expectedMin:   47, // 46 + 1
			expectedMax:   48, // min(48, 46+5) = 48
			expectedCross: false,
		},
		{
			name: "expansion limited by qpMin",
			state: &State{
				SearchMin: 10,
				SearchMax: 10,
				QPMin:     8,
				QPMax:     48,
				LastCRF:   10,
			},
			score:         65,
			target:        72.5,
			tolerance:     2.5,
			expectedMin:   8,  // max(8, 10-5) = 8
			expectedMax:   9,  // 10 - 1
			expectedCross: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			crossed := UpdateBounds(tt.state, tt.score, tt.target, tt.tolerance)

			if crossed != tt.expectedCross {
				t.Errorf("crossed = %v, want %v", crossed, tt.expectedCross)
			}
			if tt.state.SearchMin != tt.expectedMin {
				t.Errorf("SearchMin = %v, want %v", tt.state.SearchMin, tt.expectedMin)
			}
			if tt.state.SearchMax != tt.expectedMax {
				t.Errorf("SearchMax = %v, want %v", tt.state.SearchMax, tt.expectedMax)
			}
		})
	}
}

func TestNextCRF(t *testing.T) {
	// Test binary search for first two rounds
	state := NewState(72.5, 8, 48, 0)

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
			state := NewState(72.5, 8, 48, 0)
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
	state := NewState(72.5, 8, 48, 0)

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
	state := NewState(72.5, 8, 48, 0)

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

func TestNewStateWithPredictedCRF(t *testing.T) {
	tests := []struct {
		name         string
		target       float64
		qpMin        float64
		qpMax        float64
		predictedCRF float64
		expectedMin  float64
		expectedMax  float64
	}{
		{
			name:         "no prediction - full range",
			target:       72.5,
			qpMin:        8,
			qpMax:        48,
			predictedCRF: 0,
			expectedMin:  8,
			expectedMax:  48,
		},
		{
			name:         "prediction in middle - narrow range",
			target:       72.5,
			qpMin:        8,
			qpMax:        48,
			predictedCRF: 25,
			expectedMin:  20, // 25 - 5
			expectedMax:  30, // 25 + 5
		},
		{
			name:         "prediction near min - clamp to qpMin",
			target:       72.5,
			qpMin:        8,
			qpMax:        48,
			predictedCRF: 10,
			expectedMin:  8,  // clamped to qpMin (10 - 5 = 5, but qpMin is 8)
			expectedMax:  15, // 10 + 5
		},
		{
			name:         "prediction near max - clamp to qpMax",
			target:       72.5,
			qpMin:        8,
			qpMax:        48,
			predictedCRF: 45,
			expectedMin:  40, // 45 - 5
			expectedMax:  48, // clamped to qpMax (45 + 5 = 50, but qpMax is 48)
		},
		{
			name:         "prediction at min boundary",
			target:       72.5,
			qpMin:        8,
			qpMax:        48,
			predictedCRF: 8,
			expectedMin:  8,  // clamped
			expectedMax:  13, // 8 + 5
		},
		{
			name:         "prediction at max boundary",
			target:       72.5,
			qpMin:        8,
			qpMax:        48,
			predictedCRF: 48,
			expectedMin:  43, // 48 - 5
			expectedMax:  48, // clamped
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			state := NewState(tt.target, tt.qpMin, tt.qpMax, tt.predictedCRF)

			if state.SearchMin != tt.expectedMin {
				t.Errorf("SearchMin = %v, want %v", state.SearchMin, tt.expectedMin)
			}
			if state.SearchMax != tt.expectedMax {
				t.Errorf("SearchMax = %v, want %v", state.SearchMax, tt.expectedMax)
			}
			if state.Target != tt.target {
				t.Errorf("Target = %v, want %v", state.Target, tt.target)
			}
		})
	}
}
