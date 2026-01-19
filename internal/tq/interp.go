package tq

import (
	"math"
	"sort"
)

// maxTau2 is the maximum allowed tau squared for monotonicity preservation in PCHIP.
const maxTau2 = 9.0

// hermiteInterp evaluates a cubic Hermite spline at xi given interval [xk, xk1],
// function values [yk, yk1], and derivatives [dk, dk1].
func hermiteInterp(xk, xk1, yk, yk1, dk, dk1, xi float64) float64 {
	h := xk1 - xk
	t := (xi - xk) / h
	t2 := t * t
	t3 := t2 * t

	h00 := 2*t3 - 3*t2 + 1
	h10 := t3 - 2*t2 + t
	h01 := -2*t3 + 3*t2
	h11 := t3 - t2

	return h00*yk + h10*h*dk + h01*yk1 + h11*h*dk1
}

// Lerp performs linear interpolation between two points.
// x[0], y[0] is the first point, x[1], y[1] is the second point.
// Returns nil if interpolation is not possible.
func Lerp(x, y [2]float64, xi float64) *float64 {
	if x[1] <= x[0] {
		return nil
	}

	t := (xi - x[0]) / (x[1] - x[0])
	result := t*(y[1]-y[0]) + y[0]
	return &result
}

// PCHIP performs Piecewise Cubic Hermite Interpolating Polynomial interpolation.
// Requires exactly 4 points. Returns nil if interpolation is not possible.
func PCHIP(x, y [4]float64, xi float64) *float64 {
	// Verify strictly increasing x values
	for i := range 3 {
		if x[i+1] <= x[i] {
			return nil
		}
	}

	// Find the interval containing xi
	k := 0
	for i := range 3 {
		if xi >= x[i] && xi <= x[i+1] {
			k = i
			break
		}
	}

	// Compute slopes
	s0 := (y[1] - y[0]) / (x[1] - x[0])
	s1 := (y[2] - y[1]) / (x[2] - x[1])
	s2 := (y[3] - y[2]) / (x[3] - x[2])
	slopes := [3]float64{s0, s1, s2}

	// Compute derivatives
	d := [4]float64{s0, 0, 0, s2}

	// Interior points
	params := [2][4]float64{
		{s0, s1, x[1] - x[0], x[2] - x[1]},
		{s1, s2, x[2] - x[1], x[3] - x[2]},
	}

	for i := range 2 {
		sPrev, sNext := params[i][0], params[i][1]
		hPrev, hNext := params[i][2], params[i][3]
		idx := i + 1

		if sPrev*sNext <= 0 {
			d[idx] = 0
		} else {
			w1 := 2*hNext + hPrev
			w2 := 2*hPrev + hNext
			d[idx] = (w1 + w2) / (w1/sPrev + w2/sNext)
		}
	}

	// Apply monotonicity constraints
	for i := range 3 {
		if slopes[i] == 0 {
			d[i] = 0
			d[i+1] = 0
		} else {
			alpha := d[i] / slopes[i]
			beta := d[i+1] / slopes[i]
			tau := alpha*alpha + beta*beta

			if tau > maxTau2 {
				scale := 3.0 / math.Sqrt(tau)
				d[i] = scale * alpha * slopes[i]
				d[i+1] = scale * beta * slopes[i]
			}
		}
	}

	result := hermiteInterp(x[k], x[k+1], y[k], y[k+1], d[k], d[k+1], xi)
	return &result
}

