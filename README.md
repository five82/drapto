# drapto

FFmpeg wrapper for AV1 encoding with SVT-AV1 and Opus audio. Uses opinionated defaults so you can encode without dealing with ffmpeg's complexity.

## Features

- Automatic black bar crop detection
- HDR10/HLG metadata preservation
- Resolution-based CRF defaults (SD/HD/UHD)
- Multi-track audio transcoding to Opus
- Post-encode validation (codec, dimensions, duration, HDR)
- Preset profiles: `grain`, `clean`, `quick`
- Library API for embedding

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
    --quality-sd     CRF for SD (<1920 width), default 25
    --quality-hd     CRF for HD (>=1920 width), default 27
    --quality-uhd    CRF for UHD (>=3840 width), default 29
    --preset         SVT-AV1 preset 0-13, default 6
    --disable-autocrop  Disable black bar detection
    --responsive     Reserve CPU threads for responsiveness
    --no-log         Disable log file creation
-v, --verbose        Verbose output
```

## Library Usage

Drapto can be used as a Go library:

```go
import "github.com/five82/drapto"

encoder, err := drapto.New(
    drapto.WithPreset(drapto.PresetGrain),
    drapto.WithQualityHD(27),
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

## Project Structure

```
drapto/
├── drapto.go           # Public API
├── events.go           # Event types for progress callbacks
├── cmd/drapto/         # CLI
└── internal/
    ├── config/         # Configuration and presets
    ├── ffmpeg/         # FFmpeg command builder and executor
    ├── ffprobe/        # Media analysis
    ├── mediainfo/      # HDR detection
    ├── processing/     # Encoding orchestration, crop detection
    ├── validation/     # Post-encode validation
    ├── reporter/       # Progress reporting (JSON, terminal)
    ├── discovery/      # Video file discovery
    └── util/           # Formatting utilities
```

## Development

```bash
go build ./...
go test ./...
go vet ./...
```
