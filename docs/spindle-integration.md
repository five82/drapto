# Spindle Integration

Drapto is designed to be embedded by Spindle during the `ENCODING` stage. This document covers the integration contract.

## JSON Event Stream

The `--progress-json` flag emits newline-delimited JSON (NDJSON). Spindle depends on these event types:

| Event | Key Fields |
|-------|------------|
| `encoding_progress` | `type`, `percent`, `speed`, `fps`, `eta_seconds`, `timestamp` |
| `validation_complete` | `type`, `validation_passed`, `validation_steps[]`, `timestamp` |
| `encoding_complete` | `type`, `output_file`, `original_size`, `encoded_size`, `size_reduction_percent`, `timestamp` |
| `warning` | `type`, `message`, `timestamp` |
| `error` | `type`, `title`, `message`, `context`, `suggestion`, `timestamp` |
| `batch_complete` | `type`, `successful_count`, `total_files`, `total_size_reduction_percent`, `timestamp` |

**Throttling**: Progress events emit on 1% bucket change OR 5 second minimum interval.

**Schema**: Defined in `internal/reporter/json.go` (`JSONReporter`).

## Library API

```go
import "github.com/five82/drapto"

// Create encoder with options
encoder, err := drapto.New(
    drapto.WithPreset(drapto.PresetGrain),
    drapto.WithQualityHD(27),
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

## Event Types

All events implement `drapto.Event` interface. Key types:

- `EncodingProgressEvent` - periodic progress updates
- `EncodingCompleteEvent` - single file completed
- `ValidationCompleteEvent` - validation results
- `WarningEvent` - non-fatal warnings
- `ErrorEvent` - errors with context
- `BatchCompleteEvent` - batch operation summary

See `events.go` for full type definitions.

## Backward Compatibility

**Do not break the JSON event schema without updating Spindle.**

Spindle's consumer is in `internal/services/drapto/runner.go` and `internal/encodingstate/`. Changes to event structure or field names require coordinated updates.
