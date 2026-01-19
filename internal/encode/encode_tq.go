// Package encode provides the parallel chunk encoding pipeline.
package encode

import (
	"context"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sync"
	"sync/atomic"
	"unsafe"

	"github.com/five82/drapto/internal/chunk"
	"github.com/five82/drapto/internal/encoder"
	"github.com/five82/drapto/internal/ffms"
	"github.com/five82/drapto/internal/reporter"
	"github.com/five82/drapto/internal/tq"
	"github.com/five82/drapto/internal/util"
	"github.com/five82/drapto/internal/vship"
	"github.com/five82/drapto/internal/worker"
)

// TQEncodeConfig contains configuration for target quality encoding.
type TQEncodeConfig struct {
	EncodeConfig          // Embed standard encode config
	TQConfig      *tq.Config
	MetricWorkers int
	// Sample-based probing configuration
	SampleDuration    float64 // Duration in seconds to sample for TQ probing
	SampleMinChunk    float64 // Minimum chunk duration to use sampling
	DisableTQSampling bool    // Disable sample-based probing (use full chunks)
}

// EncodeAllTQ runs the parallel encoding pipeline with target quality search.
func EncodeAllTQ(
	ctx context.Context,
	chunks []chunk.Chunk,
	inf *ffms.VidInf,
	cfg *TQEncodeConfig,
	idx *ffms.VidIdx,
	workDir string,
	cropH, cropV uint32,
	progressCb ProgressCallback,
	rep reporter.Reporter,
) error {
	// Ensure encode directory exists
	if err := chunk.EnsureEncodeDir(workDir); err != nil {
		return fmt.Errorf("failed to create encode directory: %w", err)
	}

	// Create split directory for probe files
	splitDir := filepath.Join(workDir, "split")
	if err := os.MkdirAll(splitDir, 0o755); err != nil {
		return fmt.Errorf("failed to create split directory: %w", err)
	}

	// Load resume information
	resume, err := chunk.GetResume(workDir)
	if err != nil {
		return fmt.Errorf("failed to load resume info: %w", err)
	}
	doneSet := resume.DoneSet()

	// Count remaining chunks
	remainingChunks := make([]chunk.Chunk, 0, len(chunks))
	totalFrames := 0
	for _, ch := range chunks {
		totalFrames += ch.Frames()
		if !doneSet[ch.Idx] {
			remainingChunks = append(remainingChunks, ch)
		}
	}

	if len(remainingChunks) == 0 {
		return nil // All chunks already done
	}

	// Determine decode strategy
	strat, cropCalc, err := ffms.GetDecodeStrat(idx, inf, cropH, cropV)
	if err != nil {
		return fmt.Errorf("failed to determine decode strategy: %w", err)
	}

	// Calculate effective dimensions
	width := inf.Width
	height := inf.Height
	if cropCalc != nil {
		width = cropCalc.NewW
		height = cropCalc.NewH
	}

	// Initialize VSHIP for GPU-accelerated metrics
	if err := vship.InitDevice(); err != nil {
		return fmt.Errorf("failed to initialize VSHIP: %w", err)
	}

	// Create video source
	src, err := ffms.ThrVidSrc(idx, cfg.Workers)
	if err != nil {
		return fmt.Errorf("failed to create video source: %w", err)
	}
	defer src.Close()

	// Setup semaphore for memory management
	// For TQ mode, limit in-flight to worker count so more chunks complete
	// before new ones dispatch, providing better CRF prediction data
	permits := cfg.Workers
	if permits < 1 {
		permits = 1
	}

	// Memory-based permit cap: prevent OOM by limiting in-flight YUV chunks
	// based on available system memory, independent of worker count.
	// Calculate estimated memory per in-flight chunk:
	// - YUV buffer: frames * frameSize
	// - SVT-AV1 encoder process: ~1 GB per instance
	avgFramesPerChunk := totalFrames / len(chunks)
	if avgFramesPerChunk < 1 {
		avgFramesPerChunk = 1
	}
	// Frame size for 10-bit YUV420: width * height * 1.5 * 2 bytes
	frameSize := uint64(width) * uint64(height) * 3 // 10-bit YUV420
	yuvMemBytes := frameSize * uint64(avgFramesPerChunk)
	encoderOverhead := uint64(1024 * 1024 * 1024) // ~1 GB per SVT-AV1 process
	chunkMemBytes := yuvMemBytes + encoderOverhead

	// Cap permits to use at most 50% of available memory. This is conservative
	// to leave headroom for OS file caches (probe files), memory fragmentation,
	// and other system processes. Empirical testing showed 70% was too aggressive.
	memPermits := util.MaxPermitsForMemory(chunkMemBytes, 0.5)
	if memPermits < permits {
		rep.Verbose(fmt.Sprintf("Memory cap: limiting permits from %d to %d (chunk: %d MB, available: %d MB)",
			permits, memPermits, chunkMemBytes/(1024*1024), util.AvailableMemoryBytes()/(1024*1024)))
		permits = memPermits
	}

	sem := worker.NewSemaphore(permits)

	// Create dispatcher and tracker for adaptive CRF prediction
	dispatcher := chunk.NewDispatcher(remainingChunks)
	tracker := tq.NewTracker()

	// Gradual ramp-up: start with limited parallelism and increase as chunks complete.
	// This ensures early chunks provide CRF prediction data for later chunks.
	// - Start with 2 in-flight chunks
	// - Add 2 more permits per completion until reaching full capacity
	const rampStart = 2      // Initial number of chunks to dispatch
	const rampIncrement = 2  // How many permits to add per completion during ramp-up
	var rampLimit atomic.Int32
	rampLimit.Store(int32(rampStart))
	rampChan := make(chan struct{}, permits) // Signals when ramp limit increases

	// Channels for the TQ pipeline
	// Use small fixed buffer sizes (like xav) to limit memory - prevents decoder
	// from filling channels with decoded YUV data faster than encoders consume it.
	// Even with many workers, only a few items can queue at each stage.
	const chanBuffer = 2
	encodeChan := make(chan *worker.WorkPkg, chanBuffer)
	metricsChan := make(chan *worker.WorkPkg, chanBuffer)
	reworkChan := make(chan *worker.WorkPkg, chanBuffer)
	doneChan := make(chan tqResult, len(remainingChunks))

	// Progress tracking
	var progressMu sync.Mutex
	progress := worker.Progress{
		ChunksTotal:    len(chunks),
		ChunksComplete: len(chunks) - len(remainingChunks),
		FramesTotal:    totalFrames,
		FramesComplete: resume.TotalEncodedFrames(),
		BytesComplete:  resume.TotalEncodedSize(),
	}

	// Error handling
	var encodeErr atomic.Pointer[error]
	setError := func(err error) {
		encodeErr.CompareAndSwap(nil, &err)
	}
	getError := func() error {
		if p := encodeErr.Load(); p != nil {
			return *p
		}
		return nil
	}

	// Start encoder workers
	var encoderWg sync.WaitGroup
	for i := 0; i < cfg.Workers; i++ {
		encoderWg.Add(1)
		go func() {
			defer encoderWg.Done()
			tqEncodeWorker(ctx, encodeChan, metricsChan, cfg, inf, workDir, splitDir, width, height, getError)
		}()
	}

	// Start metrics workers
	var metricsWg sync.WaitGroup
	for i := 0; i < cfg.MetricWorkers; i++ {
		metricsWg.Add(1)
		go func() {
			defer metricsWg.Done()
			tqMetricsWorker(ctx, metricsChan, reworkChan, doneChan, cfg, inf, splitDir, width, height, getError, rep)
		}()
	}

	// Start coordinator goroutine (handles rework and done)
	var coordWg sync.WaitGroup
	coordWg.Add(1)
	go func() {
		defer coordWg.Done()
		tqCoordinator(ctx, reworkChan, encodeChan, doneChan, sem, workDir, &progressMu, &progress, progressCb, len(remainingChunks), getError, dispatcher, tracker, permits, rampIncrement, &rampLimit, rampChan, rep)
	}()

	// Default CRF for first chunk (midpoint of range)
	defaultCRF := (cfg.TQConfig.QPMin + cfg.TQConfig.QPMax) / 2

	// Decoder goroutine
	// NOTE: Decoder does NOT close encodeChan - the coordinator owns that
	// because the coordinator may need to re-queue work after decoder finishes
	go func() {
		dispatched := 0
		for {
			// Check for cancellation
			select {
			case <-ctx.Done():
				return
			default:
			}

			// Check for error
			if getError() != nil {
				return
			}

			// Gradual ramp-up: wait if we've hit the current ramp limit
			for dispatched >= int(rampLimit.Load()) && dispatched < permits {
				select {
				case <-rampChan:
					// Ramp limit increased, check again
				case <-ctx.Done():
					return
				}
			}

			// Get next chunk from dispatcher (picks chunk nearest to completed ones)
			ch, ok := dispatcher.Next()
			if !ok {
				return // No more chunks
			}

			// Acquire semaphore
			select {
			case <-sem.Chan():
			case <-ctx.Done():
				return
			}

			dispatched++

			// Decode chunk frames
			pkg, err := decodeChunk(src, ch, inf, strat, cropCalc, width, height)
			if err != nil {
				setError(fmt.Errorf("failed to decode chunk %d: %w", ch.Idx, err))
				sem.Release()
				return
			}

			// Calculate sample parameters for TQ probing
			fps := float64(inf.FPSNum) / float64(inf.FPSDen)
			var offset, encodeFrames, warmupFrames, measureFrames int
			var useSampling bool
			if cfg.DisableTQSampling {
				// Sampling disabled - use full chunk for probing
				useSampling = false
				encodeFrames = pkg.FrameCount
				measureFrames = pkg.FrameCount
			} else {
				offset, encodeFrames, warmupFrames, measureFrames, useSampling = worker.CalculateSample(
					pkg.FrameCount, fps, cfg.SampleDuration, cfg.SampleMinChunk,
				)
			}

			pkg.UseSampling = useSampling
			pkg.SampleOffset = offset
			pkg.SampleFrameCount = encodeFrames
			pkg.WarmupFrames = warmupFrames
			pkg.MeasureFrameCount = measureFrames

			if useSampling {
				// Extract sample YUV from full chunk
				// Note: YUV is always 10-bit after FFMS2 decoding (2 bytes per sample)
				const pixelSize = 2
				ySize := int(width) * int(height) * pixelSize
				uvSize := ySize / 4
				frameSize := ySize + 2*uvSize

				sampleStart := offset * frameSize
				sampleEnd := (offset + encodeFrames) * frameSize
				pkg.SampleYUV = pkg.YUV[sampleStart:sampleEnd]

				rep.Verbose(fmt.Sprintf("Chunk %d: using %d-frame sample (%.1fs) from offset %d, warmup=%d, measure=%d",
					ch.Idx, encodeFrames, float64(encodeFrames)/fps, offset, warmupFrames, measureFrames))
			} else {
				chunkDur := float64(pkg.FrameCount) / fps
				if chunkDur < cfg.SampleMinChunk {
					rep.Verbose(fmt.Sprintf("Chunk %d: using full %d-frame chunk (%.1fs < min %.1fs)",
						ch.Idx, pkg.FrameCount, chunkDur, cfg.SampleMinChunk))
				} else {
					rep.Verbose(fmt.Sprintf("Chunk %d: using full %d-frame chunk (sample would exceed half of %d frames)",
						ch.Idx, pkg.FrameCount, pkg.FrameCount))
				}
			}

			// Get CRF prediction from nearby completed chunks
			predictedCRF := tracker.Predict(ch.Idx, defaultCRF)

			rep.Verbose(fmt.Sprintf("Chunk %d: predicted CRF=%.1f (from %d completed chunks), search bounds [%.0f, %.0f]",
				ch.Idx, predictedCRF, tracker.Count(),
				max(cfg.TQConfig.QPMin, predictedCRF-5),
				min(cfg.TQConfig.QPMax, predictedCRF+5)))

			// Initialize TQ state with predicted CRF (narrows search bounds)
			pkg.TQState = tq.NewState(cfg.TQConfig.Target, cfg.TQConfig.QPMin, cfg.TQConfig.QPMax, predictedCRF)

			// Send to encode channel
			select {
			case encodeChan <- pkg:
			case <-ctx.Done():
				sem.Release()
				return
			}
		}
	}()

	// Wait for coordinator first - it closes encodeChan when all work is done
	// (including rework cycles). This must happen before waiting on encoders.
	coordWg.Wait()

	// Now encoders can finish (encodeChan is closed)
	encoderWg.Wait()
	close(metricsChan)

	// Wait for metrics workers
	metricsWg.Wait()
	close(reworkChan)
	close(doneChan)

	return getError()
}

