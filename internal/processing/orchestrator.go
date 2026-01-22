package processing

import (
	"context"
	"fmt"
	"time"

	"github.com/five82/drapto/internal/config"
	"github.com/five82/drapto/internal/ffmpeg"
	"github.com/five82/drapto/internal/ffprobe"
	"github.com/five82/drapto/internal/mediainfo"
	"github.com/five82/drapto/internal/reporter"
	"github.com/five82/drapto/internal/util"
	"github.com/five82/drapto/internal/validation"
)

// EncodeResult contains the result of a single file encode.
type EncodeResult struct {
	Filename          string
	Duration          time.Duration
	InputSize         uint64
	OutputSize        uint64
	VideoDurationSecs float64
	EncodingSpeed     float32
	ValidationPassed  bool
	ValidationSteps   []validation.ValidationStep
}

// ProcessVideos orchestrates encoding for a list of video files.
func ProcessVideos(
	ctx context.Context,
	cfg *config.Config,
	filesToProcess []string,
	targetFilenameOverride string,
	rep reporter.Reporter,
) ([]EncodeResult, error) {
	if rep == nil {
		rep = reporter.NullReporter{}
	}

	var results []EncodeResult

	// Emit hardware information
	sysInfo := util.GetSystemInfo()
	rep.Hardware(reporter.HardwareSummary{
		Hostname: sysInfo.Hostname,
	})

	// Show batch initialization for multiple files
	if len(filesToProcess) > 1 {
		var fileNames []string
		for _, f := range filesToProcess {
			fileNames = append(fileNames, util.GetFilename(f))
		}
		rep.BatchStarted(reporter.BatchStartInfo{
			TotalFiles: len(filesToProcess),
			FileList:   fileNames,
			OutputDir:  cfg.OutputDir,
		})
	}

	for fileIdx, inputPath := range filesToProcess {
		// Check for cancellation before starting each file
		if ctx.Err() != nil {
			rep.Warning(fmt.Sprintf("Encoding cancelled: %v", ctx.Err()))
			break
		}

		fileStartTime := time.Now()

		// Show file progress for multiple files
		if len(filesToProcess) > 1 {
			rep.FileProgress(reporter.FileProgressContext{
				CurrentFile: fileIdx + 1,
				TotalFiles:  len(filesToProcess),
			})
		}

		inputFilename := util.GetFilename(inputPath)

		// Determine output path
		override := ""
		if len(filesToProcess) == 1 && targetFilenameOverride != "" {
			override = targetFilenameOverride
		}
		outputPath := util.ResolveOutputPath(inputPath, cfg.OutputDir, override)

		// Skip if output exists
		if util.FileExists(outputPath) {
			rep.Warning(fmt.Sprintf("Output file already exists: %s. Skipping encode.", outputPath))
			continue
		}

		// Analyze video properties
		videoProps, err := ffprobe.GetVideoProperties(inputPath)
		if err != nil {
			rep.Error(reporter.ReporterError{
				Title:      "Analysis Error",
				Message:    fmt.Sprintf("Could not analyze %s: %v", inputFilename, err),
				Context:    fmt.Sprintf("File: %s", inputPath),
				Suggestion: "Check if the file is a valid video format",
			})
			continue
		}

		// Use mediainfo for HDR detection
		mediaInfoData, err := mediainfo.GetMediaInfo(inputPath)
		if err != nil {
			rep.Error(reporter.ReporterError{
				Title:      "Analysis Error",
				Message:    fmt.Sprintf("Could not get mediainfo for %s: %v", inputFilename, err),
				Context:    fmt.Sprintf("File: %s", inputPath),
				Suggestion: "Check if mediainfo is installed",
			})
			continue
		}
		hdrInfo := mediainfo.DetectHDR(mediaInfoData)

		// Determine quality settings
		quality, category := determineQualitySettings(videoProps, cfg)
		isHDR := hdrInfo.IsHDR

		// Get audio info
		audioChannels := GetAudioChannels(inputPath)
		audioStreams := GetAudioStreamInfo(inputPath)
		audioDescription := FormatAudioDescription(audioChannels)

		// Emit initialization event
		rep.Initialization(reporter.InitializationSummary{
			InputFile:        inputFilename,
			OutputFile:       util.GetFilename(outputPath),
			Duration:         util.FormatDuration(videoProps.DurationSecs),
			Resolution:       fmt.Sprintf("%dx%d", videoProps.Width, videoProps.Height),
			Category:         category,
			DynamicRange:     formatDynamicRange(isHDR),
			AudioDescription: audioDescription,
		})

		// Perform crop detection
		cropResult := DetectCrop(inputPath, videoProps, cfg.CropMode == "none")

		rep.CropResult(reporter.CropSummary{
			Message:  cropResult.Message,
			Crop:     cropResult.CropFilter,
			Required: cropResult.Required,
			Disabled: cfg.CropMode == "none",
		})

		// Setup encode parameters
		encodeParams := setupEncodeParams(cfg, inputPath, outputPath, quality, videoProps, cropResult, audioChannels, audioStreams, hdrInfo)

		// Format audio description for config display
		audioDescConfig := FormatAudioDescriptionConfig(audioChannels, audioStreams)

		// Emit encoding config
		rep.EncodingConfig(reporter.EncodingConfigSummary{
			Encoder:              "SVT-AV1",
			Preset:               fmt.Sprintf("%d", encodeParams.Preset),
			Tune:                 fmt.Sprintf("%d", encodeParams.Tune),
			Quality:              fmt.Sprintf("CRF %d", encodeParams.Quality),
			PixelFormat:          encodeParams.PixelFormat,
			MatrixCoefficients:   encodeParams.MatrixCoefficients,
			AudioCodec:           "Opus",
			AudioDescription:     audioDescConfig,
			DraptoPreset:         formatPreset(cfg.DraptoPreset),
			DraptoPresetSettings: collectPresetSettings(encodeParams),
			SVTAV1Params:         encodeParams.SVTAV1CLIParams(),
		})

		// Get total frames for progress
		mediaInfo, _ := ffprobe.GetMediaInfo(inputPath)
		totalFrames := uint64(0)
		if mediaInfo != nil {
			totalFrames = mediaInfo.TotalFrames
		}

		rep.EncodingStarted(totalFrames)

		// Run encode
		result := ffmpeg.RunEncode(ctx, encodeParams, false, totalFrames, func(progress ffmpeg.Progress) {
			rep.EncodingProgress(reporter.ProgressSnapshot{
				CurrentFrame: progress.CurrentFrame,
				TotalFrames:  progress.TotalFrames,
				Percent:      progress.Percent,
				Speed:        progress.Speed,
				FPS:          progress.FPS,
				ETA:          progress.ETA,
				Bitrate:      progress.Bitrate,
			})
		})

		if !result.Success {
			rep.Error(reporter.ReporterError{
				Title:      "Encoding Error",
				Message:    fmt.Sprintf("FFmpeg failed to encode %s: %v", inputFilename, result.Error),
				Context:    fmt.Sprintf("File: %s", inputPath),
				Suggestion: "Check FFmpeg logs for more details",
			})
			continue
		}

		fileElapsedTime := time.Since(fileStartTime)

		inputSize, _ := util.GetFileSize(inputPath)
		outputSize, _ := util.GetFileSize(outputPath)
		encodingSpeed := float32(videoProps.DurationSecs) / float32(fileElapsedTime.Seconds())

		// Calculate expected dimensions after crop
		expectedWidth, expectedHeight := GetOutputDimensions(videoProps.Width, videoProps.Height, encodeParams.CropFilter)

		// Validate output
		expectedDims := &[2]uint32{expectedWidth, expectedHeight}
		expectedDuration := videoProps.DurationSecs
		expectedAudioTracks := len(audioChannels)

		validationResult, err := validation.ValidateOutputVideo(inputPath, outputPath, validation.Options{
			ExpectedDimensions:  expectedDims,
			ExpectedDuration:    &expectedDuration,
			ExpectedHDR:         &isHDR,
			ExpectedAudioTracks: &expectedAudioTracks,
		})

		var validationPassed bool
		var validationSteps []validation.ValidationStep
		if err != nil {
			validationPassed = false
			validationSteps = []validation.ValidationStep{
				{Name: "Validation", Passed: false, Details: err.Error()},
			}
		} else {
			validationPassed = validationResult.IsValid()
			for _, step := range validationResult.GetValidationSteps() {
				validationSteps = append(validationSteps, validation.ValidationStep{
					Name:    step.Name,
					Passed:  step.Passed,
					Details: step.Details,
				})
			}
		}

		results = append(results, EncodeResult{
			Filename:          inputFilename,
			Duration:          fileElapsedTime,
			InputSize:         inputSize,
			OutputSize:        outputSize,
			VideoDurationSecs: videoProps.DurationSecs,
			EncodingSpeed:     encodingSpeed,
			ValidationPassed:  validationPassed,
			ValidationSteps:   validationSteps,
		})

		// Emit validation complete
		var repSteps []reporter.ValidationStep
		for _, s := range validationSteps {
			repSteps = append(repSteps, reporter.ValidationStep{
				Name:    s.Name,
				Passed:  s.Passed,
				Details: s.Details,
			})
		}
		rep.ValidationComplete(reporter.ValidationSummary{
			Passed: validationPassed,
			Steps:  repSteps,
		})

		// Emit encoding complete
		rep.EncodingComplete(reporter.EncodingOutcome{
			InputFile:    inputFilename,
			OutputFile:   util.GetFilename(outputPath),
			OriginalSize: inputSize,
			EncodedSize:  outputSize,
			VideoStream:  fmt.Sprintf("AV1 (libsvtav1), %dx%d", expectedWidth, expectedHeight),
			AudioStream:  GenerateAudioResultsDescription(audioChannels, audioStreams),
			TotalTime:    fileElapsedTime,
			AverageSpeed: encodingSpeed,
			OutputPath:   outputPath,
		})

		// Cooldown between encodes
		if len(filesToProcess) > 1 && fileIdx < len(filesToProcess)-1 && cfg.EncodeCooldownSecs > 0 {
			time.Sleep(time.Duration(cfg.EncodeCooldownSecs) * time.Second)
		}
	}

	// Generate summary
	switch len(results) {
	case 0:
		rep.Warning("No files were successfully encoded")
	case 1:
		rep.OperationComplete(fmt.Sprintf("Successfully encoded %s", results[0].Filename))
	default:
		// Calculate totals
		var totalDuration time.Duration
		var totalOriginalSize, totalEncodedSize uint64
		var totalVideoDuration float64
		var fileResults []reporter.FileResult
		validationPassedCount := 0

		for _, r := range results {
			totalDuration += r.Duration
			totalOriginalSize += r.InputSize
			totalEncodedSize += r.OutputSize
			totalVideoDuration += r.VideoDurationSecs
			reduction := util.CalculateSizeReduction(r.InputSize, r.OutputSize)
			fileResults = append(fileResults, reporter.FileResult{
				Filename:  r.Filename,
				Reduction: reduction,
			})
			if r.ValidationPassed {
				validationPassedCount++
			}
		}

		avgSpeed := float32(0)
		if totalDuration.Seconds() > 0 {
			avgSpeed = float32(totalVideoDuration / totalDuration.Seconds())
		}

		rep.BatchComplete(reporter.BatchSummary{
			SuccessfulCount:       len(results),
			TotalFiles:            len(filesToProcess),
			TotalOriginalSize:     totalOriginalSize,
			TotalEncodedSize:      totalEncodedSize,
			TotalDuration:         totalDuration,
			AverageSpeed:          avgSpeed,
			FileResults:           fileResults,
			ValidationPassedCount: validationPassedCount,
			ValidationFailedCount: len(results) - validationPassedCount,
		})
	}

	return results, nil
}

