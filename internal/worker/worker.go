// Package worker provides types and utilities for parallel chunk encoding.
package worker

// Semaphore provides a counting semaphore for controlling concurrency.
// It is used to limit the number of chunks in flight to prevent memory exhaustion.
type Semaphore struct {
	permits chan struct{}
}

// NewSemaphore creates a new semaphore with the given number of permits.
func NewSemaphore(count int) *Semaphore {
	if count <= 0 {
		count = 1
	}
	s := &Semaphore{
		permits: make(chan struct{}, count),
	}
	// Pre-fill the permits
	for i := 0; i < count; i++ {
		s.permits <- struct{}{}
	}
	return s
}

// Release returns a permit to the semaphore.
func (s *Semaphore) Release() {
	select {
	case s.permits <- struct{}{}:
	default:
		// Semaphore is full, this shouldn't happen in normal use
	}
}

// Chan returns the underlying permit channel for use with select.
// This allows context-aware acquisition of permits.
func (s *Semaphore) Chan() <-chan struct{} {
	return s.permits
}

// EncodeResult contains the result of encoding a single chunk.
type EncodeResult struct {
	ChunkIdx int
	Frames   int
	Size     uint64
	Error    error
}

// Progress represents encoding progress information.
type Progress struct {
	ChunksComplete int
	ChunksTotal    int
	FramesComplete int
	FramesTotal    int
	BytesComplete  uint64
}

// Percent returns the completion percentage.
func (p Progress) Percent() float64 {
	if p.FramesTotal == 0 {
		return 0
	}
	return float64(p.FramesComplete) / float64(p.FramesTotal) * 100
}
