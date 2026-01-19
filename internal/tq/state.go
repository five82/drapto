package tq

import "math"

// Probe represents a single encoding attempt at a specific CRF value.
type Probe struct {
	// CRF is the quality parameter used for this probe.
	CRF float64

	// Score is the computed SSIMULACRA2 score for this probe.
	Score float64

	// FrameScores contains per-frame SSIMULACRA2 scores.
	FrameScores []float64

	// Size is the output file size in bytes.
	Size uint64
}

// State tracks the iterative CRF search state for a single chunk.
type State struct {
	// Probes contains all completed encoding attempts.
	Probes []Probe

	// SearchMin and SearchMax define the current CRF search bounds.
	SearchMin float64
	SearchMax float64

	// QPMin and QPMax are the original (hard) CRF bounds that cannot be exceeded.
	QPMin float64
	QPMax float64

	// Round is the current iteration number (1-indexed).
	Round int

	// Target is the desired SSIMULACRA2 score.
	Target float64

	// LastCRF is the CRF value used in the most recent probe.
	LastCRF float64
}

// NewState creates a new TQ state for a chunk.
// If predictedCRF > 0, the search bounds are narrowed to [predicted-5, predicted+5]
// clamped to [qpMin, qpMax]. Otherwise, the full range is used.
func NewState(target, qpMin, qpMax, predictedCRF float64) *State {
	searchMin := qpMin
	searchMax := qpMax

	if predictedCRF > 0 {
		// Narrow bounds around predicted CRF
		searchMin = max(qpMin, predictedCRF-5)
		searchMax = min(qpMax, predictedCRF+5)
	}

	return &State{
		Probes:    make([]Probe, 0, 8),
		SearchMin: searchMin,
		SearchMax: searchMax,
		QPMin:     qpMin,
		QPMax:     qpMax,
		Round:     0,
		Target:    target,
	}
}

// AddProbe records a completed probe result.
func (s *State) AddProbe(crf, score float64, frameScores []float64, size uint64) {
	s.Probes = append(s.Probes, Probe{
		CRF:         crf,
		Score:       score,
		FrameScores: frameScores,
		Size:        size,
	})
}

// BestProbe returns the probe closest to the target score.
func (s *State) BestProbe() *Probe {
	if len(s.Probes) == 0 {
		return nil
	}

	best := &s.Probes[0]
	bestDiff := math.Abs(best.Score - s.Target)

	for i := 1; i < len(s.Probes); i++ {
		diff := math.Abs(s.Probes[i].Score - s.Target)
		if diff < bestDiff {
			best = &s.Probes[i]
			bestDiff = diff
		}
	}

	return best
}

// ProbeEntry represents a single probe result for logging.
type ProbeEntry struct {
	CRF   float64
	Score float64
	Size  uint64
}
