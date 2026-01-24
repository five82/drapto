# drapto

FFmpeg wrapper for AV1 encoding with SVT-AV1 and Opus audio. Uses opinionated defaults so you can encode without dealing with ffmpeg's complexity.

## Features

- Parallel chunked encoding with scene-based splitting
- Automatic black bar crop detection
- HDR10/HLG metadata preservation
- Multi-track audio transcoding to Opus
- Post-encode validation (codec, dimensions, duration, HDR)
- Library API for embedding

## Requirements

- Go 1.25+
- FFmpeg with `libsvtav1` and `libopus`
- SvtAv1EncApp (SVT-AV1 standalone encoder)
- FFMS2 (for frame-accurate video indexing)
- MediaInfo

```bash
# Ubuntu/Debian
sudo apt-get install ffmpeg mediainfo libffms2-dev svt-av1

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
```

### Options

```
Required:
  -i, --input          Input video file or directory (required)
  -o, --output         Output directory (required)

Quality Settings:
  --crf <0-63>         CRF quality level (default 27, lower = better quality)
  --preset <0-13>      SVT-AV1 preset (default 6, lower = slower/better)

Processing Options:
  --disable-autocrop       Disable black bar detection
  --responsive             Reserve CPU threads for responsiveness
  --workers <N>            Parallel encoder workers (default: auto)
  --buffer <N>             Chunks to buffer in memory (default: auto)
  --scene-threshold <N>    Scene detection threshold 0.0-1.0 (default 0.5)
  --min-chunk <N>          Minimum chunk duration in seconds (default 4.0)

Output Options:
  -l, --log-dir            Log directory (defaults to ~/.local/state/drapto/logs)
  -v, --verbose            Verbose output
  --no-log                 Disable log file creation
```

## Library Usage

Drapto can be used as a Go library:

```go
import "github.com/five82/drapto"

encoder, err := drapto.New(
    drapto.WithCRF(27),
    drapto.WithWorkers(4),
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
    ├── config/         # Configuration and defaults
    ├── discovery/      # Video file discovery
    ├── encoding/       # Encoder setup
    ├── encode/         # Parallel chunk encoding pipeline
    ├── chunk/          # Chunk management
    ├── keyframe/       # Scene detection and keyframe extraction
    ├── worker/         # Worker pool for parallel encoding
    ├── ffms/           # FFMS2 bindings for frame indexing
    ├── ffmpeg/         # FFmpeg parameter building
    ├── ffprobe/        # Media analysis
    ├── mediainfo/      # HDR detection
    ├── processing/     # Orchestration, crop detection, audio
    ├── validation/     # Post-encode validation
    ├── reporter/       # Progress reporting (terminal, composite)
    ├── logging/        # File logging
    └── util/           # Formatting utilities
```

## Development

```bash
go build ./...
go test ./...
golangci-lint run
./check-ci.sh          # Full CI check
```