// tqResult contains the result of a completed TQ chunk.
type tqResult struct {
	ChunkIdx     int
	Frames       int
	Size         uint64
	FinalCRF     float64
	FinalScore   float64
	Round        int
	Probes       []tq.ProbeEntry
	Error        error
	UsedSampling bool // Whether sample-based probing was used (final encode already done)
}

// tqEncodeWorker encodes chunks at the CRF determined by TQ search.
func tqEncodeWorker(
	ctx context.Context,
	workChan <-chan *worker.WorkPkg,
	metricsChan chan<- *worker.WorkPkg,
	cfg *TQEncodeConfig,
	inf *ffms.VidInf,
	workDir, splitDir string,
	width, height uint32,
	getError func() error,
) {
	for pkg := range workChan {
		// Check for cancellation
		select {
		case <-ctx.Done():
			return
		default:
		}

		if getError() != nil {
			return
		}

		// Determine next CRF to try
		crf := tq.NextCRF(pkg.TQState)

		// Encode probe at this CRF
		probePath := filepath.Join(splitDir, fmt.Sprintf("%04d_%.2f.ivf", pkg.Chunk.Idx, crf))
		if err := encodeProbe(pkg, crf, cfg, inf, probePath, width, height); err != nil {
			// TODO: handle error properly
			continue
		}

		// Send to metrics channel
		select {
		case metricsChan <- pkg:
		case <-ctx.Done():
			return
		}
	}
}

