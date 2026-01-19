# Target Quality Algorithm: Technical Reference

This document describes the CRF probe and prediction algorithm used in drapto's Target Quality (TQ) mode. The algorithm iteratively searches for the optimal CRF value that produces a target SSIMULACRA2 score for each chunk.

## Algorithm Overview

The TQ algorithm solves an inverse problem: given a target quality score, find the CRF that achieves it. Since the CRF-to-score relationship is non-linear and content-dependent, the algorithm uses iterative probing with adaptive interpolation.

```
┌─────────────────────────────────────────────────────────────────┐
│                        TQ Search Loop                           │
│                                                                 │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐  │
│  │  Predict │───▶│  Encode  │───▶│ Compute  │───▶│ Converge │  │
│  │   CRF    │    │  Probe   │    │  Score   │    │  Check   │  │
│  └──────────┘    └──────────┘    └──────────┘    └────┬─────┘  │
│       ▲                                               │        │
│       │              ┌────────────────────────────────┘        │
│       │              │                                         │
│       │         ┌────▼────┐                                    │
│       │         │ Within  │──Yes──▶ Done (use best CRF)        │
│       │         │ target? │                                    │
│       │         └────┬────┘                                    │
│       │              │ No                                      │
│       │              ▼                                         │
│       │         ┌─────────┐                                    │
│       └─────────│ Update  │                                    │
│                 │ bounds  │                                    │
│                 └─────────┘                                    │
└─────────────────────────────────────────────────────────────────┘
```

## Data Structures

### State (`internal/tq/state.go`)

Each chunk maintains independent search state:

```go
type State struct {
    Probes     []Probe   // All completed probe results
    SearchMin  float64   // Current lower CRF bound
    SearchMax  float64   // Current upper CRF bound
    QPMin      float64   // Hard lower limit (default: 8)
    QPMax      float64   // Hard upper limit (default: 48)
    Round      int       // Current iteration (1-indexed)
    Target     float64   // Desired SSIMULACRA2 score
    LastCRF    float64   // Most recent CRF tested
}

type Probe struct {
    CRF         float64   // Quality parameter used
    Score       float64   // Resulting SSIMULACRA2 score
    FrameScores []float64 // Per-frame scores
    Size        uint64    // Output file size
}
```

### Config (`internal/tq/config.go`)

```go
type Config struct {
    TargetMin  float64  // Lower bound of acceptable score range
    TargetMax  float64  // Upper bound of acceptable score range
    Target     float64  // Midpoint: (TargetMin + TargetMax) / 2
    Tolerance  float64  // Half-width: (TargetMax - TargetMin) / 2
    QPMin      float64  // CRF search floor (default: 8)
    QPMax      float64  // CRF search ceiling (default: 48)
    MaxRounds  int      // Maximum iterations (default: 10)
    MetricMode string   // Score aggregation: "mean" or "pN"
}
```

## CRF Selection Strategy

The algorithm uses different strategies based on the iteration round (`internal/tq/search.go:NextCRF`):

| Round | Strategy | Rationale |
|-------|----------|-----------|
| 1-2 | Binary search | Gather initial data points at bounds midpoint |
| 3 | Linear interpolation | 2 points available |
| 4 | Fritsch-Carlson spline | 3 points available |
| 5 | PCHIP spline | 4 points available |
| 6+ | Akima spline | 5+ points available |

### Binary Search (Rounds 1-2)

Simple midpoint calculation:

```go
func BinarySearch(min, max float64) float64 {
    return round((min + max) / 2)
}
```

### Interpolation (Rounds 3+)

Once sufficient probes exist, the algorithm fits a curve to the (score, CRF) data points and evaluates it at the target score:

```
Score (x-axis)          CRF (y-axis)
                           │
         probe 1 ──────────┼──▶ ●
                           │
         probe 2 ──────────┼──▶ ●
                           │      ╲
         target  ──────────┼──▶    ? (interpolated)
                           │      ╱
         probe 3 ──────────┼──▶ ●
                           │
```

The probes are sorted by score before interpolation, not by CRF, because we're inverting the relationship.

