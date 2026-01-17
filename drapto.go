// Package drapto provides a Go library for AV1 video encoding with SVT-AV1.
//
// Drapto is an opinionated FFmpeg wrapper that handles the complexity of
// AV1 encoding with sensible defaults, automatic crop detection, HDR metadata
// preservation, and post-encode validation.
//
// Basic usage:
//
//	encoder, err := drapto.New(
//	    drapto.WithPreset(drapto.PresetGrain),
//	)
//	if err != nil {
//	    log.Fatal(err)
//	}
//
//	result, err := encoder.Encode(ctx, "input.mkv", "output/", nil)
//	if err != nil {
//	    log.Fatal(err)
//	}
//
//	fmt.Printf("Encoded: %s, reduction: %.1f%%\n",
//	    result.OutputFile, result.SizeReductionPercent)
package drapto

import (
	"context"
	"fmt"

	"github.com/five82/drapto/internal/config"
	"github.com/five82/drapto/internal/discovery"
	"github.com/five82/drapto/internal/processing"
	"github.com/five82/drapto/internal/reporter"
	"github.com/five82/drapto/internal/util"
)

// Re-export preset types
type Preset = config.Preset

const (
	PresetGrain = config.PresetGrain
	PresetClean = config.PresetClean
	PresetQuick = config.PresetQuick
)

// ParsePreset converts a preset string to a Preset value.
// Valid values are "grain", "clean", and "quick" (case-insensitive).
func ParsePreset(s string) (Preset, error) {
	return config.ParsePreset(s)
}

// Encoder is the main entry point for video encoding.
type Encoder struct {
	config *config.Config
}

// Result contains the result of a single file encode.
type Result struct {
	OutputFile           string
	OriginalSize         uint64
	EncodedSize          uint64
	SizeReductionPercent float64
	ValidationPassed     bool
	EncodingSpeed        float32
}

// BatchResult contains the result of a batch encode.
type BatchResult struct {
	Results               []Result
	SuccessfulCount       int
	TotalFiles            int
	TotalSizeReduction    float64
	ValidationPassedCount int
}

// Option configures the encoder.
type Option func(*config.Config)

// New creates a new Encoder with the given options.
func New(opts ...Option) (*Encoder, error) {
	cfg := config.NewConfig(".", ".", ".")

	for _, opt := range opts {
		opt(cfg)
	}

	if err := cfg.Validate(); err != nil {
		return nil, err
	}

	return &Encoder{config: cfg}, nil
}

// WithPreset applies a Drapto preset.
func WithPreset(p Preset) Option {
	return func(c *config.Config) {
		c.ApplyPreset(p)
	}
}

// WithQualitySD sets the CRF quality for SD videos (<1920 width).
func WithQualitySD(crf uint8) Option {
	return func(c *config.Config) {
		c.QualitySD = crf
	}
}

// WithQualityHD sets the CRF quality for HD videos (>=1920 width).
func WithQualityHD(crf uint8) Option {
	return func(c *config.Config) {
		c.QualityHD = crf
	}
}

// WithQualityUHD sets the CRF quality for UHD videos (>=3840 width).
func WithQualityUHD(crf uint8) Option {
	return func(c *config.Config) {
		c.QualityUHD = crf
	}
}

// WithDisableAutocrop disables automatic black bar detection.
func WithDisableAutocrop() Option {
	return func(c *config.Config) {
		c.CropMode = "none"
	}
}

// WithResponsive enables responsive encoding (reserves CPU threads).
func WithResponsive() Option {
	return func(c *config.Config) {
		c.ResponsiveEncoding = true
	}
}

// WithFilmGrain enables SVT-AV1 film grain synthesis with the given strength.
// Strength should be 0-50, where higher values add more synthetic grain.
func WithFilmGrain(strength uint8) Option {
	return func(c *config.Config) {
		c.SVTAV1FilmGrain = &strength
	}
}