// determineQualitySettings selects quality based on resolution.
func determineQualitySettings(props *ffprobe.VideoProperties, cfg *config.Config) (uint32, string) {
	if props.Width >= config.UHDWidthThreshold {
		return uint32(cfg.QualityUHD), "UHD"
	}
	if props.Width >= config.HDWidthThreshold {
		return uint32(cfg.QualityHD), "HD"
	}
	return uint32(cfg.QualitySD), "SD"
}

func formatDynamicRange(isHDR bool) string {
	if isHDR {
		return "HDR"
	}
	return "SDR"
}

func formatPreset(p *config.Preset) string {
	if p == nil {
		return "Default"
	}
	switch *p {
	case config.PresetGrain:
		return "Grain"
	case config.PresetClean:
		return "Clean"
	case config.PresetQuick:
		return "Quick"
	default:
		return "Default"
	}
}

func setupEncodeParams(
	cfg *config.Config,
	inputPath, outputPath string,
	quality uint32,
	props *ffprobe.VideoProperties,
	crop CropResult,
	audioChannels []uint32,
	audioStreams []ffprobe.AudioStreamInfo,
	hdrInfo mediainfo.HDRInfo,
) *ffmpeg.EncodeParams {
	params := &ffmpeg.EncodeParams{
		InputPath:             inputPath,
		OutputPath:            outputPath,
		Quality:               quality,
		Preset:                cfg.SVTAV1Preset,
		Tune:                  cfg.SVTAV1Tune,
		ACBias:                cfg.SVTAV1ACBias,
		EnableVarianceBoost:   cfg.SVTAV1EnableVarianceBoost,
		VarianceBoostStrength: cfg.SVTAV1VarianceBoostStrength,
		VarianceOctile:        cfg.SVTAV1VarianceOctile,
		VideoDenoiseFilter:    cfg.VideoDenoiseFilter,
		FilmGrain:             cfg.SVTAV1FilmGrain,
		FilmGrainDenoise:      cfg.SVTAV1FilmGrainDenoise,
		Duration:              props.DurationSecs,
		AudioChannels:         audioChannels,
		AudioStreams:          audioStreams,
		VideoCodec:            "libsvtav1",
		PixelFormat:           "yuv420p10le",
		AudioCodec:            "libopus",
	}

	if crop.Required {
		params.CropFilter = crop.CropFilter
	}

	// Set matrix coefficients based on HDR
	if hdrInfo.IsHDR {
		params.MatrixCoefficients = hdrInfo.MatrixCoefficients
		if params.MatrixCoefficients == "" {
			params.MatrixCoefficients = "bt2020nc"
		}
	} else {
		params.MatrixCoefficients = "bt709"
	}

	// Responsive encoding: run at low priority
	params.LowPriority = cfg.ResponsiveEncoding

	return params
}