## Interpolation Methods (`internal/tq/interp.go`)

### Linear Interpolation (Round 3)

With only 2 data points, linear interpolation is used:

```go
func Lerp(x, y [2]float64, xi float64) *float64 {
    t := (xi - x[0]) / (x[1] - x[0])
    result := t*(y[1]-y[0]) + y[0]
    return &result
}
```

### Fritsch-Carlson (Round 4)

A monotonicity-preserving cubic Hermite spline for 3 points. Monotonicity is important because the CRF-score relationship should be monotonic (lower CRF → higher quality).

The method computes weighted harmonic means of slopes at interior points:

```go
// Interior point derivative (weighted harmonic mean)
if d0*d1 <= 0 {
    m[1] = 0  // Sign change = local extremum
} else {
    w1 := 2*h1 + h0
    w2 := 2*h0 + h1
    m[1] = (w1 + w2) / (w1/d0 + w2/d1)
}
```

### PCHIP (Round 5)

Piecewise Cubic Hermite Interpolating Polynomial for 4 points. Extends Fritsch-Carlson with additional monotonicity constraints:

```go
// Monotonicity constraint: limit derivatives to prevent overshoot
tau := alpha*alpha + beta*beta
if tau > 9.0 {  // maxTau² = 9
    scale := 3.0 / sqrt(tau)
    d[i] = scale * alpha * slopes[i]
    d[i+1] = scale * beta * slopes[i]
}
```

### Akima Spline (Round 6+)

For 5+ points, Akima spline interpolation provides smooth curves while avoiding the oscillation problems of natural cubic splines:

```go
// Akima tangent calculation (weighted average based on slope differences)
w1 := abs(m[i+2] - m[i+1])
w2 := abs(m[i] - m[i+1])
tan[i] = (w1*m[i] + w2*m[i+1]) / (w1 + w2)
```

Akima's method gives more weight to slopes that differ less from their neighbors, providing natural-looking curves that respect local data behavior.

## Bounds Management

### Initial Bounds

Bounds are initialized based on CRF prediction from nearby completed chunks:

```go
func NewState(target, qpMin, qpMax, predictedCRF float64) *State {
    searchMin := qpMin
    searchMax := qpMax

    if predictedCRF > 0 {
        // Narrow to ±5 around prediction
        searchMin = max(qpMin, predictedCRF-5)
        searchMax = min(qpMax, predictedCRF+5)
    }
    // ...
}
```

### Bounds Update (`internal/tq/search.go:UpdateBounds`)

After each probe, bounds are tightened based on the score:

```go
if score < target - tolerance {
    // Quality too low → need lower CRF
    SearchMax = LastCRF - 1
} else if score > target + tolerance {
    // Quality too high → need higher CRF
    SearchMin = LastCRF + 1
}
```

### Bounds Expansion

If bounds cross (SearchMin > SearchMax) before convergence, the algorithm attempts to expand:

```go
// Score too low but can go lower
if score < target-tolerance && LastCRF-1 >= QPMin {
    SearchMin = max(QPMin, LastCRF-5)
    SearchMax = LastCRF - 1
}

// Score too high but can go higher
if score > target+tolerance && LastCRF+1 <= QPMax {
    SearchMin = LastCRF + 1
    SearchMax = min(QPMax, LastCRF+5)
}
```

## Convergence Criteria

The search terminates when any condition is met (`internal/tq/search.go:CheckComplete`):

1. **Score within tolerance**: `|score - target| ≤ tolerance`
2. **Maximum rounds reached**: `round ≥ MaxRounds` (default: 10)
3. **Bounds exhausted**: SearchMin > SearchMax and cannot expand

On termination, the probe with the score closest to target is selected as the final CRF.

## Cross-Chunk CRF Prediction (`internal/tq/tracker.go`)

Adjacent video chunks typically have similar complexity, so completed chunks inform starting bounds for pending chunks.

### Tracker

```go
type CRFTracker struct {
    results map[int]float64  // chunkIdx → final CRF
}
```

