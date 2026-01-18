package tq

import (
	"fmt"
	"os"
	"sync"
)

var debugTQ = os.Getenv("DRAPTO_DEBUG_TQ") == "1"

// CRFTracker maintains completed chunk CRF values and provides predictions
// for new chunks based on nearby completed chunks.
type CRFTracker struct {
	mu      sync.RWMutex
	results map[int]float64 // chunkIdx → final CRF
}

// NewTracker creates a new CRF tracker.
func NewTracker() *CRFTracker {
	return &CRFTracker{
		results: make(map[int]float64),
	}
}

// Record stores the final CRF value for a completed chunk.
func (t *CRFTracker) Record(chunkIdx int, crf float64) {
	t.mu.Lock()
	defer t.mu.Unlock()
	t.results[chunkIdx] = crf
}

// Predict returns a predicted CRF for the given chunk index.
// It uses a weighted average of up to 4 nearest completed chunks,
// weighted by 1/distance. Returns defaultCRF if no completed chunks exist.
func (t *CRFTracker) Predict(chunkIdx int, defaultCRF float64) float64 {
	t.mu.RLock()
	defer t.mu.RUnlock()

	if len(t.results) == 0 {
		return defaultCRF
	}

	// Find the 4 nearest completed chunks
	type neighbor struct {
		idx  int
		dist int
		crf  float64
	}

	neighbors := make([]neighbor, 0, len(t.results))
	for idx, crf := range t.results {
		dist := chunkIdx - idx
		if dist < 0 {
			dist = -dist
		}
		neighbors = append(neighbors, neighbor{idx, dist, crf})
	}

	// Sort by distance (simple insertion sort for small slices)
	for i := 1; i < len(neighbors); i++ {
		for j := i; j > 0 && neighbors[j].dist < neighbors[j-1].dist; j-- {
			neighbors[j], neighbors[j-1] = neighbors[j-1], neighbors[j]
		}
	}

	// Use up to 4 nearest neighbors
	neighbors = neighbors[:min(4, len(neighbors))]

	// Compute weighted average (weight = 1/distance)
	var weightedSum, weightSum float64
	for _, n := range neighbors {
		if n.dist == 0 {
			// Exact match - return the CRF directly
			if debugTQ {
				fmt.Printf("[TQ-DEBUG]   -> exact match at chunk %d, CRF=%.1f\n", n.idx, n.crf)
			}
			return n.crf
		}
		weight := 1.0 / float64(n.dist)
		weightedSum += n.crf * weight
		weightSum += weight
		if debugTQ {
			fmt.Printf("[TQ-DEBUG]   -> neighbor chunk %d: CRF=%.1f, dist=%d, weight=%.3f\n",
				n.idx, n.crf, n.dist, weight)
		}
	}

	if weightSum == 0 {
		return defaultCRF
	}

	result := weightedSum / weightSum
	if debugTQ {
		fmt.Printf("[TQ-DEBUG]   -> weighted avg=%.1f\n", result)
	}
	return result
}

// Count returns the number of recorded results.
func (t *CRFTracker) Count() int {
	t.mu.RLock()
	defer t.mu.RUnlock()
	return len(t.results)
}
