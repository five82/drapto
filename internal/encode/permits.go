package encode

import "github.com/five82/drapto/internal/util"

// Resolution-based worker limits.
// SVT-AV1 is internally parallel, so more workers beyond these limits
// just adds memory pressure without improving speed.
const (
	MaxWorkers4K    = 8  // 4K: ~4-5 GB per worker, high CPU per worker
	MaxWorkers1080p = 16 // 1080p: ~1-2 GB per worker
	MaxWorkersSD    = 24 // SD: ~500 MB per worker, more workers can help
)

// Estimated memory per worker by resolution (bytes).
// Conservative estimates based on real-world measurements.
const (
	MemPerWorker4K    = 5 << 30  // 5 GB
	MemPerWorker1080p = 2 << 30  // 2 GB
	MemPerWorkerSD    = 512 << 20 // 512 MB
)

// MemoryFraction is the fraction of available memory to use for workers.
const MemoryFraction = 0.6

// CapWorkers returns the safe number of workers based on resolution AND available memory.
// Returns (actualWorkers, wasCapped).
//
// Uses the minimum of:
//   - Resolution-based cap (more workers don't help due to SVT-AV1's internal parallelism)
//   - Memory-based cap (prevents OOM on systems with less RAM)
func CapWorkers(requested int, width, height uint32) (int, bool) {
	var maxByResolution int
	var memPerWorker uint64

	switch {
	case width >= 3840 || height >= 2160:
		maxByResolution = MaxWorkers4K
		memPerWorker = MemPerWorker4K
	case width >= 1920 || height >= 1080:
		maxByResolution = MaxWorkers1080p
		memPerWorker = MemPerWorker1080p
	default:
		maxByResolution = MaxWorkersSD
		memPerWorker = MemPerWorkerSD
	}

	// Calculate memory-based limit
	maxByMemory := maxByResolution // default if we can't determine memory
	if available := util.AvailableMemoryBytes(); available > 0 {
		usable := uint64(float64(available) * MemoryFraction)
		maxByMemory = max(int(usable/memPerWorker), 1)
	}

	// Take the lower of resolution and memory limits
	maxWorkers := min(maxByResolution, maxByMemory)

	if requested > maxWorkers {
		return maxWorkers, true
	}
	return requested, false
}

// CalculatePermits returns the number of in-flight chunk permits.
// Permits = workers + buffer to allow prefetching chunks.
// Returns at least 1.
func CalculatePermits(workers, buffer int) int {
	return max(workers+buffer, 1)
}
