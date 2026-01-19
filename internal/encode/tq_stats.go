package encode

import (
	"fmt"
	"math"
	"sort"

	"github.com/five82/drapto/internal/reporter"
	"github.com/five82/drapto/internal/tq"
)

// TQStats contains aggregated statistics from TQ encoding.
type TQStats struct {
	// Iterations
	AvgRounds float64
	MinRounds int
	MaxRounds int

	// Score distribution
	ScoreBuckets map[string]int

	// Prediction accuracy
	AvgPredictionDelta float64
	MaxPredictionDelta float64
	PredictedChunks    int

	// Chunk lengths
	ChunkFrames    []int
	ChunkDurations []float64

	// CRF distribution
	CRFMin    float64
	CRFMax    float64
	CRFMean   float64
	CRFStdDev float64

	// Rounds breakdown
	RoundsBreakdown map[int]int

	// Sampling accuracy
	AvgSamplingDelta float64
	MaxSamplingDelta float64
	SampledChunks    int

	// Bound expansion
	BoundExpansionCount int

	// Failed convergence
	FailedChunks []FailedChunkInfo
}

// FailedChunkInfo contains details for chunks that hit max rounds.
type FailedChunkInfo struct {
	ChunkIdx   int
	Probes     []tq.ProbeEntry
	FinalCRF   float64
	FinalScore float64
}

// ComputeTQStats computes aggregated statistics from TQ results.
func ComputeTQStats(results []tqResult, fps float64, maxRounds int) *TQStats {
	if len(results) == 0 {
		return nil
	}

	stats := &TQStats{
		ScoreBuckets:    make(map[string]int),
		RoundsBreakdown: make(map[int]int),
		ChunkFrames:     make([]int, 0, len(results)),
		ChunkDurations:  make([]float64, 0, len(results)),
	}

	var totalRounds int
	stats.MinRounds = math.MaxInt
	stats.MaxRounds = 0

	var crfValues []float64
	var totalPredDelta float64
	var totalSamplingDelta float64

	for _, r := range results {
		if r.Error != nil {
			continue
		}

		// Iterations
		totalRounds += r.Round
		if r.Round < stats.MinRounds {
			stats.MinRounds = r.Round
		}
		if r.Round > stats.MaxRounds {
			stats.MaxRounds = r.Round
		}

		// Rounds breakdown
		roundKey := r.Round
		if roundKey >= 4 {
			roundKey = 4 // Group 4+ together
		}
		stats.RoundsBreakdown[roundKey]++

		// CRF values for distribution
		crfValues = append(crfValues, r.FinalCRF)

		// Chunk lengths
		stats.ChunkFrames = append(stats.ChunkFrames, r.Frames)
		if fps > 0 {
			stats.ChunkDurations = append(stats.ChunkDurations, float64(r.Frames)/fps)
		}

		// Prediction accuracy (only for chunks with predictions)
		if r.PredictedCRF > 0 {
			delta := math.Abs(r.PredictedCRF - r.FinalCRF)
			totalPredDelta += delta
			if delta > stats.MaxPredictionDelta {
				stats.MaxPredictionDelta = delta
			}
			stats.PredictedChunks++
		}

		// Sampling accuracy (only for sampled chunks with full-chunk score computed)
		if r.UsedSampling && r.FullChunkScore > 0 {
			delta := math.Abs(r.FinalScore - r.FullChunkScore)
			totalSamplingDelta += delta
			if delta > stats.MaxSamplingDelta {
				stats.MaxSamplingDelta = delta
			}
			stats.SampledChunks++
		}

		// Bound expansion
		if r.BoundExpanded {
			stats.BoundExpansionCount++
		}

		// Failed convergence (hit max rounds)
		if r.Round >= maxRounds {
			stats.FailedChunks = append(stats.FailedChunks, FailedChunkInfo{
				ChunkIdx:   r.ChunkIdx,
				Probes:     r.Probes,
				FinalCRF:   r.FinalCRF,
				FinalScore: r.FinalScore,
			})
		}
	}

	// Calculate averages
	validCount := len(results)
	for _, r := range results {
		if r.Error != nil {
			validCount--
		}
	}

	if validCount > 0 {
		stats.AvgRounds = float64(totalRounds) / float64(validCount)
	}
	if stats.PredictedChunks > 0 {
		stats.AvgPredictionDelta = totalPredDelta / float64(stats.PredictedChunks)
	}
	if stats.SampledChunks > 0 {
		stats.AvgSamplingDelta = totalSamplingDelta / float64(stats.SampledChunks)
	}

	// CRF distribution
	if len(crfValues) > 0 {
		stats.CRFMin = crfValues[0]
		stats.CRFMax = crfValues[0]
		var sum float64
		for _, crf := range crfValues {
			if crf < stats.CRFMin {
				stats.CRFMin = crf
			}
			if crf > stats.CRFMax {
				stats.CRFMax = crf
			}
			sum += crf
		}
		stats.CRFMean = sum / float64(len(crfValues))

		// Standard deviation
		var variance float64
		for _, crf := range crfValues {
			diff := crf - stats.CRFMean
			variance += diff * diff
		}
		stats.CRFStdDev = math.Sqrt(variance / float64(len(crfValues)))
	}

	// Handle edge case where no valid rounds were found
	if stats.MinRounds == math.MaxInt {
		stats.MinRounds = 0
	}

	return stats
}

