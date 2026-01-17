# Spindle Integration

Drapto is designed to be embedded by Spindle during the `ENCODING` stage. This document covers the integration contract.

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