// tqMetricsWorker computes SSIMULACRA2 scores and decides convergence.
func tqMetricsWorker(
	ctx context.Context,
	metricsChan <-chan *worker.WorkPkg,
	reworkChan chan<- *worker.WorkPkg,
	doneChan chan<- tqResult,
	cfg *TQEncodeConfig,
	inf *ffms.VidInf,
	splitDir string,
	width, height uint32,
	getError func() error,
	rep reporter.Reporter,
) {
	// Initialize VSHIP processor for this worker
	var proc *vship.Processor
	defer func() {
		if proc != nil {
			_ = proc.Close()
		}
	}()

	for pkg := range metricsChan {
		// Check for cancellation
		select {
		case <-ctx.Done():
			return
		default:
		}

		if getError() != nil {
			return
		}

		// Initialize processor lazily
		if proc == nil {
			var err error
			proc, err = vship.NewProcessor(
				width, height, inf.Is10Bit,
				int32PtrToIntPtr(inf.MatrixCoefficients),
				int32PtrToIntPtr(inf.TransferCharacteristics),
				int32PtrToIntPtr(inf.ColorPrimaries),
				nil, // ColorRange not available in VidInf
				nil, // ChromaSamplePosition not available in VidInf
			)
			if err != nil {
				doneChan <- tqResult{
					ChunkIdx: pkg.Chunk.Idx,
					Error:    fmt.Errorf("failed to create VSHIP processor: %w", err),
				}
				continue
			}
		}

		// Compute metrics
		crf := pkg.TQState.LastCRF
		probePath := filepath.Join(splitDir, fmt.Sprintf("%04d_%.2f.ivf", pkg.Chunk.Idx, crf))

		score, frameScores, size, err := computeMetrics(pkg, probePath, proc, width, height)
		if err != nil {
			doneChan <- tqResult{
				ChunkIdx: pkg.Chunk.Idx,
				Error:    fmt.Errorf("failed to compute metrics: %w", err),
			}
			continue
		}

		// Record probe
		pkg.TQState.AddProbe(crf, score, frameScores, size)

		// Check if we should complete
		if tq.ShouldComplete(pkg.TQState, score, cfg.TQConfig) {
			best := pkg.TQState.BestProbe()
			if best == nil {
				best = &pkg.TQState.Probes[len(pkg.TQState.Probes)-1]
			}

			// Build probe entries for logging
			probeEntries := make([]tq.ProbeEntry, len(pkg.TQState.Probes))
			for i, p := range pkg.TQState.Probes {
				probeEntries[i] = tq.ProbeEntry{CRF: p.CRF, Score: p.Score, Size: p.Size}
			}

			finalCRF := best.CRF
			finalSize := best.Size

			// When sampling was used, encode the full chunk at the determined CRF
			if pkg.UseSampling {
				// Derive workDir from splitDir (splitDir = workDir/split)
				workDir := filepath.Dir(splitDir)
				finalPath := chunk.IVFPath(workDir, pkg.Chunk.Idx)

				rep.Verbose(fmt.Sprintf("Chunk %d: sample probing complete at CRF=%.0f, encoding full chunk", pkg.Chunk.Idx, finalCRF))

				if err := encodeFinal(pkg, finalCRF, cfg, inf, finalPath, width, height); err != nil {
					doneChan <- tqResult{
						ChunkIdx: pkg.Chunk.Idx,
						Error:    fmt.Errorf("failed to encode final chunk: %w", err),
					}
					continue
				}

				// Get actual file size from final encode
				stat, err := os.Stat(finalPath)
				if err != nil {
					doneChan <- tqResult{
						ChunkIdx: pkg.Chunk.Idx,
						Error:    fmt.Errorf("failed to stat final chunk: %w", err),
					}
					continue
				}
				finalSize = uint64(stat.Size())
			}

			doneChan <- tqResult{
				ChunkIdx:    pkg.Chunk.Idx,
				Frames:      pkg.FrameCount,
				Size:        finalSize,
				FinalCRF:    finalCRF,
				FinalScore:  best.Score,
				Round:       pkg.TQState.Round,
				Probes:      probeEntries,
				UsedSampling: pkg.UseSampling,
			}

			// Clear YUV data to free memory
			pkg.YUV = nil
			pkg.SampleYUV = nil
		} else {
			// Need more iterations
			select {
			case reworkChan <- pkg:
			case <-ctx.Done():
				return
			}
		}
	}
}

