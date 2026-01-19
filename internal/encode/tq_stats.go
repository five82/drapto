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

	// Prediction accuracy
	AvgPredictionDelta float64
	MaxPredictionDelta float64
	PredictedChunks    int

	// Chunk lengths
	MinFrames int
	MaxFrames int
	MinDur    float64
	MaxDur    float64
	NumChunks int

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
		RoundsBreakdown: make(map[int]int),
		MinRounds:       math.MaxInt,
		MinFrames:       math.MaxInt,
		MinDur:          math.MaxFloat64,
	}

	var totalRounds int
	var crfSum, totalPredDelta, totalSamplingDelta float64
	var crfValues []float64
	var validCount int

	for _, r := range results {
		if r.Error != nil {
			continue
		}
		validCount++

		// Iterations
		totalRounds += r.Round
		stats.MinRounds = min(stats.MinRounds, r.Round)
		stats.MaxRounds = max(stats.MaxRounds, r.Round)

		// Rounds breakdown (group 4+ together)
		roundKey := min(r.Round, 4)
		stats.RoundsBreakdown[roundKey]++

		// CRF values
		crfValues = append(crfValues, r.FinalCRF)
		crfSum += r.FinalCRF

		// Chunk lengths
		stats.MinFrames = min(stats.MinFrames, r.Frames)
		stats.MaxFrames = max(stats.MaxFrames, r.Frames)
		if fps > 0 {
			dur := float64(r.Frames) / fps
			stats.MinDur = min(stats.MinDur, dur)
			stats.MaxDur = max(stats.MaxDur, dur)
		}

		// Prediction accuracy (only for chunks with predictions)
		if r.PredictedCRF > 0 {
			delta := math.Abs(r.PredictedCRF - r.FinalCRF)
			totalPredDelta += delta
			stats.MaxPredictionDelta = max(stats.MaxPredictionDelta, delta)
			stats.PredictedChunks++
		}

		// Sampling accuracy (only for sampled chunks with full-chunk score computed)
		if r.UsedSampling && r.FullChunkScore > 0 {
			delta := math.Abs(r.FinalScore - r.FullChunkScore)
			totalSamplingDelta += delta
			stats.MaxSamplingDelta = max(stats.MaxSamplingDelta, delta)
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

	stats.NumChunks = validCount

	// Calculate averages
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
		for _, crf := range crfValues {
			stats.CRFMin = min(stats.CRFMin, crf)
			stats.CRFMax = max(stats.CRFMax, crf)
		}
		stats.CRFMean = crfSum / float64(len(crfValues))

		// Standard deviation
		var variance float64
		for _, crf := range crfValues {
			diff := crf - stats.CRFMean
			variance += diff * diff
		}
		stats.CRFStdDev = math.Sqrt(variance / float64(len(crfValues)))
	}

	// Handle edge case where no valid results were found
	if stats.MinRounds == math.MaxInt {
		stats.MinRounds = 0
	}
	if stats.MinFrames == math.MaxInt {
		stats.MinFrames = 0
	}
	if stats.MinDur == math.MaxFloat64 {
		stats.MinDur = 0
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
func OutputTQStats(stats *TQStats, rep reporter.Reporter, targetMin, targetMax float64, results []tqResult, fps float64) {
	if stats == nil {
		return
	}

	rep.Verbose("")
	rep.Verbose("=== TQ Debug Statistics ===")

	// Iterations
	rep.Verbose(fmt.Sprintf("Iterations: avg=%.1f, min=%d, max=%d",
		stats.AvgRounds, stats.MinRounds, stats.MaxRounds))

	// Score distribution
	outputScoreDistribution(rep, results, targetMin, targetMax)

	// Prediction accuracy
	if stats.PredictedChunks > 0 {
		rep.Verbose(fmt.Sprintf("Prediction accuracy: avg delta=%.1f CRF, max delta=%.1f CRF (%d chunks)",
			stats.AvgPredictionDelta, stats.MaxPredictionDelta, stats.PredictedChunks))
	}

	// Chunk lengths
	if stats.NumChunks > 0 {
		rep.Verbose(fmt.Sprintf("Chunk lengths: %d chunks, frames %d-%d, duration %.1fs-%.1fs",
			stats.NumChunks, stats.MinFrames, stats.MaxFrames, stats.MinDur, stats.MaxDur))

		// Per-chunk details
		for _, r := range results {
			if r.Error != nil {
				continue
			}
			dur := float64(r.Frames) / fps
			rep.Verbose(fmt.Sprintf("  Chunk %d: %d frames (%.1fs)", r.ChunkIdx, r.Frames, dur))
		}
	}

	// CRF distribution
	rep.Verbose(fmt.Sprintf("CRF distribution: min=%.0f, max=%.0f, mean=%.1f, stddev=%.1f",
		stats.CRFMin, stats.CRFMax, stats.CRFMean, stats.CRFStdDev))

	// Rounds breakdown
	outputRoundsBreakdown(rep, stats.RoundsBreakdown)

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
	outputFailedChunks(rep, stats.FailedChunks)

	rep.Verbose("=== End TQ Debug Statistics ===")
	rep.Verbose("")
}

// outputScoreDistribution outputs score distribution buckets.
func outputScoreDistribution(rep reporter.Reporter, results []tqResult, targetMin, targetMax float64) {
	buckets := ComputeScoreDistribution(results, targetMin, targetMax)
	rep.Verbose(fmt.Sprintf("Score distribution (target %.0f-%.0f):", targetMin, targetMax))

	// Sort bucket keys for consistent output
	var bucketKeys []string
	for k := range buckets {
		if k != "below" && k != "above" {
			bucketKeys = append(bucketKeys, k)
		}
	}
	sort.Strings(bucketKeys)

	if buckets["below"] > 0 {
		rep.Verbose(fmt.Sprintf("  <%.0f: %d chunks", targetMin, buckets["below"]))
	}
	for _, k := range bucketKeys {
		if buckets[k] > 0 {
			rep.Verbose(fmt.Sprintf("  %s: %d chunks", k, buckets[k]))
		}
	}
	if buckets["above"] > 0 {
		rep.Verbose(fmt.Sprintf("  >%.0f: %d chunks", targetMax, buckets["above"]))
	}
}

// outputRoundsBreakdown outputs the rounds breakdown.
func outputRoundsBreakdown(rep reporter.Reporter, breakdown map[int]int) {
	rep.Verbose("Rounds breakdown:")
	for round := 1; round <= 4; round++ {
		count := breakdown[round]
		if count == 0 {
			continue
		}
		if round == 4 {
			rep.Verbose(fmt.Sprintf("  4+ rounds: %d chunks", count))
		} else {
			rep.Verbose(fmt.Sprintf("  %d round%s: %d chunks", round, pluralS(round), count))
		}
	}
}

// outputFailedChunks outputs details for chunks that failed to converge.
func outputFailedChunks(rep reporter.Reporter, failed []FailedChunkInfo) {
	if len(failed) == 0 {
		return
	}
	rep.Verbose(fmt.Sprintf("Failed convergence: %d chunks hit max rounds", len(failed)))
	for _, fc := range failed {
		rep.Verbose(fmt.Sprintf("  Chunk %d: final CRF=%.0f, score=%.1f",
			fc.ChunkIdx, fc.FinalCRF, fc.FinalScore))
		rep.Verbose("    Probe history:")
		for _, p := range fc.Probes {
			rep.Verbose(fmt.Sprintf("      CRF %.0f -> %.1f", p.CRF, p.Score))
		}
	}
}

func pluralS(n int) string {
	if n == 1 {
		return ""
	}
	return "s"
}
