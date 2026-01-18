package tq

import (
	"math"
	"testing"
)

const epsilon = 1e-6

func almostEqual(a, b, eps float64) bool {
	return math.Abs(a-b) < eps
}

func TestLerp(t *testing.T) {
	tests := []struct {
		name     string
		x        [2]float64
		y        [2]float64
		xi       float64
		expected float64
		wantNil  bool
	}{
		{
			name:     "midpoint",
			x:        [2]float64{0, 10},
			y:        [2]float64{0, 100},
			xi:       5,
			expected: 50,
		},
		{
			name:     "at start",
			x:        [2]float64{0, 10},
			y:        [2]float64{20, 40},
			xi:       0,
			expected: 20,
		},
		{
			name:     "at end",
			x:        [2]float64{0, 10},
			y:        [2]float64{20, 40},
			xi:       10,
			expected: 40,
		},
		{
			name:     "quarter point",
			x:        [2]float64{0, 10},
			y:        [2]float64{0, 100},
			xi:       2.5,
			expected: 25,
		},
		{
			name:    "invalid - x1 <= x0",
			x:       [2]float64{10, 10},
			y:       [2]float64{0, 100},
			xi:      5,
			wantNil: true,
		},
		{
			name:    "invalid - x1 < x0",
			x:       [2]float64{10, 5},
			y:       [2]float64{0, 100},
			xi:      5,
			wantNil: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := Lerp(tt.x, tt.y, tt.xi)
			if tt.wantNil {
				if result != nil {
					t.Errorf("Lerp() = %v, want nil", *result)
				}
				return
			}
			if result == nil {
				t.Errorf("Lerp() = nil, want %v", tt.expected)
				return
			}
			if !almostEqual(*result, tt.expected, epsilon) {
				t.Errorf("Lerp() = %v, want %v", *result, tt.expected)
			}
		})
	}
}

func TestFritschCarlson(t *testing.T) {
	// Test with 3 points
	x := []float64{60, 70, 80}
	y := []float64{35, 28, 22}

	// At a known point
	result := FritschCarlson(x, y, 70)
	if result == nil {
		t.Fatal("FritschCarlson() returned nil for valid input")
	}
	if !almostEqual(*result, 28, 0.1) {
		t.Errorf("FritschCarlson() at x=70 = %v, want ~28", *result)
	}

	// Out of bounds
	result = FritschCarlson(x, y, 50)
	if result != nil {
		t.Errorf("FritschCarlson() outside range = %v, want nil", *result)
	}

	// Wrong number of points
	result = FritschCarlson([]float64{60, 70}, []float64{35, 28}, 65)
	if result != nil {
		t.Errorf("FritschCarlson() with 2 points = %v, want nil", *result)
	}
}

func TestPCHIP(t *testing.T) {
	// Test with 4 points
	x := [4]float64{60, 65, 70, 75}
	y := [4]float64{40, 35, 28, 22}

	// At a known point
	result := PCHIP(x, y, 65)
	if result == nil {
		t.Fatal("PCHIP() returned nil for valid input")
	}
	if !almostEqual(*result, 35, 0.1) {
		t.Errorf("PCHIP() at x=65 = %v, want ~35", *result)
	}

	// Mid-interval
	result = PCHIP(x, y, 67.5)
	if result == nil {
		t.Fatal("PCHIP() returned nil for valid input")
	}
	// Should be between 35 and 28
	if *result < 28 || *result > 35 {
		t.Errorf("PCHIP() at x=67.5 = %v, want value between 28 and 35", *result)
	}

	// Non-increasing x should fail
	badX := [4]float64{60, 65, 65, 75}
	result = PCHIP(badX, y, 67.5)
	if result != nil {
		t.Errorf("PCHIP() with non-increasing x = %v, want nil", *result)
	}
}

func TestAkima(t *testing.T) {
	// Test with 5 points (minimum for Akima)
	x := []float64{55, 60, 65, 70, 75}
	y := []float64{45, 40, 35, 28, 22}

	// At a known point
	result := Akima(x, y, 65)
	if result == nil {
		t.Fatal("Akima() returned nil for valid input")
	}
	if !almostEqual(*result, 35, 0.1) {
		t.Errorf("Akima() at x=65 = %v, want ~35", *result)
	}

	// Mid-interval
	result = Akima(x, y, 67.5)
	if result == nil {
		t.Fatal("Akima() returned nil for valid input")
	}

	// Out of bounds
	result = Akima(x, y, 50)
	if result != nil {
		t.Errorf("Akima() below range = %v, want nil", *result)
	}

	result = Akima(x, y, 80)
	if result != nil {
		t.Errorf("Akima() above range = %v, want nil", *result)
	}

	// Too few points
	result = Akima([]float64{60, 65, 70, 75}, []float64{40, 35, 28, 22}, 67.5)
	if result != nil {
		t.Errorf("Akima() with 4 points = %v, want nil", *result)
	}
}

func TestRoundCRF(t *testing.T) {
	tests := []struct {
		input    float64
		expected float64
	}{
		{28.0, 28.0},
		{28.1, 28.0},
		{28.4, 28.0},
		{28.5, 29.0}, // standard rounding
		{28.6, 29.0},
		{28.9, 29.0},
	}

	for _, tt := range tests {
		result := roundCRF(tt.input)
		if !almostEqual(result, tt.expected, epsilon) {
			t.Errorf("roundCRF(%v) = %v, want %v", tt.input, result, tt.expected)
		}
	}
}

func TestInterpolateCRF(t *testing.T) {
	probes := []Probe{
		{CRF: 35, Score: 65},
		{CRF: 28, Score: 72},
		{CRF: 22, Score: 78},
		{CRF: 18, Score: 82},
		{CRF: 15, Score: 86},
	}

	// Round 1-2 should return nil (binary search)
	result := InterpolateCRF(probes[:2], 70, 1)
	if result != nil {
		t.Errorf("InterpolateCRF(round=1) = %v, want nil", *result)
	}

	result = InterpolateCRF(probes[:2], 70, 2)
	if result != nil {
		t.Errorf("InterpolateCRF(round=2) = %v, want nil", *result)
	}

	// Round 3 uses lerp
	result = InterpolateCRF(probes[:2], 70, 3)
	if result == nil {
		t.Fatal("InterpolateCRF(round=3) returned nil")
	}
	// Target 70 is between scores 65 and 72, so CRF should be between 35 and 28
	if *result < 28 || *result > 35 {
		t.Errorf("InterpolateCRF(round=3) = %v, want value between 28 and 35", *result)
	}

	// Round 4 uses Fritsch-Carlson
	result = InterpolateCRF(probes[:3], 73, 4)
	if result == nil {
		t.Fatal("InterpolateCRF(round=4) returned nil")
	}

	// Round 5 uses PCHIP
	result = InterpolateCRF(probes[:4], 75, 5)
	if result == nil {
		t.Fatal("InterpolateCRF(round=5) returned nil")
	}

	// Round 6+ uses Akima
	result = InterpolateCRF(probes, 80, 6)
	if result == nil {
		t.Fatal("InterpolateCRF(round=6) returned nil")
	}
}
