// Package encode provides the parallel chunk encoding pipeline.
package encode

import (
	"context"
	"fmt"
	"io"
	"os"
	"sync"

	"github.com/five82/drapto/internal/chunk"
	"github.com/five82/drapto/internal/encoder"
	"github.com/five82/drapto/internal/ffms"
	"github.com/five82/drapto/internal/worker"
)

// EncodeConfig contains configuration for the parallel encode pipeline.
type EncodeConfig struct {
	Workers     int     // Number of parallel encoder workers
	ChunkBuffer int     // Extra chunks to buffer in memory
	CRF         float32 // Quality (CRF value)
	Preset      uint8   // SVT-AV1 preset
	Tune        uint8   // SVT-AV1 tune
	GrainTable  *string // Optional film grain table path

	// Advanced SVT-AV1 parameters
	ACBias                float32
	EnableVarianceBoost   bool
	VarianceBoostStrength uint8
	VarianceOctile        uint8
	LogicalProcessors     *uint32 // Optional limit on CPU threads per encoder
}

// ProgressCallback is called to report encoding progress.
type ProgressCallback func(progress worker.Progress)

// EncodeAll runs the parallel encoding pipeline.
func EncodeAll(
	ctx context.Context,
	chunks []chunk.Chunk,
	inf *ffms.VidInf,
	cfg *EncodeConfig,
	idx *ffms.VidIdx,
	workDir string,
	cropH, cropV uint32,
	progressCb ProgressCallback,
) error {
	// Ensure encode directory exists
	if err := chunk.EnsureEncodeDir(workDir); err != nil {
		return fmt.Errorf("failed to create encode directory: %w", err)
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

	// Create video source
	src, err := ffms.ThrVidSrc(idx, cfg.Workers)
	if err != nil {
		return fmt.Errorf("failed to create video source: %w", err)
	}
	defer src.Close()

	// Setup semaphore for memory management
	// Permits = workers + buffer
	permits := cfg.Workers + cfg.ChunkBuffer
	if permits < 1 {
		permits = 1
	}
	sem := worker.NewSemaphore(permits)

	// Work channel
	workChan := make(chan *worker.WorkPkg, permits)

	// Results channel
	resultChan := make(chan worker.EncodeResult, len(remainingChunks))

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
	var encodeErr error
	var errOnce sync.Once

	// Start encoder workers
	var encoderWg sync.WaitGroup
	for i := 0; i < cfg.Workers; i++ {
		encoderWg.Add(1)
		go func(workerID int) {
			defer encoderWg.Done()
			encodeWorker(ctx, workChan, resultChan, sem, cfg, inf, workDir, width, height)
		}(i)
	}

	// Start result collector
	var collectorWg sync.WaitGroup
	collectorWg.Add(1)
	go func() {
		defer collectorWg.Done()
		for result := range resultChan {
			if result.Error != nil {
				errOnce.Do(func() {
					encodeErr = result.Error
				})
				continue
			}

			// Update progress
			progressMu.Lock()
			progress.ChunksComplete++
			progress.FramesComplete += result.Frames
			progress.BytesComplete += result.Size
			progressMu.Unlock()

			// Append to done file (ignore errors, resume will handle incomplete state)
			_ = chunk.AppendDone(chunk.ChunkComp{
				Idx:    result.ChunkIdx,
				Frames: result.Frames,
				Size:   result.Size,
			}, workDir)

			// Report progress
			if progressCb != nil {
				progressMu.Lock()
				p := progress
				progressMu.Unlock()
				progressCb(p)
			}
		}
	}()

	// Decoder goroutine
	go func() {
		defer close(workChan)

		for _, ch := range remainingChunks {
			// Check for cancellation
			select {
			case <-ctx.Done():
				return
			default:
			}

			// Check for error
			if encodeErr != nil {
				return
			}

			// Acquire semaphore (blocks if too many chunks in flight)
			sem.Acquire()

			// Decode chunk frames
			pkg, err := decodeChunk(src, ch, inf, strat, cropCalc, width, height)
			if err != nil {
				errOnce.Do(func() {
					encodeErr = fmt.Errorf("failed to decode chunk %d: %w", ch.Idx, err)
				})
				sem.Release()
				return
			}

			workChan <- pkg
		}
	}()

	// Wait for encoders to finish
	encoderWg.Wait()
	close(resultChan)

	// Wait for result collector
	collectorWg.Wait()

	return encodeErr
}

// decodeChunk extracts all frames for a chunk.
func decodeChunk(
	src *ffms.VidSrc,
	ch chunk.Chunk,
	inf *ffms.VidInf,
	strat ffms.DecodeStrat,
	cropCalc *ffms.CropCalc,
	width, height uint32,
) (*worker.WorkPkg, error) {
	frameCount := ch.Frames()
	frameSize := ffms.CalcFrameSize(inf, cropCalc)
	totalSize := frameSize * frameCount

	// Allocate buffer for all frames
	yuv := make([]byte, totalSize)

	// Extract each frame
	for i := 0; i < frameCount; i++ {
		frameIdx := ch.Start + i
		offset := i * frameSize

		if err := ffms.ExtractFrame(src, frameIdx, yuv[offset:offset+frameSize], inf, strat, cropCalc); err != nil {
			return nil, fmt.Errorf("failed to extract frame %d: %w", frameIdx, err)
		}
	}

	return &worker.WorkPkg{
		Chunk:      ch,
		YUV:        yuv,
		FrameCount: frameCount,
		Width:      width,
		Height:     height,
		Is10Bit:    inf.Is10Bit,
	}, nil
}

// encodeWorker runs in a goroutine and encodes work packages.
func encodeWorker(
	ctx context.Context,
	workChan <-chan *worker.WorkPkg,
	resultChan chan<- worker.EncodeResult,
	sem *worker.Semaphore,
	cfg *EncodeConfig,
	inf *ffms.VidInf,
	workDir string,
	width, height uint32,
) {
	for pkg := range workChan {
		// Check for cancellation
		select {
		case <-ctx.Done():
			sem.Release()
			resultChan <- worker.EncodeResult{
				ChunkIdx: pkg.Chunk.Idx,
				Error:    ctx.Err(),
			}
			continue
		default:
		}

		// Encode the chunk
		result := encodeChunk(pkg, cfg, inf, workDir, width, height)

		// Free memory IMMEDIATELY after encoding
		pkg.YUV = nil

		// Release semaphore
		sem.Release()

		// Send result
		resultChan <- result
	}
}

// encodeChunk encodes a single work package.
func encodeChunk(
	pkg *worker.WorkPkg,
	cfg *EncodeConfig,
	inf *ffms.VidInf,
	workDir string,
	width, height uint32,
) worker.EncodeResult {
	outputPath := chunk.IVFPath(workDir, pkg.Chunk.Idx)

	encCfg := &encoder.EncConfig{
		Inf:                   inf,
		CRF:                   cfg.CRF,
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
		LogicalProcessors:     cfg.LogicalProcessors,
	}

	cmd := encoder.MakeSvtCmd(encCfg)

	// Setup stdin pipe
	stdin, err := cmd.StdinPipe()
	if err != nil {
		return worker.EncodeResult{
			ChunkIdx: pkg.Chunk.Idx,
			Error:    fmt.Errorf("failed to create stdin pipe: %w", err),
		}
	}

	// Start encoder
	if err := cmd.Start(); err != nil {
		return worker.EncodeResult{
			ChunkIdx: pkg.Chunk.Idx,
			Error:    fmt.Errorf("failed to start encoder: %w", err),
		}
	}

	// Write YUV data to encoder
	_, err = io.Copy(stdin, &yuvReader{data: pkg.YUV})
	_ = stdin.Close()

	if err != nil {
		_ = cmd.Wait()
		return worker.EncodeResult{
			ChunkIdx: pkg.Chunk.Idx,
			Error:    fmt.Errorf("failed to write YUV data: %w", err),
		}
	}

	// Wait for encoder to finish
	if err := cmd.Wait(); err != nil {
		return worker.EncodeResult{
			ChunkIdx: pkg.Chunk.Idx,
			Error:    fmt.Errorf("encoder failed: %w", err),
		}
	}

	// Get output file size
	stat, err := os.Stat(outputPath)
	if err != nil {
		return worker.EncodeResult{
			ChunkIdx: pkg.Chunk.Idx,
			Error:    fmt.Errorf("failed to stat output: %w", err),
		}
	}

	return worker.EncodeResult{
		ChunkIdx: pkg.Chunk.Idx,
		Frames:   pkg.FrameCount,
		Size:     uint64(stat.Size()),
	}
}

// yuvReader wraps a byte slice to implement io.Reader.
type yuvReader struct {
	data []byte
	pos  int
}

func (r *yuvReader) Read(p []byte) (n int, err error) {
	if r.pos >= len(r.data) {
		return 0, io.EOF
	}
	n = copy(p, r.data[r.pos:])
	r.pos += n
	return n, nil
}
