# drapto

FFmpeg wrapper for AV1 encoding with SVT-AV1 and Opus audio. Uses opinionated defaults so you can encode without dealing with ffmpeg's complexity.

## Features

- Automatic black bar crop detection
- HDR10/HLG metadata preservation
- Resolution-based CRF defaults (SD/HD/UHD)
- Multi-track audio transcoding to Opus
- Post-encode validation (codec, dimensions, duration, HDR)
- Preset profiles: `grain`, `clean`, `quick`
- JSON progress output for automation

## Requirements

- FFmpeg with `libsvtav1` and `libopus`
- MediaInfo

## Install

```bash
cargo install --git https://github.com/five82/drapto
```

## Usage

```bash
drapto encode -i input.mkv -o output/
drapto encode -i /videos/ -o /encoded/
drapto encode -i input.mkv -o output/ --drapto-preset grain
drapto encode -i input.mkv -o output/ --progress-json
```

See [docs/USAGE.md](docs/USAGE.md) for all options and [docs/PRESETS.md](docs/PRESETS.md) for preset details.

## Project Structure

- **drapto-cli** - CLI, progress display, JSON output
- **drapto-core** - Video analysis, ffmpeg integration, validation

## Development

```bash
cargo build --release
RUST_LOG=debug cargo run -- encode -i input.mkv -o output/
```
