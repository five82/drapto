# Spindle Integration

Drapto is designed to be embedded by Spindle during the `ENCODING` stage. This document covers the integration contract.

## Library API

### Creating an Encoder

```go
import "github.com/five82/drapto"

// Create encoder with options
encoder, err := drapto.New(
    drapto.WithPreset(drapto.PresetGrain),
    drapto.WithCRFHD(27),
)
if err != nil {
    log.Fatal(err)
}
```

### Configuration Options

| Option | Description |
|--------|-------------|
| `WithPreset(Preset)` | Apply preset: `PresetGrain`, `PresetClean`, `PresetQuick` |
| `WithCRFSD(uint8)` | CRF for SD videos (<1920 width) |
| `WithCRFHD(uint8)` | CRF for HD videos (1920-3839 width) |
| `WithCRFUHD(uint8)` | CRF for UHD videos (≥3840 width) |
| `WithCRF(string)` | Parse CRF: single value or "SD,HD,UHD" triple |
| `WithDisableAutocrop()` | Skip black bar detection |
| `WithResponsive()` | Run at low priority (nice -n 19) |
| `WithFilmGrain(uint8)` | Enable film grain synthesis (0-50) |
| `WithFilmGrainDenoise(bool)` | Denoise before adding grain |

### Encoding Methods

```go
// Single file encode with event handler
result, err := encoder.Encode(ctx, "input.mkv", "output/", func(event drapto.Event) error {
    switch e := event.(type) {
    case drapto.EncodingProgressEvent:
        fmt.Printf("Progress: %.1f%%\n", e.Percent)
    case drapto.EncodingCompleteEvent:
        fmt.Printf("Done: %.1f%% reduction\n", e.SizeReductionPercent)
    }
    return nil
})

// Batch encode
batchResult, err := encoder.EncodeBatch(ctx, inputs, "output/", handler)

// Encode with custom Reporter (full control over all events)
result, err := encoder.EncodeWithReporter(ctx, "input.mkv", "output/", customReporter)
```

### Result Types

```go
// Result from single file encode
type Result struct {
    OutputFile           string
    OriginalSize         uint64
    EncodedSize          uint64
    SizeReductionPercent float64
    ValidationPassed     bool
    EncodingSpeed        float32
}

// Result from batch encode
type BatchResult struct {
    Results               []Result
    SuccessfulCount       int
    TotalFiles            int
    TotalSizeReduction    float64
    ValidationPassedCount int
}
```

### Standalone Crop Detection

For debugging or pre-analysis without encoding:

```go
cropResult, err := drapto.DetectCrop(ctx, "input.mkv")
if err != nil {
    log.Fatal(err)
}

fmt.Printf("Crop required: %v\n", cropResult.Required)
fmt.Printf("Filter: %s\n", cropResult.CropFilter)
fmt.Printf("Message: %s\n", cropResult.Message)

// Analyze candidates when multiple ratios detected
for _, c := range cropResult.Candidates {
    fmt.Printf("  %s: %d samples (%.1f%%)\n", c.Crop, c.Count, c.Percent)
}
```

### Helper Functions

```go
// Find video files in a directory
files, err := drapto.FindVideos("/path/to/videos")

// Parse CRF string
sd, hd, uhd, err := drapto.ParseCRF("25,27,29")

// Parse preset string
preset, err := drapto.ParsePreset("grain")
```

## Event Types

All events implement `drapto.Event` interface with `Type()` and `Timestamp()` methods.

### Progress Events

```go
// EncodingProgressEvent - periodic progress updates
type EncodingProgressEvent struct {
    Percent    float32 // 0-100
    Speed      float32 // encoding speed multiplier
    FPS        float32 // frames per second
    ETASeconds int64   // estimated time remaining
}
```

### Completion Events

```go
// EncodingCompleteEvent - single file completed
type EncodingCompleteEvent struct {
    OutputFile           string
    OriginalSize         uint64
    EncodedSize          uint64
    SizeReductionPercent float64
}

// ValidationCompleteEvent - validation results
type ValidationCompleteEvent struct {
    ValidationPassed bool
    ValidationSteps  []ValidationStep
}

// BatchCompleteEvent - batch operation summary
type BatchCompleteEvent struct {
    SuccessfulCount           int
    TotalFiles                int
    TotalSizeReductionPercent float64
}
```

### Notification Events

```go
// WarningEvent - non-fatal warnings
type WarningEvent struct {
    Message string
}

// ErrorEvent - errors with context
type ErrorEvent struct {
    Title      string
    Message    string
    Context    string
    Suggestion string
}
```

## Reporter Interface

For fine-grained control over all encoding events, implement the `drapto.Reporter` interface:

```go
type Reporter interface {
    Hardware(HardwareSummary)
    Initialization(InitializationSummary)
    StageProgress(StageProgress)
    CropResult(CropSummary)
    EncodingConfig(EncodingConfigSummary)
    EncodingStarted(totalFrames uint64)
    EncodingProgress(ProgressSnapshot)
    ValidationComplete(ValidationSummary)
    EncodingComplete(EncodingOutcome)
    Warning(message string)
    Error(ReporterError)
    OperationComplete(summary string)
    BatchStarted(BatchStartInfo)
    FileProgress(FileProgressContext)
    BatchComplete(BatchSummary)
}
```

All reporter types are re-exported from the `drapto` package.

See `events.go` and `reporter.go` for full type definitions.
