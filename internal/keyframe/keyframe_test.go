package keyframe

import (
	"testing"
)

func TestCalculateMaxFrames(t *testing.T) {
	tests := []struct {
		name     string
		fpsNum   uint32
		fpsDen   uint32
		expected int
	}{
		{
			name:     "24fps film",
			fpsNum:   24,
			fpsDen:   1,
			expected: 720, // 24 * 30 = 720
		},
		{
			name:     "23.976fps NTSC film",
			fpsNum:   24000,
			fpsDen:   1001,
			expected: 719, // ~23.976 * 30 = ~719
		},
		{
			name:     "30fps",
			fpsNum:   30,
			fpsDen:   1,
			expected: 900, // 30 * 30 = 900
		},
		{
			name:     "60fps capped at 1000",
			fpsNum:   60,
			fpsDen:   1,
			expected: 1000, // 60 * 30 = 1800, but capped at 1000
		},
		{
			name:     "50fps capped at 1000",
			fpsNum:   50,
			fpsDen:   1,
			expected: 1000, // 50 * 30 = 1500, but capped at 1000
		},
		{
			name:     "zero denominator returns default",
			fpsNum:   24,
			fpsDen:   0,
			expected: 1000,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := CalculateMaxFrames(tt.fpsNum, tt.fpsDen)
			if result != tt.expected {
				t.Errorf("CalculateMaxFrames(%d, %d) = %d, want %d",
					tt.fpsNum, tt.fpsDen, result, tt.expected)
			}
		})
	}
}

func TestSplitLongScenes(t *testing.T) {
	tests := []struct {
		name        string
		keyframes   []int
		totalFrames int
		maxFrames   int
		expected    []int
	}{
		{
			name:        "no split needed",
			keyframes:   []int{0, 100, 200},
			totalFrames: 300,
			maxFrames:   200,
			expected:    []int{0, 100, 200},
		},
		{
			name:        "split one long scene",
			keyframes:   []int{0, 1000},
			totalFrames: 1200,
			maxFrames:   300,
			expected:    []int{0, 250, 500, 750, 1000}, // 1000 frames split into 4 chunks of 250
		},
		{
			name:        "split last scene",
			keyframes:   []int{0, 100},
			totalFrames: 800,
			maxFrames:   300,
			expected:    []int{0, 100, 333, 566}, // 700 frames split into 3 chunks
		},
		{
			name:        "empty keyframes",
			keyframes:   []int{},
			totalFrames: 1000,
			maxFrames:   300,
			expected:    []int{0},
		},
		{
			name:        "single keyframe at 0",
			keyframes:   []int{0},
			totalFrames: 600,
			maxFrames:   300,
			expected:    []int{0, 300}, // 600 frames split into 2 chunks of 300
		},
		{
			name:        "exact max size",
			keyframes:   []int{0, 300},
			totalFrames: 600,
			maxFrames:   300,
			expected:    []int{0, 300},
		},
		{
			name:        "just over max",
			keyframes:   []int{0, 301},
			totalFrames: 602,
			maxFrames:   300,
			expected:    []int{0, 150, 301, 451}, // Both 301-frame scenes split into 2 chunks each
		},
		{
			name:        "multiple long scenes",
			keyframes:   []int{0, 500, 1000},
			totalFrames: 1500,
			maxFrames:   200,
			expected:    []int{0, 166, 332, 500, 666, 832, 1000, 1166, 1332},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := SplitLongScenes(tt.keyframes, tt.totalFrames, tt.maxFrames)
			if !intSliceEqual(result, tt.expected) {
				t.Errorf("SplitLongScenes(%v, %d, %d) = %v, want %v",
					tt.keyframes, tt.totalFrames, tt.maxFrames, result, tt.expected)
			}
		})
	}
}

func TestDedupe(t *testing.T) {
	tests := []struct {
		name     string
		input    []int
		expected []int
	}{
		{
			name:     "no duplicates",
			input:    []int{1, 2, 3},
			expected: []int{1, 2, 3},
		},
		{
			name:     "with duplicates",
			input:    []int{1, 1, 2, 3, 3, 3},
			expected: []int{1, 2, 3},
		},
		{
			name:     "all same",
			input:    []int{5, 5, 5},
			expected: []int{5},
		},
		{
			name:     "empty",
			input:    []int{},
			expected: []int{},
		},
		{
			name:     "single element",
			input:    []int{42},
			expected: []int{42},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := dedupe(tt.input)
			if !intSliceEqual(result, tt.expected) {
				t.Errorf("dedupe(%v) = %v, want %v",
					tt.input, result, tt.expected)
			}
		})
	}
}

