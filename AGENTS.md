# AGENTS.md

This file provides guidance when working with code in this repository. 

CLAUDE.md and GEMINI.md are symlinks to this file so all agent guidance stays in one place.
Do not modify this header.

Do not run `git commit` or `git push` unless explicitly instructed.

## System Dependencies

Building requires `libmediainfo-dev` and `pkg-config`:

```bash
# Ubuntu/Debian
sudo apt-get install libmediainfo-dev pkg-config

# Runtime also requires FFmpeg with libsvtav1 and libopus, plus MediaInfo
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

## Project Overview

Drapto is an ffmpeg wrapper for AV1 encoding with SVT-AV1 and Opus audio. It uses opinionated defaults so you can encode without dealing with ffmpeg's complexity. Features include automatic crop detection, HDR metadata preservation, and post-encode validation.

## Architecture

The project follows a modular Rust workspace architecture with two main components:

1. **drapto-cli**: Command-line interface and user interaction
   - Handles argument parsing, logging, and progress orchestration
   - Provides user-friendly command interface
   - Manages progress reporting and feedback

2. **drapto-core**: Core video processing and analysis library
   - Video analysis (crop detection, HDR awareness)
   - FFmpeg integration and command generation
   - Video encoding orchestration

## Development Commands

```bash
# Build
cargo build --release

# Run
cargo run -- encode -i input.mkv -o output/
RUST_LOG=debug cargo run -- encode -i input.mkv -o output/

# Test
cargo test --workspace              # all tests
cargo test -p drapto-core           # just core library
cargo test test_name                # single test by name

# Lint and format
cargo clippy --workspace
cargo fmt --check                   # check formatting
cargo fmt                           # fix formatting
```

## Key Components

### FFmpeg Integration

The project uses FFmpeg for video processing via:

1. `ffmpeg-sidecar` for command execution
2. `ffprobe` for media file analysis
3. Custom command builders that generate optimized encoding commands

### Progress Reporting

The progress reporting system provides feedback during long-running operations:

1. Terminal-based progress bars for foreground mode
2. Detailed logging for automated consumers

### CLI Output Style

1. Render just four sections in human mode: Hardware → Video → Encoding → Validation → Results; keep each section to a handful of lines.
2. Use plain `println!` plus `indicatif` for the progress bar—no template engine or ASCII art. Colors are fine when they reinforce meaning (cyan headers, magenta stage bullets, green checkmarks, yellow warnings, red errors).
3. Show real progress information only once: rely on the progress bar during encoding, and print a single validation summary after it completes.
4. Keep the JSON stream (`--progress-json`) backward-compatible with the existing objects Spindle consumes (`encoding_progress`, `validation_complete`, `encoding_complete`, `warning`, `error`, `batch_complete`).
5. Prefer natural language sentences (“Encoding finished successfully”) and reserve emphatic formatting for values that matter (reduction %, warnings, output paths).

## Project Structure (drapto-core/src/)

- **config/** - `CoreConfig`, preset definitions (`DraptoPresetValues`)
- **discovery.rs** - File discovery and filtering
- **external/** - FFmpeg, FFprobe, MediaInfo command execution
  - `ffmpeg_builder.rs` - Builds ffmpeg command arguments
  - `ffprobe_executor.rs` - Extracts video metadata
  - `mediainfo_executor.rs` - HDR detection
- **processing/** - Core encoding logic
  - `video.rs` - Main encoding orchestration
  - `crop_detection.rs` - Black bar detection
  - `audio.rs` - Opus transcoding setup
  - `validation/` - Post-encode checks (codec, dimensions, duration, HDR)
- **reporting/** - Progress reporting (`Reporter` trait, `JsonReporter`, `TerminalReporter`)

## JSON Events (Spindle Contract)

The `--progress-json` flag emits newline-delimited JSON. Spindle depends on these event types:

| Event | Key Fields |
|-------|------------|
| `encoding_progress` | `percent`, `speed`, `fps`, `eta_seconds` |
| `validation_complete` | `validation_passed`, `validation_steps[]` |
| `encoding_complete` | `output_file`, `original_size`, `encoded_size`, `size_reduction_percent` |
| `warning` | `message` |
| `error` | `title`, `message`, `context`, `suggestion` |
| `batch_complete` | `successful_count`, `total_files`, `total_size_reduction_percent` |

All events include a `timestamp` field. Schema is defined in `reporting/mod.rs` (`JsonReporter` impl).

## Entry Points

| Task | Start Here |
|------|------------|
| Add/modify encoding parameters | `config/mod.rs` (presets), `external/ffmpeg_builder.rs` (command args) |
| Change crop detection | `processing/crop_detection.rs` |
| Add validation check | `processing/validation/` |
| Modify JSON output | `reporting/mod.rs` (`JsonReporter`) |
| Change terminal output | `reporting/mod.rs` (`TerminalReporter`) |
| HDR detection | `external/mediainfo_executor.rs` |

## Principles

1. Keep it simple. This is a small hobby project maintained by a single developer.
2. Avoid scope creep and overengineering.
3. Prefer unit tests over running actual encodes (encoding is slow).
4. When running drapto with a timeout, use at least 120 seconds so encoding steps can complete.
5. Do not break the JSON event schema without updating Spindle.