// ComputeScoreDistribution computes the score distribution buckets.
func ComputeScoreDistribution(results []tqResult, targetMin, targetMax float64) map[string]int {
	buckets := make(map[string]int)

	// Create 1-point buckets within the target range
	bucketStart := math.Floor(targetMin)
	bucketEnd := math.Ceil(targetMax)

	for score := bucketStart; score < bucketEnd; score++ {
		key := fmt.Sprintf("%.0f-%.0f", score, score+1)
		buckets[key] = 0
	}
	buckets["below"] = 0
	buckets["above"] = 0

	for _, r := range results {
		if r.Error != nil {
			continue
		}

		if r.FinalScore < targetMin {
			buckets["below"]++
		} else if r.FinalScore > targetMax {
			buckets["above"]++
		} else {
			bucketScore := math.Floor(r.FinalScore)
			key := fmt.Sprintf("%.0f-%.0f", bucketScore, bucketScore+1)
			buckets[key]++
		}
	}

	return buckets
}

// OutputTQStats outputs the TQ statistics to the reporter.
func OutputTQStats(stats *TQStats, rep reporter.Reporter, targetMin, targetMax float64, results []tqResult) {
	if stats == nil {
		return
	}

	rep.Verbose("")
	rep.Verbose("=== TQ Debug Statistics ===")

	// Iterations
	rep.Verbose(fmt.Sprintf("Iterations: avg=%.1f, min=%d, max=%d",
		stats.AvgRounds, stats.MinRounds, stats.MaxRounds))

	// Score distribution
	scoreBuckets := ComputeScoreDistribution(results, targetMin, targetMax)
	rep.Verbose(fmt.Sprintf("Score distribution (target %.0f-%.0f):", targetMin, targetMax))

	// Sort bucket keys for consistent output
	var bucketKeys []string
	for k := range scoreBuckets {
		if k != "below" && k != "above" {
			bucketKeys = append(bucketKeys, k)
		}
	}
	sort.Strings(bucketKeys)

	if scoreBuckets["below"] > 0 {
		rep.Verbose(fmt.Sprintf("  <%.0f: %d chunks", targetMin, scoreBuckets["below"]))
	}
	for _, k := range bucketKeys {
		if scoreBuckets[k] > 0 {
			rep.Verbose(fmt.Sprintf("  %s: %d chunks", k, scoreBuckets[k]))
		}
	}
	if scoreBuckets["above"] > 0 {
		rep.Verbose(fmt.Sprintf("  >%.0f: %d chunks", targetMax, scoreBuckets["above"]))
	}

	// Prediction accuracy
	if stats.PredictedChunks > 0 {
		rep.Verbose(fmt.Sprintf("Prediction accuracy: avg delta=%.1f CRF, max delta=%.1f CRF (%d chunks)",
			stats.AvgPredictionDelta, stats.MaxPredictionDelta, stats.PredictedChunks))
	}

	// Chunk lengths
	if len(stats.ChunkFrames) > 0 {
		minFrames, maxFrames := stats.ChunkFrames[0], stats.ChunkFrames[0]
		minDur, maxDur := stats.ChunkDurations[0], stats.ChunkDurations[0]
		for i, f := range stats.ChunkFrames {
			if f < minFrames {
				minFrames = f
			}
			if f > maxFrames {
				maxFrames = f
			}
			if i < len(stats.ChunkDurations) {
				if stats.ChunkDurations[i] < minDur {
					minDur = stats.ChunkDurations[i]
				}
				if stats.ChunkDurations[i] > maxDur {
					maxDur = stats.ChunkDurations[i]
				}
			}
		}
		rep.Verbose(fmt.Sprintf("Chunk lengths: %d chunks, frames %d-%d, duration %.1fs-%.1fs",
			len(stats.ChunkFrames), minFrames, maxFrames, minDur, maxDur))

		// Per-chunk details
		for i, r := range results {
			if r.Error != nil {
				continue
			}
			var dur float64
			if i < len(stats.ChunkDurations) {
				dur = stats.ChunkDurations[i]
			}
			rep.Verbose(fmt.Sprintf("  Chunk %d: %d frames (%.1fs)", r.ChunkIdx, r.Frames, dur))
		}
	}

	// CRF distribution
	rep.Verbose(fmt.Sprintf("CRF distribution: min=%.0f, max=%.0f, mean=%.1f, stddev=%.1f",
		stats.CRFMin, stats.CRFMax, stats.CRFMean, stats.CRFStdDev))

	// Rounds breakdown
	rep.Verbose("Rounds breakdown:")
	for round := 1; round <= 4; round++ {
		count := stats.RoundsBreakdown[round]
		if count > 0 {
			if round == 4 {
				rep.Verbose(fmt.Sprintf("  4+ rounds: %d chunks", count))
			} else {
				rep.Verbose(fmt.Sprintf("  %d round%s: %d chunks", round, pluralS(round), count))
			}
		}
	}

	// Sampling accuracy
	if stats.SampledChunks > 0 {
		rep.Verbose(fmt.Sprintf("Sampling accuracy: avg delta=%.1f, max delta=%.1f (%d sampled chunks)",
			stats.AvgSamplingDelta, stats.MaxSamplingDelta, stats.SampledChunks))
	}

	// Bound expansions
	if stats.BoundExpansionCount > 0 {
		rep.Verbose(fmt.Sprintf("Bound expansions: %d chunks", stats.BoundExpansionCount))
	}

	// Failed convergence
	if len(stats.FailedChunks) > 0 {
		rep.Verbose(fmt.Sprintf("Failed convergence: %d chunks hit max rounds", len(stats.FailedChunks)))
		for _, fc := range stats.FailedChunks {
			rep.Verbose(fmt.Sprintf("  Chunk %d: final CRF=%.0f, score=%.1f",
				fc.ChunkIdx, fc.FinalCRF, fc.FinalScore))
			rep.Verbose("    Probe history:")
			for _, p := range fc.Probes {
				rep.Verbose(fmt.Sprintf("      CRF %.0f -> %.1f", p.CRF, p.Score))
			}
		}
	}

	rep.Verbose("=== End TQ Debug Statistics ===")
	rep.Verbose("")
}

func pluralS(n int) string {
	if n == 1 {
		return ""
	}
	return "s"
}