// tqCoordinator manages the feedback loop and progress reporting.
func tqCoordinator(
	ctx context.Context,
	reworkChan <-chan *worker.WorkPkg,
	encodeChan chan<- *worker.WorkPkg,
	doneChan <-chan tqResult,
	sem *worker.Semaphore,
	workDir string,
	progressMu *sync.Mutex,
	progress *worker.Progress,
	progressCb ProgressCallback,
	totalRemaining int,
	getError func() error,
	dispatcher *chunk.Dispatcher,
	tracker *tq.CRFTracker,
	maxPermits int,
	rampIncrement int,
	rampLimit *atomic.Int32,
	rampChan chan<- struct{},
	rep reporter.Reporter,
) {
	// Coordinator owns closing encodeChan - it knows when all work is complete
	// (including rework cycles). Decoder cannot close it because rework may
	// still be needed after decoder finishes sending initial chunks.
	defer close(encodeChan)

	completed := 0

	for completed < totalRemaining {
		// Check for errors from other goroutines
		if getError() != nil {
			return
		}
		select {
		case <-ctx.Done():
			return

		case pkg, ok := <-reworkChan:
			if !ok {
				continue
			}
			// Re-queue for encoding
			select {
			case encodeChan <- pkg:
			case <-ctx.Done():
				return
			}

		case result, ok := <-doneChan:
			if !ok {
				continue
			}
			completed++

			if result.Error != nil {
				continue
			}

			// Record completion for adaptive CRF prediction
			dispatcher.MarkComplete(result.ChunkIdx)
			tracker.Record(result.ChunkIdx, result.FinalCRF)

			rep.Verbose(fmt.Sprintf("Chunk %d complete: CRF=%.0f, score=%.1f, %d iterations",
				result.ChunkIdx, result.FinalCRF, result.FinalScore, result.Round))

			// Gradual ramp-up: increase dispatch limit as chunks complete
			currentLimit := int(rampLimit.Load())
			if currentLimit < maxPermits {
				newLimit := min(currentLimit+rampIncrement, maxPermits)
				rampLimit.Store(int32(newLimit))
				rep.Verbose(fmt.Sprintf("Ramp-up: increased dispatch limit to %d", newLimit))
				// Signal decoder that limit increased (non-blocking)
				select {
				case rampChan <- struct{}{}:
				default:
				}
			}

			// Copy best probe to final output (skip if sampling already encoded to final path)
			if !result.UsedSampling {
				bestPath := filepath.Join(workDir, "split", fmt.Sprintf("%04d_%.2f.ivf", result.ChunkIdx, result.FinalCRF))
				finalPath := chunk.IVFPath(workDir, result.ChunkIdx)
				if err := copyFile(bestPath, finalPath); err != nil {
					// Log error but continue
					continue
				}
			}

			// Update resume info
			_ = chunk.AppendDone(chunk.ChunkComp{
				Idx:    result.ChunkIdx,
				Frames: result.Frames,
				Size:   result.Size,
			}, workDir)

			// Release semaphore
			sem.Release()

			// Update progress
			progressMu.Lock()
			progress.ChunksComplete++
			progress.FramesComplete += result.Frames
			progress.BytesComplete += result.Size
			p := *progress
			progressMu.Unlock()

			if progressCb != nil {
				progressCb(p)
			}
		}
	}
}

