# drapto

FFmpeg wrapper for AV1 encoding with SVT-AV1 and Opus audio. Uses opinionated defaults so you can encode without dealing with ffmpeg's complexity.

## Features

- Automatic black bar crop detection with multi-ratio handling
- HDR10/HLG metadata preservation
- Resolution-based CRF defaults (SD/HD/UHD)
- Multi-track audio transcoding to Opus with per-layout bitrates
- Post-encode validation (codec, bit depth, dimensions, duration, HDR, A/V sync)
- Preset profiles: `grain`, `clean`, `quick`
- Library API for Spindle integration
- Standalone crop detection API for debugging

## Requirements

- Go 1.25+
- FFmpeg with `libsvtav1` and `libopus`
- MediaInfo

```bash
# Ubuntu/Debian
sudo apt-get install ffmpeg mediainfo

# Verify FFmpeg has required encoders
ffmpeg -encoders | grep -E "svtav1|opus"
```

## Install

```bash
go install github.com/five82/drapto/cmd/drapto@latest
```

Or build from source:

```bash
git clone https://github.com/five82/drapto
cd drapto
go build -o drapto ./cmd/drapto
```

## Usage

```bash
drapto encode -i input.mkv -o output/
drapto encode -i /videos/ -o /encoded/
drapto encode -i input.mkv -o output/ --drapto-preset grain
```

### Options

```
-i, --input          Input video file or directory (required)
-o, --output         Output directory (required)
-l, --log-dir        Log directory (defaults to ~/.local/state/drapto/logs)
    --drapto-preset  Apply preset: grain, clean, quick
    --crf            CRF quality (0-63), single value or SD,HD,UHD triple
                     Default: 25,27,29 (SD,HD,UHD)
    --preset         SVT-AV1 preset 0-13, default 6
    --disable-autocrop  Disable black bar detection
    --responsive     Use nice -n 19 for improved system responsiveness
    --no-log         Disable log file creation
-v, --verbose        Verbose output
```

## Library Usage

Drapto can be used as a Go library:

```go
import "github.com/five82/drapto"

encoder, err := drapto.New(
    drapto.WithPreset(drapto.PresetGrain),
    drapto.WithCRFHD(27),
)
if err != nil {
    log.Fatal(err)
}

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

### Additional Library Methods

```go
// Batch encode multiple files
batchResult, err := encoder.EncodeBatch(ctx, []string{"a.mkv", "b.mkv"}, "output/", handler)

// Use custom Reporter for detailed progress
result, err := encoder.EncodeWithReporter(ctx, "input.mkv", "output/", customReporter)

// Standalone crop detection (no encoding)
cropResult, err := drapto.DetectCrop(ctx, "input.mkv")

// Find video files in directory
files, err := drapto.FindVideos("/path/to/videos")
```

See `docs/spindle-integration.md` for event types and integration details.

## Project Structure

```
drapto/
├── drapto.go           # Public API: Encoder, Options, Result types
├── events.go           # Event types for EventHandler callbacks
├── reporter.go         # Re-exported Reporter interface and types
├── cmd/drapto/         # CLI wrapper
└── internal/
    ├── config/         # Configuration, presets, defaults
    ├── ffmpeg/         # FFmpeg command builder and executor
    ├── ffprobe/        # Media analysis via ffprobe
    ├── mediainfo/      # HDR detection via MediaInfo
    ├── processing/     # Encoding orchestration, crop detection, audio
    ├── validation/     # Post-encode validation checks
    ├── reporter/       # Progress reporting (terminal, composite, log)
    ├── discovery/      # Video file discovery
    ├── logging/        # File logging
    └── util/           # Formatting, file, and temp file utilities
```

## Development

```bash
go build ./...
go test ./...
go vet ./...
```
