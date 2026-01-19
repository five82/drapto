// Package processing provides video processing orchestration.
package processing

import (
	"context"
	"fmt"
	"os"
	"os/exec"
	"time"

	"github.com/five82/drapto/internal/chunk"
	"github.com/five82/drapto/internal/config"
	"github.com/five82/drapto/internal/encode"
	"github.com/five82/drapto/internal/ffms"
	"github.com/five82/drapto/internal/ffprobe"
	"github.com/five82/drapto/internal/keyframe"
	"github.com/five82/drapto/internal/reporter"
	"github.com/five82/drapto/internal/tq"
	"github.com/five82/drapto/internal/worker"
)

// ProcessChunked runs the chunked encoding pipeline for a single file.
func ProcessChunked(
	ctx context.Context,
	cfg *config.Config,
	inputPath, outputPath string,
	videoProps *ffprobe.VideoProperties,
	audioStreams []ffprobe.AudioStreamInfo,
	quality uint32,
	rep reporter.Reporter,
) error {
	// Create work directory
	workDir := chunk.GetWorkDirPath(inputPath, cfg.GetTempDir())
	if err := chunk.CreateWorkDir(workDir); err != nil {
		return fmt.Errorf("failed to create work directory: %w", err)
	}

	// Cleanup on completion (unless resuming a failed encode)
	defer func() {
		// Only cleanup if output was successfully created
		if _, err := os.Stat(outputPath); err == nil {
			_ = chunk.CleanupWorkDir(workDir)
		}
	}()

	// Initialize FFMS2 and create index
	rep.StageProgress(reporter.StageProgress{Stage: "Indexing", Message: "Creating video index"})
	idx, err := ffms.NewVidIdx(inputPath, true)
	if err != nil {
		return fmt.Errorf("failed to create video index: %w", err)
	}
	defer idx.Close()

	// Get video info
	vidInf, err := ffms.GetVidInf(idx)
	if err != nil {
		return fmt.Errorf("failed to get video info: %w", err)
	}

	// Detect scene changes
	rep.StageProgress(reporter.StageProgress{Stage: "Scene Detection", Message: "Detecting scene changes"})
	rep.Verbose(fmt.Sprintf("Scene threshold: %.2f", cfg.SceneThreshold))
	sceneFile, err := keyframe.ExtractKeyframesIfNeeded(
		inputPath,
		workDir,
		vidInf.FPSNum,
		vidInf.FPSDen,
		vidInf.Frames,
		cfg.SceneThreshold,
	)
	if err != nil {
		return fmt.Errorf("scene detection failed: %w", err)
	}

	// Load scenes
	scenes, err := chunk.LoadScenes(sceneFile, vidInf.Frames)
	if err != nil {
		return fmt.Errorf("failed to load scenes: %w", err)
	}
	rep.Verbose(fmt.Sprintf("Detected %d scenes", len(scenes)))

	// Convert scenes to chunks
	chunks := chunk.Chunkify(scenes)
	rep.StageProgress(reporter.StageProgress{Stage: "Chunking", Message: fmt.Sprintf("Split video into %d chunks", len(chunks))})

	// Calculate average chunk duration for verbose output
	fps := float64(vidInf.FPSNum) / float64(vidInf.FPSDen)
	totalFrames := 0
	for _, c := range chunks {
		totalFrames += int(c.End - c.Start)
	}
	avgChunkFrames := float64(totalFrames) / float64(len(chunks))
	avgChunkDuration := avgChunkFrames / fps
	rep.Verbose(fmt.Sprintf("Average chunk duration: %.1fs (%d frames)", avgChunkDuration, int(avgChunkFrames)))

	// Perform crop detection using existing drapto logic
	cropResult := DetectCrop(inputPath, videoProps, cfg.CropMode == "none")

	// Convert crop filter to cropH/cropV
	var cropH, cropV uint32
	if cropResult.Required && cropResult.CropFilter != "" {
		cropH, cropV = parseCropFilter(cropResult.CropFilter, videoProps.Width, videoProps.Height)
		rep.Verbose(fmt.Sprintf("Crop offsets: horizontal %d, vertical %d", cropH, cropV))
	}

	// Setup encode config
	encCfg := &encode.EncodeConfig{
		Workers:               cfg.Workers,
		ChunkBuffer:           cfg.ChunkBuffer,
		CRF:                   float32(quality),
		Preset:                cfg.SVTAV1Preset,
		Tune:                  cfg.SVTAV1Tune,
		ACBias:                cfg.SVTAV1ACBias,
		EnableVarianceBoost:   cfg.SVTAV1EnableVarianceBoost,
		VarianceBoostStrength: cfg.SVTAV1VarianceBoostStrength,
		VarianceOctile:        cfg.SVTAV1VarianceOctile,
		LowPriority:           cfg.ResponsiveEncoding,
	}

	// Run parallel encode
	rep.StageProgress(reporter.StageProgress{Stage: "Encoding", Message: fmt.Sprintf("Starting chunked encoding with %d workers", cfg.Workers)})

	rep.EncodingStarted(uint64(vidInf.Frames))

	startTime := time.Now()

	progressCallback := func(progress worker.Progress) {
		// Calculate speed and ETA
		elapsed := time.Since(startTime)
		var speed float32
		var eta time.Duration

		if elapsed.Seconds() > 0 && progress.FramesComplete > 0 {
			// Video seconds encoded
			videoSeconds := float64(progress.FramesComplete) / fps
			// Speed = video seconds per real second
			speed = float32(videoSeconds / elapsed.Seconds())

			// ETA based on remaining frames
			if speed > 0 {
				remainingFrames := progress.FramesTotal - progress.FramesComplete
				remainingVideoSeconds := float64(remainingFrames) / fps
				eta = time.Duration(remainingVideoSeconds/float64(speed)) * time.Second
			}
		}

		rep.EncodingProgress(reporter.ProgressSnapshot{
			CurrentFrame:   uint64(progress.FramesComplete),
			TotalFrames:    uint64(progress.FramesTotal),
			Percent:        float32(progress.Percent()),
			Speed:          speed,
			ETA:            eta,
			ChunksComplete: progress.ChunksComplete,
			ChunksTotal:    progress.ChunksTotal,
		})
	}

	// Use target quality pipeline if configured
	var encodeErr error
	if cfg.TargetQuality != "" {
		tqCfg, parseErr := tq.ParseTargetRange(cfg.TargetQuality)
		if parseErr != nil {
			return fmt.Errorf("invalid target quality: %w", parseErr)
		}

		// Parse QP range if specified
		if cfg.QPRange != "" {
			qpMin, qpMax, qpErr := tq.ParseQPRange(cfg.QPRange)
			if qpErr != nil {
				return fmt.Errorf("invalid QP range: %w", qpErr)
			}
			tqCfg.QPMin = qpMin
			tqCfg.QPMax = qpMax
		}

		tqCfg.MetricMode = cfg.MetricMode

		rep.Verbose(fmt.Sprintf("Target quality: SSIMULACRA2 %.0f-%.0f", tqCfg.TargetMin, tqCfg.TargetMax))
		rep.Verbose(fmt.Sprintf("CRF search range: %.0f-%.0f", tqCfg.QPMin, tqCfg.QPMax))
		rep.Verbose(fmt.Sprintf("Metric mode: %s, workers %d", cfg.MetricMode, cfg.MetricWorkers))

		tqEncCfg := &encode.TQEncodeConfig{
			EncodeConfig:      *encCfg,
			TQConfig:          tqCfg,
			MetricWorkers:     cfg.MetricWorkers,
			SampleDuration:    cfg.SampleDuration,
			SampleMinChunk:    cfg.SampleMinChunk,
			DisableTQSampling: cfg.DisableTQSampling,
			Verbose:           cfg.Verbose,
		}

		encodeErr = encode.EncodeAllTQ(
			ctx,
			chunks,
			vidInf,
			tqEncCfg,
			idx,
			workDir,
			cropH,
			cropV,
			progressCallback,
			rep,
		)
	} else {
		encodeErr = encode.EncodeAll(
			ctx,
			chunks,
			vidInf,
			encCfg,
			idx,
			workDir,
			cropH,
			cropV,
			progressCallback,
		)
	}

	if encodeErr != nil {
		return fmt.Errorf("chunked encoding failed: %w", encodeErr)
	}

	// Merge IVF files
	rep.StageProgress(reporter.StageProgress{Stage: "Merging", Message: "Merging encoded chunks"})
	if len(chunks) > 500 {
		// Use batched merge for large number of chunks
		if err := chunk.MergeBatched(workDir, len(chunks)); err != nil {
			return fmt.Errorf("batched merge failed: %w", err)
		}
	}

	if err := chunk.MergeOutput(workDir, outputPath, vidInf, inputPath); err != nil {
		return fmt.Errorf("video merge failed: %w", err)
	}

	// Extract and encode audio
	if len(audioStreams) > 0 {
		rep.StageProgress(reporter.StageProgress{Stage: "Audio", Message: "Extracting audio"})
		if err := chunk.ExtractAudio(inputPath, workDir, audioStreams); err != nil {
			return fmt.Errorf("audio extraction failed: %w", err)
		}
	}

	// Final mux
	rep.StageProgress(reporter.StageProgress{Stage: "Muxing", Message: "Creating final output"})
	if err := chunk.MuxFinal(inputPath, workDir, outputPath, audioStreams); err != nil {
		return fmt.Errorf("final mux failed: %w", err)
	}

	return nil
}

// parseCropFilter extracts cropH and cropV from a crop filter string.
// Format: "crop=W:H:X:Y" where X is left offset and Y is top offset.
func parseCropFilter(filter string, srcWidth, srcHeight uint32) (cropH, cropV uint32) {
	// Parse "crop=W:H:X:Y"
	var w, h, x, y uint32
	_, err := fmt.Sscanf(filter, "crop=%d:%d:%d:%d", &w, &h, &x, &y)
	if err != nil {
		return 0, 0
	}

	// cropH = X (horizontal offset from left)
	// cropV = Y (vertical offset from top)
	// These represent how many pixels are cropped from each side
	cropH = x
	cropV = y

	return cropH, cropV
}

// CheckChunkedDependencies verifies that required tools are available.
func CheckChunkedDependencies() error {
	// Check for SvtAv1EncApp in PATH
	if _, err := exec.LookPath("SvtAv1EncApp"); err != nil {
		return fmt.Errorf("SvtAv1EncApp not found in PATH (required for encoding)")
	}

	// Check for ffmpeg in PATH (used for scene detection)
	if _, err := exec.LookPath("ffmpeg"); err != nil {
		return fmt.Errorf("ffmpeg not found in PATH (required for scene detection)")
	}

	return nil
}