// encodeProbe encodes a chunk at a specific CRF value.
// When sampling is enabled, only the sample portion is encoded for probing.
func encodeProbe(
	pkg *worker.WorkPkg,
	crf float64,
	cfg *TQEncodeConfig,
	inf *ffms.VidInf,
	outputPath string,
	width, height uint32,
) error {
	// Use sample data for probing if sampling is enabled
	var yuvData []byte
	var frameCount int
	if pkg.UseSampling && pkg.SampleYUV != nil {
		yuvData = pkg.SampleYUV
		frameCount = pkg.SampleFrameCount
	} else {
		yuvData = pkg.YUV
		frameCount = pkg.FrameCount
	}

	encCfg := &encoder.EncConfig{
		Inf:                   inf,
		CRF:                   float32(crf),
		Preset:                cfg.Preset,
		Tune:                  cfg.Tune,
		Output:                outputPath,
		GrainTable:            cfg.GrainTable,
		Width:                 width,
		Height:                height,
		Frames:                frameCount,
		ACBias:                cfg.ACBias,
		EnableVarianceBoost:   cfg.EnableVarianceBoost,
		VarianceBoostStrength: cfg.VarianceBoostStrength,
		VarianceOctile:        cfg.VarianceOctile,
		LowPriority:           cfg.LowPriority,
	}

	cmd := encoder.MakeSvtCmd(encCfg)

	stdin, err := cmd.StdinPipe()
	if err != nil {
		return fmt.Errorf("failed to create stdin pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to start encoder: %w", err)
	}

	_, err = io.Copy(stdin, &yuvReader{data: yuvData})
	_ = stdin.Close()

	if err != nil {
		_ = cmd.Wait()
		return fmt.Errorf("failed to write YUV data: %w", err)
	}

	if err := cmd.Wait(); err != nil {
		return fmt.Errorf("encoder failed: %w", err)
	}

	return nil
}

