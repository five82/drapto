# drapto

High-quality AV1 encoding wrapper that layers intelligent analysis and ergonomic reporting atop FFmpeg. Drapto ships sane defaults so you can run it once manually or wire it into an automated workflow like Spindle.

## Why Drapto

- Automatic black-bar cropping plus HDR-aware processing
- SVT-AV1 + Opus pipelines tuned per resolution, with preset bundles for common scenes
- Foreground-friendly UX: concise terminal sections, colored progress bar, and JSON progress stream for automation
- Post-encode validation covering codecs, dimensions, HDR metadata, and duration sanity checks

## Install

1. Install FFmpeg (with `libsvtav1` + `libopus`) and MediaInfo.
2. Install Drapto from source:
   ```bash
   cargo install --git https://github.com/five82/drapto
   ```

## Quick Start

```bash
# Encode one file
drapto encode -i input.mkv -o output/

# Encode an entire directory
drapto encode -i /videos/ -o /encoded/
```

Need granular flag descriptions, preset tables, and diagnostics? See [`docs/USAGE.md`](docs/USAGE.md) plus [`docs/PRESETS.md`](docs/PRESETS.md).

## Project Layout

- `drapto-cli`: CLI entrypoint, argument parsing, colored terminal output, progress + JSON reporting
- `drapto-core`: Video/audio analysis, FFmpeg/FFprobe integration, preset logic, validation, and automation hooks

## Development

```bash
git clone https://github.com/five82/drapto.git
cd drapto
./build.sh # or cargo build --release
./target/release/drapto --help
```

Enable additional logging with `RUST_LOG=debug` or `trace`. Unit tests are preferred over full encodes for iteration speed.
