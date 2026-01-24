package encode

import "github.com/five82/drapto/internal/util"

// CalculatePermits determines the number of in-flight chunk permits based on
// the requested base permits and available system memory.
//
// basePermits is the requested number (e.g., workers + buffer).
//
// The function caps permits to use at most memFraction (e.g., 0.5 for 50%) of
// available system memory, accounting for per-worker memory usage.
//
// With streaming frame pipeline, memory per worker is dramatically reduced:
// - Single frame buffer: ~6 MB for 1080p 10-bit (instead of ~5 GB for entire chunk)
// - SVT-AV1 encoder process: ~1 GB per instance
//
// Returns at least 1.
func CalculatePermits(basePermits int, width, height uint32, memFraction float64) int {
	permits := max(basePermits, 1)

	// Calculate estimated memory per worker with streaming:
	// - Single frame buffer: width * height * 3 bytes (10-bit YUV420)
	// - SVT-AV1 encoder process: ~1 GB per instance
	frameSize := uint64(width) * uint64(height) * 3
	encoderOverhead := uint64(1 << 30) // ~1 GB per SVT-AV1 process
	workerMemBytes := frameSize + encoderOverhead

	memPermits := util.MaxPermitsForMemory(workerMemBytes, memFraction)
	if memPermits < permits {
		permits = memPermits
	}

	return permits
}

// WorkerMemoryBytes returns the estimated memory per worker in bytes.
// With streaming, this is just one frame buffer plus encoder overhead.
// Useful for verbose logging.
func WorkerMemoryBytes(width, height uint32) uint64 {
	frameSize := uint64(width) * uint64(height) * 3
	encoderOverhead := uint64(1 << 30)
	return frameSize + encoderOverhead
}