// encodeFinal encodes the full chunk at the determined CRF value.
// This is called after probing determines the optimal CRF.
func encodeFinal(
	pkg *worker.WorkPkg,
	crf float64,
	cfg *TQEncodeConfig,
	inf *ffms.VidInf,
	outputPath string,
	width, height uint32,
) error {
	encCfg := &encoder.EncConfig{
		Inf:                   inf,
		CRF:                   float32(crf),
		Preset:                cfg.Preset,
		Tune:                  cfg.Tune,
		Output:                outputPath,
		GrainTable:            cfg.GrainTable,
		Width:                 width,
		Height:                height,
		Frames:                pkg.FrameCount,
		ACBias:                cfg.ACBias,
		EnableVarianceBoost:   cfg.EnableVarianceBoost,
		VarianceBoostStrength: cfg.VarianceBoostStrength,
		VarianceOctile:        cfg.VarianceOctile,
		LowPriority:           cfg.LowPriority,
	}

	cmd := encoder.MakeSvtCmd(encCfg)

	stdin, err := cmd.StdinPipe()
	if err != nil {
		return fmt.Errorf("failed to create stdin pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to start encoder: %w", err)
	}

	_, err = io.Copy(stdin, &yuvReader{data: pkg.YUV})
	_ = stdin.Close()

	if err != nil {
		_ = cmd.Wait()
		return fmt.Errorf("failed to write YUV data: %w", err)
	}

	if err := cmd.Wait(); err != nil {
		return fmt.Errorf("encoder failed: %w", err)
	}

	return nil
}

