package tq

import (
	"math"
	"sync"
	"testing"
)

func TestTracker_Empty(t *testing.T) {
	tr := NewTracker()

	// Should return default when no data
	crf := tr.Predict(5, 28.0)
	if crf != 28.0 {
		t.Errorf("Predict() = %v, want 28.0 (default)", crf)
	}

	if c := tr.Count(); c != 0 {
		t.Errorf("Count() = %d, want 0", c)
	}
}

func TestTracker_SingleResult(t *testing.T) {
	tr := NewTracker()
	tr.Record(5, 25.0)

	// Predict for adjacent chunk should return the recorded value
	crf := tr.Predict(6, 28.0)
	if crf != 25.0 {
		t.Errorf("Predict(6) = %v, want 25.0", crf)
	}

	// Predict for same index should return exact value
	crf = tr.Predict(5, 28.0)
	if crf != 25.0 {
		t.Errorf("Predict(5) = %v, want 25.0 (exact match)", crf)
	}
}

func TestTracker_WeightedAverage(t *testing.T) {
	tr := NewTracker()
	tr.Record(0, 20.0) // distance 5 from chunk 5
	tr.Record(10, 30.0) // distance 5 from chunk 5

	// Equal distances should give equal weight
	crf := tr.Predict(5, 28.0)
	if crf != 25.0 {
		t.Errorf("Predict(5) = %v, want 25.0 (average of 20 and 30)", crf)
	}
}

func TestTracker_CloserNeighborHigherWeight(t *testing.T) {
	tr := NewTracker()
	tr.Record(4, 20.0) // distance 1
	tr.Record(10, 30.0) // distance 5

	// weight for 4: 1/1 = 1.0
	// weight for 10: 1/5 = 0.2
	// weighted avg: (20*1 + 30*0.2) / (1 + 0.2) = 26/1.2 = 21.67
	crf := tr.Predict(5, 28.0)
	expected := 26.0 / 1.2
	if math.Abs(crf-expected) > 0.01 {
		t.Errorf("Predict(5) = %v, want %v", crf, expected)
	}
}

func TestTracker_MaxFourNeighbors(t *testing.T) {
	tr := NewTracker()
	// Record 6 chunks
	tr.Record(0, 20.0)
	tr.Record(2, 22.0)
	tr.Record(4, 24.0)
	tr.Record(6, 26.0)
	tr.Record(8, 28.0)
	tr.Record(10, 30.0)

	// Predict for chunk 5 should use 4 nearest: 4, 6, 2, 8
	// distances: 4=1, 6=1, 2=3, 8=3
	// weights: 1, 1, 0.33, 0.33
	// Only 4 nearest used: indices 4, 6, 2, 8 (or 4, 6, 8, 2)
	crf := tr.Predict(5, 28.0)

	// Expected: using 4 nearest (4, 6, 2, 8)
	// weights: 1/1 + 1/1 + 1/3 + 1/3 = 2.67
	// sum: 24*1 + 26*1 + 22*(1/3) + 28*(1/3) = 24 + 26 + 7.33 + 9.33 = 66.67
	// avg: 66.67 / 2.67 = 25.0
	expected := (24.0 + 26.0 + 22.0/3.0 + 28.0/3.0) / (1.0 + 1.0 + 1.0/3.0 + 1.0/3.0)
	if math.Abs(crf-expected) > 0.01 {
		t.Errorf("Predict(5) = %v, want %v", crf, expected)
	}
}

func TestTracker_ExactMatch(t *testing.T) {
	tr := NewTracker()
	tr.Record(3, 22.0)
	tr.Record(5, 25.0)
	tr.Record(7, 28.0)

	// Predict for recorded index should return exact value
	crf := tr.Predict(5, 30.0)
	if crf != 25.0 {
		t.Errorf("Predict(5) = %v, want 25.0 (exact match)", crf)
	}
}

func TestTracker_Count(t *testing.T) {
	tr := NewTracker()

	tr.Record(1, 20.0)
	if c := tr.Count(); c != 1 {
		t.Errorf("Count() = %d, want 1", c)
	}

	tr.Record(2, 22.0)
	tr.Record(3, 24.0)
	if c := tr.Count(); c != 3 {
		t.Errorf("Count() = %d, want 3", c)
	}
}

func TestTracker_Concurrent(t *testing.T) {
	tr := NewTracker()

	var wg sync.WaitGroup
	// 10 writers
	for i := range 10 {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			tr.Record(idx, float64(20+idx))
		}(i)
	}

	// 10 readers
	for i := range 10 {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			_ = tr.Predict(idx, 28.0)
		}(i)
	}

	wg.Wait()

	if c := tr.Count(); c != 10 {
		t.Errorf("Count() = %d, want 10", c)
	}
}

func TestTracker_Overwrite(t *testing.T) {
	tr := NewTracker()
	tr.Record(5, 20.0)
	tr.Record(5, 30.0) // overwrite

	crf := tr.Predict(5, 28.0)
	if crf != 30.0 {
		t.Errorf("Predict(5) = %v, want 30.0 (overwritten value)", crf)
	}
}
