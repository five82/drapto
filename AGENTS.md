# AGENTS.md

This file provides guidance when working with code in this repository.

CLAUDE.md and GEMINI.md are symlinks to this file so all agent guidance stays in one place.
Do not modify this header.

Do not run `git commit` or `git push` unless explicitly instructed.

## System Dependencies

Runtime requires FFmpeg with libsvtav1 and libopus, plus MediaInfo:

```bash
# Ubuntu/Debian
sudo apt-get install ffmpeg mediainfo

# FFmpeg must have libsvtav1 and libopus support
ffmpeg -encoders | grep -E "svtav1|opus"
```

## Related Repos (Local Dev Layout)

Drapto is one of three sibling repos that are developed together on this machine:

- **drapto** (this repo): `~/projects/drapto/` — ffmpeg encoding wrapper + JSON progress stream
- **spindle**: `~/projects/spindle/` — orchestrator that shells out to Drapto during `ENCODING`
- **flyer**: `~/projects/flyer/` — read-only TUI for Spindle (not a Drapto consumer)

GitHub:

- flyer - https://github.com/five82/flyer
- spindle - https://github.com/five82/spindle
- drapto - https://github.com/five82/drapto

Key contract: keep the `--progress-json` stream backward-compatible with the objects Spindle consumes (`encoding_progress`, `validation_complete`, `encoding_complete`, `warning`, `error`, `batch_complete`).

## MCP

Always use Context7 MCP when I need library/API documentation, code generation, setup or configuration steps without me having to explicitly ask.

## Project Overview

Drapto is an ffmpeg wrapper for AV1 encoding with SVT-AV1 and Opus audio. It uses opinionated defaults so you can encode without dealing with ffmpeg's complexity. Features include automatic crop detection, HDR metadata preservation, and post-encode validation.

## Architecture

The project is written in Go with a library-first design for Spindle integration:

1. **Public API** (`drapto.go`, `events.go`): Library interface for embedding
   - `Encoder` type with `Encode()` and `EncodeBatch()` methods
   - Functional options pattern for configuration
   - `EventHandler` callback for progress events

2. **CLI** (`cmd/drapto/main.go`): Thin wrapper using Cobra
   - Matches the original Rust CLI flags
   - `--progress-json` for machine-readable output

3. **Internal packages**:
   - `internal/config/` - Configuration types and presets
   - `internal/ffmpeg/` - FFmpeg command builder and executor
   - `internal/ffprobe/` - Media analysis
   - `internal/mediainfo/` - HDR detection
   - `internal/processing/` - Encoding orchestration, crop detection, audio
   - `internal/validation/` - Post-encode validation checks
   - `internal/reporter/` - Progress reporting (JSON, Terminal, Composite)
   - `internal/discovery/` - Video file discovery
   - `internal/util/` - Formatting and file utilities

## Build, Test, Lint Commands

```bash
# Build
go build ./...                          # Build all packages
go build -o drapto ./cmd/drapto         # Build CLI binary

# Run
./drapto encode -i input.mkv -o output/            # output as output/input.mkv
./drapto encode -i input.mkv -o output/custom.mkv  # specific filename
./drapto encode -i input.mkv -o output/ --progress-json

# Test
go test ./...                           # Run all tests
go test -race ./...                     # Run all tests with race detector
go test ./internal/config              # Run tests for a specific package
go test -v ./internal/config -run TestName  # Run a single test by name

# Lint
golangci-lint run                       # Run linter
golangci-lint run --fix                 # Auto-fix safe issues

# Full CI check (recommended before handing off)
./check-ci.sh                           # Runs: go mod tidy, go test, go test -race,
                                        # go build, golangci-lint, govulncheck

# Dependencies
go mod tidy                             # sync go.mod
```

## Development Workflow

- Install Go 1.23+ and keep `golangci-lint` v2.0+ up to date via `go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest`.
- Before handing off, execute `./check-ci.sh`. If you cannot run it, state why.

## Key Components

### FFmpeg Integration

The project uses FFmpeg for video processing via:

1. Direct `exec.Command` calls for FFmpeg/FFprobe
2. Progress parsing from FFmpeg stderr
3. Custom command builders in `internal/ffmpeg/`

### Progress Reporting

The progress reporting system provides feedback during long-running operations:

1. Terminal-based progress bars (`schollz/progressbar`)
2. NDJSON events for Spindle integration

### CLI Output Style