// WithFilmGrainDenoise sets whether to denoise when using film grain synthesis.
// When true (default), the source is denoised before adding synthetic grain.
func WithFilmGrainDenoise(enable bool) Option {
	return func(c *config.Config) {
		c.SVTAV1FilmGrainDenoise = &enable
	}
}

// EncodeWithReporter encodes a single video file using a custom Reporter.
// This provides direct access to all encoding events, unlike Encode which
// uses the EventHandler abstraction.
func (e *Encoder) EncodeWithReporter(ctx context.Context, input, outputDir string, rep Reporter) (*Result, error) {
	// Update config paths
	cfg := *e.config
	cfg.OutputDir = outputDir

	// Ensure output directory exists
	if err := util.EnsureDirectory(outputDir); err != nil {
		return nil, fmt.Errorf("failed to create output directory: %w", err)
	}

	// Use provided reporter or null reporter
	if rep == nil {
		rep = reporter.NullReporter{}
	}

	// Process single file
	results, err := processing.ProcessVideos(ctx, &cfg, []string{input}, "", rep)
	if err != nil {
		return nil, err
	}

	if len(results) == 0 {
		return nil, fmt.Errorf("no files were encoded")
	}

	r := results[0]
	return &Result{
		OutputFile:           util.ResolveOutputPath(input, outputDir, ""),
		OriginalSize:         r.InputSize,
		EncodedSize:          r.OutputSize,
		SizeReductionPercent: util.CalculateSizeReduction(r.InputSize, r.OutputSize),
		ValidationPassed:     r.ValidationPassed,
		EncodingSpeed:        r.EncodingSpeed,
	}, nil
}

// Encode encodes a single video file.
func (e *Encoder) Encode(ctx context.Context, input, outputDir string, handler EventHandler) (*Result, error) {
	// Update config paths
	cfg := *e.config
	cfg.OutputDir = outputDir

	// Ensure output directory exists
	if err := util.EnsureDirectory(outputDir); err != nil {
		return nil, fmt.Errorf("failed to create output directory: %w", err)
	}

	// Create reporter
	var rep reporter.Reporter = reporter.NullReporter{}
	if handler != nil {
		rep = newEventReporter(handler)
	}

	// Process single file
	results, err := processing.ProcessVideos(ctx, &cfg, []string{input}, "", rep)
	if err != nil {
		return nil, err
	}

	if len(results) == 0 {
		return nil, fmt.Errorf("no files were encoded")
	}

	r := results[0]
	return &Result{
		OutputFile:           util.ResolveOutputPath(input, outputDir, ""),
		OriginalSize:         r.InputSize,
		EncodedSize:          r.OutputSize,
		SizeReductionPercent: util.CalculateSizeReduction(r.InputSize, r.OutputSize),
		ValidationPassed:     r.ValidationPassed,
		EncodingSpeed:        r.EncodingSpeed,
	}, nil
}

// EncodeBatch encodes multiple video files.
func (e *Encoder) EncodeBatch(ctx context.Context, inputs []string, outputDir string, handler EventHandler) (*BatchResult, error) {
	// Update config paths
	cfg := *e.config
	cfg.OutputDir = outputDir

	// Ensure output directory exists
	if err := util.EnsureDirectory(outputDir); err != nil {
		return nil, fmt.Errorf("failed to create output directory: %w", err)
	}

	// Create reporter
	var rep reporter.Reporter = reporter.NullReporter{}
	if handler != nil {
		rep = newEventReporter(handler)
	}

	// Process files
	results, err := processing.ProcessVideos(ctx, &cfg, inputs, "", rep)
	if err != nil {
		return nil, err
	}

	batch := &BatchResult{
		TotalFiles: len(inputs),
	}

	var totalInputSize, totalOutputSize uint64
	for _, r := range results {
		batch.Results = append(batch.Results, Result{
			OutputFile:           util.ResolveOutputPath(r.Filename, outputDir, ""),
			OriginalSize:         r.InputSize,
			EncodedSize:          r.OutputSize,
			SizeReductionPercent: util.CalculateSizeReduction(r.InputSize, r.OutputSize),
			ValidationPassed:     r.ValidationPassed,
			EncodingSpeed:        r.EncodingSpeed,
		})
		batch.SuccessfulCount++
		totalInputSize += r.InputSize
		totalOutputSize += r.OutputSize
		if r.ValidationPassed {
			batch.ValidationPassedCount++
		}
	}

	batch.TotalSizeReduction = util.CalculateSizeReduction(totalInputSize, totalOutputSize)

	return batch, nil
}

