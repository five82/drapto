package encode

import (
	"testing"
)

func TestCalculateThreadsPerWorker(t *testing.T) {
	tests := []struct {
		name    string
		workers int
		width   uint32
		wantMin int
		wantMax int
	}{
		// Edge cases
		{
			name:    "zero workers returns 1",
			workers: 0,
			width:   1920,
			wantMin: 1,
			wantMax: 1,
		},
		{
			name:    "negative workers returns 1",
			workers: -1,
			width:   1920,
			wantMin: 1,
			wantMax: 1,
		},

		// 4K resolution (width >= 3840) - max 16 threads
		{
			name:    "4K with 8 workers",
			workers: 8,
			width:   3840,
			wantMin: 1,
			wantMax: 16,
		},
		{
			name:    "4K with 1 worker",
			workers: 1,
			width:   3840,
			wantMin: 1,
			wantMax: 16,
		},

		// 1080p resolution (width >= 1920) - max 10 threads
		{
			name:    "1080p with 16 workers",
			workers: 16,
			width:   1920,
			wantMin: 1,
			wantMax: 10,
		},
		{
			name:    "1080p with 8 workers",
			workers: 8,
			width:   1920,
			wantMin: 1,
			wantMax: 10,
		},

		// SD resolution (width < 1920) - max 6 threads
		{
			name:    "SD with 24 workers",
			workers: 24,
			width:   720,
			wantMin: 1,
			wantMax: 6,
		},
		{
			name:    "720p with 12 workers",
			workers: 12,
			width:   1280,
			wantMin: 1,
			wantMax: 6,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := calculateThreadsPerWorker(tt.workers, tt.width)
			if got < tt.wantMin || got > tt.wantMax {
				t.Errorf("calculateThreadsPerWorker(%d, %d) = %d, want between %d and %d",
					tt.workers, tt.width, got, tt.wantMin, tt.wantMax)
			}
		})
	}
}

func TestCalculateThreadsPerWorkerResolutionCaps(t *testing.T) {
	// Verify resolution-based caps are enforced
	// Use 1 worker to get maximum threads (physical cores / 1 + SMT bonus)

	// 4K cap is 16
	threads4K := calculateThreadsPerWorker(1, 3840)
	if threads4K > 16 {
		t.Errorf("4K threads = %d, exceeds cap of 16", threads4K)
	}

	// 1080p cap is 10
	threads1080p := calculateThreadsPerWorker(1, 1920)
	if threads1080p > 10 {
		t.Errorf("1080p threads = %d, exceeds cap of 10", threads1080p)
	}

	// SD cap is 6
	threadsSD := calculateThreadsPerWorker(1, 720)
	if threadsSD > 6 {
		t.Errorf("SD threads = %d, exceeds cap of 6", threadsSD)
	}
}

func TestCalculateThreadsPerWorkerManyWorkers(t *testing.T) {
	// With many workers relative to cores, should get at least 1 thread
	// even with SMT bonus potentially adding 1
	threads := calculateThreadsPerWorker(100, 1920)
	if threads < 1 {
		t.Errorf("With 100 workers, got %d threads, want at least 1", threads)
	}
	if threads > 10 { // 1080p cap
		t.Errorf("With 100 workers at 1080p, got %d threads, exceeds cap of 10", threads)
	}
}

func TestCalculateThreadsPerWorkerReturnsPositive(t *testing.T) {
	// Test various combinations to ensure we always return positive
	widths := []uint32{480, 720, 1280, 1920, 2560, 3840, 4096, 7680}
	workerCounts := []int{1, 2, 4, 8, 12, 16, 24, 32, 64}

	for _, width := range widths {
		for _, workers := range workerCounts {
			threads := calculateThreadsPerWorker(workers, width)
			if threads < 1 {
				t.Errorf("calculateThreadsPerWorker(%d, %d) = %d, want >= 1", workers, width, threads)
			}
		}
	}
}