func collectPresetSettings(params *ffmpeg.EncodeParams) [][2]string {
	settings := [][2]string{
		{"CRF", fmt.Sprintf("%d", params.Quality)},
		{"SVT preset", fmt.Sprintf("%d", params.Preset)},
		{"Tune", fmt.Sprintf("%d", params.Tune)},
		{"AC bias", fmt.Sprintf("%.2f", params.ACBias)},
	}

	if params.EnableVarianceBoost {
		settings = append(settings, [2]string{"Variance boost",
			fmt.Sprintf("enabled (strength %d, octile %d)",
				params.VarianceBoostStrength, params.VarianceOctile)})
	} else {
		settings = append(settings, [2]string{"Variance boost", "disabled"})
	}

	if params.VideoDenoiseFilter != "" {
		settings = append(settings, [2]string{"Denoise", params.VideoDenoiseFilter})
	}

	if params.FilmGrain != nil {
		denoise := "-"
		if params.FilmGrainDenoise != nil {
			if *params.FilmGrainDenoise {
				denoise = "1"
			} else {
				denoise = "0"
			}
		}
		settings = append(settings, [2]string{"Film grain synth",
			fmt.Sprintf("film-grain %d, denoise %s", *params.FilmGrain, denoise)})
	}

	return settings
}
