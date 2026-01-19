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

	// Sample-based TQ probing fields
	SampleYUV         []byte // Encode window (sample + warmup buffer) YUV data
	SampleFrameCount  int    // Frame count to encode (includes warmup)
	SampleOffset      int    // Frame offset where sample starts in full chunk
	WarmupFrames      int    // Frames to skip when measuring (0.5s worth)
	MeasureFrameCount int    // Frames to actually measure (excludes warmup)
	UseSampling       bool   // Whether sampling is enabled for this chunk

	// Stats tracking fields
	PredictedCRF float64 // Predicted CRF for stats computation
}

// WarmupDuration is the fixed warmup period in seconds.
// This time is encoded but not measured to avoid encoder warmup artifacts.
const WarmupDuration = 0.5

// CalculateSample computes sample parameters for TQ probing.
// Returns offset (frame index where sample starts in full chunk),
// encodeFrames (frames to encode, including warmup), warmupFrames (frames to skip),
// and measureFrames (frames to actually measure).
func CalculateSample(totalFrames int, fps, sampleDur, minChunkDur float64) (offset, encodeFrames, warmupFrames, measureFrames int, useSampling bool) {
	// Calculate chunk duration
	chunkDur := float64(totalFrames) / fps

	// If chunk is too short, don't use sampling
	if chunkDur < minChunkDur {
		return 0, totalFrames, 0, totalFrames, false
	}

	// Calculate warmup and sample frames
	warmupFrames = int(WarmupDuration * fps)
	measureFrames = int(sampleDur * fps)
	encodeFrames = warmupFrames + measureFrames

	// If encode window is larger than half the chunk, don't use sampling
	if encodeFrames > totalFrames/2 {
		return 0, totalFrames, 0, totalFrames, false
	}

	// Center the sample in the middle of the chunk
	// Avoid first few frames (keyframe overhead) and last few frames
	middleFrame := totalFrames / 2
	offset = middleFrame - encodeFrames/2

	// Clamp offset to valid range
	if offset < 0 {
		offset = 0
	}
	if offset+encodeFrames > totalFrames {
		offset = totalFrames - encodeFrames
	}

	return offset, encodeFrames, warmupFrames, measureFrames, true
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