1. Render just four sections in human mode: Hardware → Video → Encoding → Validation → Results; keep each section to a handful of lines.
2. Use `fatih/color` for colored output and `schollz/progressbar` for progress. Colors reinforce meaning (cyan headers, magenta stage bullets, green checkmarks, yellow warnings, red errors).
3. Show real progress information only once: rely on the progress bar during encoding, and print a single validation summary after it completes.
4. Keep the JSON stream (`--progress-json`) backward-compatible with the existing objects Spindle consumes.
5. Prefer natural language sentences ("Encoding finished successfully") and reserve emphatic formatting for values that matter (reduction %, warnings, output paths).

## Project Structure

```
drapto/
├── go.mod                          # github.com/five82/drapto
├── drapto.go                       # Public API: Encoder, Options
├── events.go                       # Event types (Spindle contract)
├── cmd/drapto/main.go              # CLI wrapper
├── internal/
│   ├── config/                     # Configuration and presets
│   ├── discovery/                  # File discovery
│   ├── ffmpeg/                     # FFmpeg command builder + executor
│   ├── ffprobe/                    # FFprobe executor + parsing
│   ├── mediainfo/                  # MediaInfo executor + HDR detection
│   ├── processing/                 # Orchestrator, crop, audio
│   ├── validation/                 # Post-encode validation checks
│   ├── reporter/                   # Reporter interface + implementations
│   └── util/                       # Formatting, file utils, system info
└── reference/                      # Rust source (kept for reference)
```

## JSON Events (Spindle Contract)

The `--progress-json` flag emits newline-delimited JSON. Spindle depends on these event types:

| Event | Key Fields |
|-------|------------|
| `encoding_progress` | `type`, `percent`, `speed`, `fps`, `eta_seconds`, `timestamp` |
| `validation_complete` | `type`, `validation_passed`, `validation_steps[]`, `timestamp` |
| `encoding_complete` | `type`, `output_file`, `original_size`, `encoded_size`, `size_reduction_percent`, `timestamp` |
| `warning` | `type`, `message`, `timestamp` |
| `error` | `type`, `title`, `message`, `context`, `suggestion`, `timestamp` |
| `batch_complete` | `type`, `successful_count`, `total_files`, `total_size_reduction_percent`, `timestamp` |

Progress throttling: emit on 1% bucket change OR 5 second minimum interval.

Schema is defined in `internal/reporter/json.go` (`JSONReporter`).

## Entry Points

| Task | Start Here |
|------|------------|
| Add/modify encoding parameters | `internal/config/config.go` (presets), `internal/ffmpeg/command.go` |
| Change crop detection | `internal/processing/crop.go` |
| Add validation check | `internal/validation/validate.go` |
| Modify JSON output | `internal/reporter/json.go` |
| Change terminal output | `internal/reporter/terminal.go` |
| HDR detection | `internal/mediainfo/mediainfo.go`, `internal/ffprobe/ffprobe.go` |
| Public API | `drapto.go` |
| CLI flags | `cmd/drapto/main.go` |

## Library Usage (Spindle Integration)

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

## Principles

1. Keep it simple. This is a small hobby project maintained by a single developer.
2. Avoid scope creep and overengineering.
3. Prefer unit tests over running actual encodes (encoding is slow).
4. When running drapto with a timeout, use at least 120 seconds so encoding steps can complete.
5. Do not break the JSON event schema without updating Spindle.

## Go vs Rust Reference

The `reference/` directory contains the original Rust implementation. The Go rewrite has **full feature parity** with Rust. Do not report these as gaps:

| Feature | Go Location | Status |
|---------|-------------|--------|
| Film grain CLI flags | `cmd/drapto/main.go:73-74,179-183` | Implemented |
| Film grain public API | `drapto.go:131-143` (`WithFilmGrain`, `WithFilmGrainDenoise`) | Implemented |
| Film grain config | `internal/config/config.go:161-162` | Implemented |
| All presets (grain/clean/quick) | `internal/config/config.go` | Implemented |
| Crop detection (141 samples) | `internal/processing/crop.go` | Implemented |
| HDR detection | `internal/mediainfo/mediainfo.go` | Implemented |
| Validation (7 checks) | `internal/validation/validate.go` | Implemented |
| Spindle JSON contract | `internal/reporter/json.go` | Implemented |

When comparing Go to Rust, verify features by reading the actual Go code before reporting gaps.