// FindVideos finds video files in a directory.
func FindVideos(dir string) ([]string, error) {
	return discovery.FindVideoFiles(dir)
}

// eventReporter adapts EventHandler to the Reporter interface.
type eventReporter struct {
	handler EventHandler
}

func newEventReporter(handler EventHandler) *eventReporter {
	return &eventReporter{handler: handler}
}

func (r *eventReporter) Hardware(reporter.HardwareSummary)             {}
func (r *eventReporter) Initialization(reporter.InitializationSummary) {}
func (r *eventReporter) StageProgress(reporter.StageProgress)          {}
func (r *eventReporter) CropResult(reporter.CropSummary)               {}
func (r *eventReporter) EncodingConfig(reporter.EncodingConfigSummary) {}
func (r *eventReporter) EncodingStarted(uint64)                        {}

func (r *eventReporter) EncodingProgress(p reporter.ProgressSnapshot) {
	_ = r.handler(EncodingProgressEvent{
		BaseEvent:  BaseEvent{EventType: EventTypeEncodingProgress, Time: NewTimestamp()},
		Percent:    p.Percent,
		Speed:      p.Speed,
		FPS:        p.FPS,
		ETASeconds: int64(p.ETA.Seconds()),
	})
}

func (r *eventReporter) ValidationComplete(s reporter.ValidationSummary) {
	steps := make([]ValidationStep, len(s.Steps))
	for i, step := range s.Steps {
		steps[i] = ValidationStep{
			Step:    step.Name,
			Passed:  step.Passed,
			Details: step.Details,
		}
	}
	_ = r.handler(ValidationCompleteEvent{
		BaseEvent:        BaseEvent{EventType: EventTypeValidationComplete, Time: NewTimestamp()},
		ValidationPassed: s.Passed,
		ValidationSteps:  steps,
	})
}

func (r *eventReporter) EncodingComplete(s reporter.EncodingOutcome) {
	_ = r.handler(EncodingCompleteEvent{
		BaseEvent:            BaseEvent{EventType: EventTypeEncodingComplete, Time: NewTimestamp()},
		OutputFile:           s.OutputFile,
		OriginalSize:         s.OriginalSize,
		EncodedSize:          s.EncodedSize,
		SizeReductionPercent: util.CalculateSizeReduction(s.OriginalSize, s.EncodedSize),
	})
}

func (r *eventReporter) Warning(message string) {
	_ = r.handler(WarningEvent{
		BaseEvent: BaseEvent{EventType: EventTypeWarning, Time: NewTimestamp()},
		Message:   message,
	})
}

func (r *eventReporter) Error(e reporter.ReporterError) {
	_ = r.handler(ErrorEvent{
		BaseEvent:  BaseEvent{EventType: EventTypeError, Time: NewTimestamp()},
		Title:      e.Title,
		Message:    e.Message,
		Context:    e.Context,
		Suggestion: e.Suggestion,
	})
}

func (r *eventReporter) OperationComplete(string)                  {}
func (r *eventReporter) BatchStarted(reporter.BatchStartInfo)      {}
func (r *eventReporter) FileProgress(reporter.FileProgressContext) {}

func (r *eventReporter) BatchComplete(s reporter.BatchSummary) {
	_ = r.handler(BatchCompleteEvent{
		BaseEvent:                 BaseEvent{EventType: EventTypeBatchComplete, Time: NewTimestamp()},
		SuccessfulCount:           s.SuccessfulCount,
		TotalFiles:                s.TotalFiles,
		TotalSizeReductionPercent: util.CalculateSizeReduction(s.TotalOriginalSize, s.TotalEncodedSize),
	})
}