// Akima performs Akima spline interpolation.
// Requires at least 5 points. Returns nil if interpolation is not possible.
func Akima(x, y []float64, xi float64) *float64 {
	n := len(x)
	if n < 5 || len(y) != n {
		return nil
	}

	// Verify strictly increasing x values
	for i := 0; i < n-1; i++ {
		if x[i+1] <= x[i] {
			return nil
		}
	}

	// Check bounds
	if xi < x[0] || xi > x[n-1] {
		return nil
	}

	// Find interval (searching from right for efficiency)
	k := 0
	for i := n - 2; i >= 0; i-- {
		if xi >= x[i] {
			k = i
			break
		}
	}

	// Compute slopes (m[1] to m[n-1] are the actual segment slopes)
	m := make([]float64, n+1)
	for i := 0; i < n-1; i++ {
		m[i+1] = (y[i+1] - y[i]) / (x[i+1] - x[i])
	}

	// Extend slopes at boundaries
	m[0] = 2*m[1] - m[2]
	m[n] = 2*m[n-1] - m[n-2]

	// Compute tangents
	tan := make([]float64, n)
	for i := 0; i < n-1; i++ {
		w1 := math.Abs(m[i+2] - m[i+1])
		w2 := math.Abs(m[i] - m[i+1])

		if w1+w2 < 1e-10 {
			tan[i] = 0.5 * (m[i] + m[i+1])
		} else {
			tan[i] = (w1*m[i] + w2*m[i+1]) / (w1 + w2)
		}
	}
	tan[n-1] = m[n-1]

	result := hermiteInterp(x[k], x[k+1], y[k], y[k+1], tan[k], tan[k+1], xi)
	return &result
}

// FritschCarlson performs Fritsch-Carlson monotonic spline interpolation.
// Requires exactly 3 points. Returns nil if interpolation is not possible.
func FritschCarlson(x, y []float64, xi float64) *float64 {
	n := len(x)
	if n != 3 || xi < x[0] || xi > x[n-1] {
		return nil
	}

	// Find interval
	k := 0
	for i := range 2 {
		if xi >= x[i] && xi <= x[i+1] {
			k = i
			break
		}
	}

	// Compute segment slopes
	d0 := (y[1] - y[0]) / (x[1] - x[0])
	d1 := (y[2] - y[1]) / (x[2] - x[1])

	// Compute tangents
	m := [3]float64{d0, 0, d1}

	if d0*d1 <= 0 {
		m[1] = 0
	} else {
		h0 := x[1] - x[0]
		h1 := x[2] - x[1]
		w1 := 2*h1 + h0
		w2 := 2*h0 + h1
		m[1] = (w1 + w2) / (w1/d0 + w2/d1)
	}

	result := hermiteInterp(x[k], x[k+1], y[k], y[k+1], m[k], m[k+1], xi)
	return &result
}

// InterpolateCRF uses the appropriate interpolation method based on the round number.
// - Rounds 1-2: Returns nil (binary search should be used)
// - Round 3: Linear interpolation (requires 2 probes)
// - Round 4: Fritsch-Carlson (requires 3 probes)
// - Round 5: PCHIP (requires 4 probes)
// - Round 6+: Akima (requires 5+ probes)
// The result is rounded to the nearest integer.
func InterpolateCRF(probes []Probe, target float64, round int) *float64 {
	if round <= 2 {
		return nil
	}

	// Sort probes by score for interpolation
	sorted := make([]Probe, len(probes))
	copy(sorted, probes)
	sort.Slice(sorted, func(i, j int) bool {
		return sorted[i].Score < sorted[j].Score
	})

	// Extract x (scores) and y (CRFs)
	n := len(sorted)
	x := make([]float64, n)
	y := make([]float64, n)
	for i, p := range sorted {
		x[i] = p.Score
		y[i] = p.CRF
	}

	var result *float64

	switch round {
	case 3:
		if n >= 2 {
			result = Lerp([2]float64{x[0], x[1]}, [2]float64{y[0], y[1]}, target)
		}
	case 4:
		if n >= 3 {
			result = FritschCarlson(x[:3], y[:3], target)
		}
	case 5:
		if n >= 4 {
			result = PCHIP([4]float64{x[0], x[1], x[2], x[3]}, [4]float64{y[0], y[1], y[2], y[3]}, target)
		}
	default:
		if n >= 5 {
			result = Akima(x, y, target)
		}
	}

	if result == nil {
		return nil
	}

	// Round to nearest integer
	rounded := roundCRF(*result)
	return &rounded
}

// roundCRF rounds a CRF value to the nearest integer.
func roundCRF(crf float64) float64 {
	return math.Round(crf)
}