// computeMetrics computes SSIMULACRA2 scores by comparing source YUV to encoded output.
// When sampling is enabled, only the measured frames (after warmup) are compared.
func computeMetrics(
	pkg *worker.WorkPkg,
	probePath string,
	proc *vship.Processor,
	width, height uint32,
) (score float64, frameScores []float64, size uint64, err error) {
	// Get file size
	stat, err := os.Stat(probePath)
	if err != nil {
		return 0, nil, 0, fmt.Errorf("failed to stat probe file: %w", err)
	}
	size = uint64(stat.Size())

	// Create index for encoded file
	probeIdx, err := ffms.NewVidIdx(probePath, false)
	if err != nil {
		return 0, nil, 0, fmt.Errorf("failed to index probe file: %w", err)
	}
	defer probeIdx.Close()

	// Create video source for encoded file
	probeSrc, err := ffms.ThrVidSrc(probeIdx, 1)
	if err != nil {
		return 0, nil, 0, fmt.Errorf("failed to create probe video source: %w", err)
	}
	defer probeSrc.Close()

	// Calculate frame dimensions
	// Note: pkg.YUV always contains 10-bit data (2 bytes per sample) because
	// FFMS2 converts 8-bit sources to 10-bit. The inf.Is10Bit flag reflects
	// the original source bit depth, not the pkg.YUV format.
	const pixelSize = 2 // Always 10-bit (16-bit per sample)
	ySize := int(width) * int(height) * pixelSize
	uvSize := ySize / 4
	frameSize := ySize + 2*uvSize

	// Determine frames to measure
	var measureCount int
	var srcFrameOffset int // Offset in source YUV (full chunk)
	var encFrameOffset int // Offset in encoded probe
	if pkg.UseSampling && pkg.SampleYUV != nil {
		// Skip warmup frames when measuring
		measureCount = pkg.MeasureFrameCount
		srcFrameOffset = pkg.SampleOffset + pkg.WarmupFrames
		encFrameOffset = pkg.WarmupFrames
	} else {
		measureCount = pkg.FrameCount
		srcFrameOffset = 0
		encFrameOffset = 0
	}

	frameScores = make([]float64, measureCount)
	var total float64

	for i := 0; i < measureCount; i++ {
		// Get source frame from full chunk YUV
		srcIdx := srcFrameOffset + i
		srcOffset := srcIdx * frameSize
		srcY := unsafe.Pointer(&pkg.YUV[srcOffset])
		srcU := unsafe.Pointer(&pkg.YUV[srcOffset+ySize])
		srcV := unsafe.Pointer(&pkg.YUV[srcOffset+ySize+uvSize])

		srcPlanes := [3]unsafe.Pointer{srcY, srcU, srcV}
		srcStrides := [3]int64{
			int64(width) * int64(pixelSize),
			int64(width) / 2 * int64(pixelSize),
			int64(width) / 2 * int64(pixelSize),
		}

		// Get encoded frame (skip warmup frames if sampling)
		encIdx := encFrameOffset + i
		frame, err := ffms.GetFrame(probeSrc, encIdx)
		if err != nil {
			return 0, nil, 0, fmt.Errorf("failed to get frame %d: %w", encIdx, err)
		}

		disPlanes := [3]unsafe.Pointer{frame.Data[0], frame.Data[1], frame.Data[2]}
		disStrides := [3]int64{int64(frame.Linesize[0]), int64(frame.Linesize[1]), int64(frame.Linesize[2])}

		// Compute SSIMULACRA2
		s, err := proc.ComputeSSIMULACRA2(srcPlanes, disPlanes, srcStrides, disStrides)
		if err != nil {
			return 0, nil, 0, fmt.Errorf("failed to compute SSIMULACRA2 for frame %d: %w", encIdx, err)
		}

		frameScores[i] = s
		total += s
	}

	score = total / float64(measureCount)
	return score, frameScores, size, nil
}

// copyFile copies a file from src to dst.
func copyFile(src, dst string) error {
	in, err := os.Open(src)
	if err != nil {
		return err
	}
	defer func() { _ = in.Close() }()

	out, err := os.Create(dst)
	if err != nil {
		return err
	}
	defer func() { _ = out.Close() }()

	_, err = io.Copy(out, in)
	if err != nil {
		return err
	}

	return out.Close()
}

// int32PtrToIntPtr converts *int32 to *int, returning nil for nil input.
func int32PtrToIntPtr(v *int32) *int {
	if v == nil {
		return nil
	}
	val := int(*v)
	return &val
}
