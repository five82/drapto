// Package worker provides types and utilities for parallel chunk encoding.
package worker

import (
	"github.com/five82/drapto/internal/chunk"
	"github.com/five82/drapto/internal/tq"
)

// WorkPkg represents a work package containing decoded frames ready for encoding.
type WorkPkg struct {
	Chunk      chunk.Chunk // The chunk being encoded
	YUV        []byte      // Raw YUV frame data for all frames in the chunk
	FrameCount int         // Number of frames in this package
	Width      uint32      // Frame width (after cropping)
	Height     uint32      // Frame height (after cropping)
	Is10Bit    bool        // Whether frames are 10-bit
	TQState    *tq.State   // Target quality search state (nil when TQ disabled)
}

// FrameSize returns the size of a single frame in bytes.
func (w *WorkPkg) FrameSize() int {
	if w.Is10Bit {
		// YUV420P10LE: Y = w*h*2, U = w*h/2, V = w*h/2
		return int(w.Width) * int(w.Height) * 3
	}
	// YUV420P: Y = w*h, U = w*h/4, V = w*h/4
	return int(w.Width) * int(w.Height) * 3 / 2
}

// TotalSize returns the total size of all frames in bytes.
func (w *WorkPkg) TotalSize() int {
	return w.FrameSize() * w.FrameCount
}

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

// Acquire blocks until a permit is available, then takes it.
func (s *Semaphore) Acquire() {
	<-s.permits
}

// TryAcquire attempts to acquire a permit without blocking.
// Returns true if successful, false if no permits available.
func (s *Semaphore) TryAcquire() bool {
	select {
	case <-s.permits:
		return true
	default:
		return false
	}
}

// Release returns a permit to the semaphore.
func (s *Semaphore) Release() {
	select {
	case s.permits <- struct{}{}:
	default:
		// Semaphore is full, this shouldn't happen in normal use
	}
}

// Available returns the number of available permits.
func (s *Semaphore) Available() int {
	return len(s.permits)
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