### Prediction Algorithm

```go
func (t *CRFTracker) Predict(chunkIdx int, defaultCRF float64) float64 {
    // Find 4 nearest completed chunks
    // Weight by inverse distance: w = 1/distance
    // Return weighted average of their CRF values
}
```

Example: For chunk 10 with chunks 5, 8, 12, 15 completed:

| Chunk | Distance | CRF | Weight | Contribution |
|-------|----------|-----|--------|--------------|
| 8 | 2 | 28 | 0.50 | 14.0 |
| 12 | 2 | 30 | 0.50 | 15.0 |
| 5 | 5 | 25 | 0.20 | 5.0 |
| 15 | 5 | 32 | 0.20 | 6.4 |

Weighted sum = 40.4, Weight sum = 1.40
**Predicted CRF = 40.4 / 1.40 ≈ 29**

This prediction narrows the search bounds from [8, 48] to [24, 34], reducing iterations needed.

### Disabling Prediction

For troubleshooting or benchmarking, prediction can be disabled with `--no-tq-prediction`. When disabled, each chunk uses the full CRF search range instead of narrowed bounds based on neighbors.

## Sample-Based Probing

For long chunks, encoding full frames during every probe iteration is wasteful. Sample-based probing encodes only a representative portion during the search phase.

### Sample Calculation

```go
// Default: 3 seconds sample, 6 second minimum chunk
sampleDuration  = 3.0  // seconds
sampleMinChunk  = 6.0  // seconds

// Only sample if chunk is long enough
if chunkDuration >= sampleMinChunk {
    sampleFrames = fps * sampleDuration
    // Extract from middle of chunk for representativeness
}
```

### Warmup Frames

The first ~0.5 seconds of encoded output are discarded from quality measurement because:
1. Encoder state is not yet stable
2. Keyframe overhead skews quality metrics

```
Sample: [warmup frames | measured frames]
         ~~~~~~~~~~~~   ^^^^^^^^^^^^^^^
         (discarded)    (used for score)
```

### Final Encode

After the search converges using samples, the full chunk is encoded at the determined CRF:

```
Probe 1: sample → score → not converged
Probe 2: sample → score → not converged
Probe 3: sample → score → converged at CRF 28
Final:   full chunk at CRF 28 → output
```

## Complete Search Example

Target: 75-80 (midpoint=77.5, tolerance=2.5)
Chunk: 240 frames, QPMin=8, QPMax=48, predicted CRF=30

**Initialization:**
- SearchMin = max(8, 30-5) = 25
- SearchMax = min(48, 30+5) = 35

**Round 1** (binary search):
- CRF = (25+35)/2 = 30
- Encode → Score = 72.1 (too low)
- Update: SearchMax = 29

**Round 2** (binary search):
- CRF = (25+29)/2 = 27
- Encode → Score = 76.5 (in range!)
- Converged: |76.5 - 77.5| = 1.0 ≤ 2.5

**Result:** CRF 27, achieved in 2 rounds

## Relationship to Similar Tools

This algorithm is derived from [xav](https://github.com/Line-fr/xav), which pioneered GPU-accelerated SSIMULACRA2 for target quality encoding. Key differences:

| Aspect | xav | drapto |
|--------|-----|--------|
| Language | Rust | Go |
| GPU backend | VSHIP | VSHIP (via CGO) |
| Interpolation | Similar spline methods | Same progression: Lerp→FC→PCHIP→Akima |
| CRF prediction | Per-chunk | Tracker with k-nearest weighted average |
| Sample probing | Supported | Supported with configurable duration |

## Implementation Files

| File | Purpose |
|------|---------|
| `internal/tq/state.go` | State and Probe data structures |
| `internal/tq/search.go` | NextCRF, UpdateBounds, CheckComplete |
| `internal/tq/interp.go` | Lerp, FritschCarlson, PCHIP, Akima |
| `internal/tq/tracker.go` | Cross-chunk CRF prediction |
| `internal/tq/config.go` | Configuration parsing |
| `internal/encode/encode_tq.go` | Pipeline orchestration |
