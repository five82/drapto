package encode

import "github.com/five82/drapto/internal/util"

// CalculatePermits determines the number of in-flight chunk permits based on
// the requested base permits and available system memory.
//
// basePermits is the requested number (e.g., workers + buffer for standard mode,
// or just workers for TQ mode).
//
// The function caps permits to use at most memFraction (e.g., 0.5 for 50%) of
// available system memory, accounting for YUV buffer size and encoder overhead.
//
// Returns at least 1.
func CalculatePermits(basePermits int, width, height uint32, avgFramesPerChunk int, memFraction float64) int {
	permits := max(basePermits, 1)

	// Calculate estimated memory per in-flight chunk:
	// - YUV buffer: frames * frameSize (10-bit YUV420: width * height * 3 bytes)
	// - SVT-AV1 encoder process: ~1 GB per instance
	frameSize := uint64(width) * uint64(height) * 3
	yuvMemBytes := frameSize * uint64(avgFramesPerChunk)
	encoderOverhead := uint64(1 << 30) // ~1 GB per SVT-AV1 process
	chunkMemBytes := yuvMemBytes + encoderOverhead

	memPermits := util.MaxPermitsForMemory(chunkMemBytes, memFraction)
	if memPermits < permits {
		permits = memPermits
	}

	return permits
}

// ChunkMemoryBytes returns the estimated memory per in-flight chunk in bytes.
// Useful for verbose logging.
func ChunkMemoryBytes(width, height uint32, avgFramesPerChunk int) uint64 {
	frameSize := uint64(width) * uint64(height) * 3
	yuvMemBytes := frameSize * uint64(avgFramesPerChunk)
	encoderOverhead := uint64(1 << 30)
	return yuvMemBytes + encoderOverhead
}
