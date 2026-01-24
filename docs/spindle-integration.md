# Spindle Integration

Drapto is designed to be embedded by Spindle during the `ENCODING` stage. This document covers the integration contract.

## Library API

```go
import "github.com/five82/drapto"

// Create encoder with options
encoder, err := drapto.New(
    drapto.WithCRF(27),
    drapto.WithWorkers(4),
    drapto.WithChunkBuffer(4),
)
if err != nil {
    log.Fatal(err)
}

// Encode with progress callback
result, err := encoder.Encode(ctx, "input.mkv", "output/", func(event drapto.Event) error {
    switch e := event.(type) {
    case drapto.EncodingProgressEvent:
        fmt.Printf("Progress: %.1f%%\n", e.Percent)
    case drapto.EncodingCompleteEvent:
        fmt.Printf("Done: %.1f%% reduction\n", e.SizeReductionPercent)
    }
    return nil
})
```

## Option Functions

```go
// Quality settings
drapto.WithCRF(crf uint8)                    // CRF quality level (0-63, lower = better)

// Processing options
drapto.WithDisableAutocrop()                 // Skip automatic crop detection
drapto.WithWorkers(n int)                    // Number of parallel encoder workers
drapto.WithChunkBuffer(n int)                // Extra chunks to buffer in memory

// Film grain
drapto.WithFilmGrain(strength uint8)         // Enable film grain synthesis (0-50)
drapto.WithFilmGrainDenoise(enable bool)     // Denoise when using film grain
```

## Encoding Methods

```go
// Single file with event handler
result, err := encoder.Encode(ctx, input, outputDir, handler)

// Single file with Reporter interface (direct access to all events)
result, err := encoder.EncodeWithReporter(ctx, input, outputDir, reporter)

// Multiple files
batchResult, err := encoder.EncodeBatch(ctx, inputs, outputDir, handler)

// Find video files in directory
files, err := drapto.FindVideos(dir)
```

## Result Types

```go
// Single file result
type Result struct {
    OutputFile           string
    OriginalSize         uint64
    EncodedSize          uint64
    SizeReductionPercent float64
    ValidationPassed     bool
    EncodingSpeed        float32
}

// Batch result
type BatchResult struct {
    Results               []Result
    SuccessfulCount       int
    TotalFiles            int
    TotalSizeReduction    float64
    ValidationPassedCount int
}
```

## Event Types

All events implement `drapto.Event` interface with `Type()` and `Timestamp()` methods.

### Progress Events

```go
type EncodingProgressEvent struct {
    Percent    float64  // 0-100
    Speed      float32  // Encoding speed multiplier
    FPS        float32  // Frames per second
    ETASeconds int64    // Estimated time remaining
}
```

### Completion Events

```go
type EncodingCompleteEvent struct {
    OutputFile           string
    OriginalSize         uint64
    EncodedSize          uint64
    SizeReductionPercent float64
}

type ValidationCompleteEvent struct {
    ValidationPassed bool
    ValidationSteps  []ValidationStep
}

type BatchCompleteEvent struct {
    SuccessfulCount           int
    TotalFiles                int
    TotalSizeReductionPercent float64
}
```

### Warning and Error Events

```go
type WarningEvent struct {
    Message string
}

type ErrorEvent struct {
    Title      string
    Message    string
    Context    string
    Suggestion string
}
```

## Reporter Interface

For more control over progress reporting, implement the `Reporter` interface and use `EncodeWithReporter`:

```go
type Reporter interface {
    Hardware(HardwareSummary)
    Initialization(InitializationSummary)
    StageProgress(StageProgress)
    CropResult(CropSummary)
    EncodingConfig(EncodingConfigSummary)
    EncodingStarted(uint64)
    EncodingProgress(ProgressSnapshot)
    ValidationComplete(ValidationSummary)
    EncodingComplete(EncodingOutcome)
    Warning(string)
    Error(ReporterError)
    OperationComplete(string)
    BatchStarted(BatchStartInfo)
    FileProgress(FileProgressContext)
    BatchComplete(BatchSummary)
}
```

See `events.go` and `internal/reporter/reporter.go` for full type definitions.