func TestCalculateMinFrames(t *testing.T) {
	tests := []struct {
		name            string
		fpsNum          uint32
		fpsDen          uint32
		minDurationSecs float64
		expected        int
	}{
		{
			name:            "24fps 4 seconds",
			fpsNum:          24,
			fpsDen:          1,
			minDurationSecs: 4.0,
			expected:        96, // 24 * 4 = 96
		},
		{
			name:            "23.976fps 4 seconds",
			fpsNum:          24000,
			fpsDen:          1001,
			minDurationSecs: 4.0,
			expected:        95, // ~23.976 * 4 = ~95
		},
		{
			name:            "zero denominator",
			fpsNum:          24,
			fpsDen:          0,
			minDurationSecs: 4.0,
			expected:        0,
		},
		{
			name:            "zero duration",
			fpsNum:          24,
			fpsDen:          1,
			minDurationSecs: 0,
			expected:        0,
		},
		{
			name:            "negative duration",
			fpsNum:          24,
			fpsDen:          1,
			minDurationSecs: -1.0,
			expected:        0,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := CalculateMinFrames(tt.fpsNum, tt.fpsDen, tt.minDurationSecs)
			if result != tt.expected {
				t.Errorf("CalculateMinFrames(%d, %d, %f) = %d, want %d",
					tt.fpsNum, tt.fpsDen, tt.minDurationSecs, result, tt.expected)
			}
		})
	}
}

func TestMergeShortScenes(t *testing.T) {
	tests := []struct {
		name        string
		keyframes   []int
		totalFrames int
		minFrames   int
		expected    []int
	}{
		{
			name:        "no merge needed",
			keyframes:   []int{0, 100, 200},
			totalFrames: 300,
			minFrames:   50,
			expected:    []int{0, 100, 200},
		},
		{
			name:        "merge short middle scene with smaller neighbor",
			keyframes:   []int{0, 100, 120, 300},
			totalFrames: 400,
			minFrames:   50,
			expected:    []int{0, 120, 300}, // 20-frame scene merged with prev (100 frames < 180 next)
		},
		{
			name:        "merge short scene with next when next is smaller",
			keyframes:   []int{0, 200, 220, 250},
			totalFrames: 300,
			minFrames:   50,
			expected:    []int{0, 200, 250}, // 20-frame scene merges with next (30 frames), result 50 frames is at threshold
		},
		{
			name:        "cascade merge multiple short scenes",
			keyframes:   []int{0, 10, 20, 30, 200},
			totalFrames: 300,
			minFrames:   50,
			expected:    []int{0, 200}, // All tiny scenes get merged
		},
		{
			name:        "empty keyframes",
			keyframes:   []int{},
			totalFrames: 1000,
			minFrames:   50,
			expected:    []int{},
		},
		{
			name:        "single keyframe",
			keyframes:   []int{0},
			totalFrames: 100,
			minFrames:   50,
			expected:    []int{0},
		},
		{
			name:        "minFrames zero disables merging",
			keyframes:   []int{0, 10, 20},
			totalFrames: 100,
			minFrames:   0,
			expected:    []int{0, 10, 20},
		},
		{
			name:        "first scene too short stays (can't merge frame 0)",
			keyframes:   []int{0, 10, 200},
			totalFrames: 300,
			minFrames:   50,
			expected:    []int{0, 200}, // 10-frame scene at start can't remove frame 0, so it merges with next by removing keyframe at 10
		},
		{
			name:        "last scene too short merges with previous",
			keyframes:   []int{0, 100, 180},
			totalFrames: 200,
			minFrames:   50,
			expected:    []int{0, 100}, // 20-frame last scene merges with prev
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := MergeShortScenes(tt.keyframes, tt.totalFrames, tt.minFrames)
			if !intSliceEqual(result, tt.expected) {
				t.Errorf("MergeShortScenes(%v, %d, %d) = %v, want %v",
					tt.keyframes, tt.totalFrames, tt.minFrames, result, tt.expected)
			}
		})
	}
}

func intSliceEqual(a, b []int) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}
