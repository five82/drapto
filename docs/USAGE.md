# Usage Guide

This guide collects the detailed CLI switches and behavior notes that were previously in the top-level README. Run `drapto encode --help` for the authoritative flag list; the sections below provide practical context.

## CLI Basics

```bash
# Basic foreground encode
drapto encode -i input.mkv -o output/

# Batch encode an entire directory
drapto encode -i /videos/ -o /encoded/

# Override defaults
drapto encode -i input.mkv -o output/ --quality-hd 24 --preset 6

# Verbose output
drapto encode -v -i input.mkv -o output/
```

## Frequently Used Options

**Required**
- `-i, --input <PATH>`: Input file or directory containing `.mkv` files
- `-o, --output <DIR>`: Output directory (or filename when single file)

**Common**
- `-v, --verbose`: Verbose output with detailed status
- `--no-color`: Disable colored output
- `-l, --log-dir <DIR>`: Override the log directory (defaults to `<output>/logs`)
- `--preset <0-13>`: SVT-AV1 encoder speed/quality (default `6`, lower is slower but higher quality)
- `--drapto-preset <grain|clean>`: Project-defined bundles that set CRF, SVT preset/tune, AC bias, and variance boost together
- `--quality-sd/hd/uhd <CRF>`: Override CRF defaults (SD=24, HD=26, UHD=28)
- `--responsive`: Reserve a few CPU threads so other apps stay responsive
- `--disable-autocrop`: Skip black-bar detection and cropping
- `--progress-json`: Emit structured progress events (used by Spindle and other automation)

CLI overrides such as `--quality-hd` still take precedence over the preset-provided values, so you can start from a profile and tweak selectively per encode.

## Preset Profiles

| Profile | CRF (SD/HD/UHD) | SVT Preset | Tune | AC Bias | Variance Boost | Boost Strength | Octile |
|---------|-----------------|------------|------|---------|----------------|----------------|--------|
| _Base defaults (no preset)_ | 24 / 26 / 28 | 6 | 0 | 0.30 | Enabled | 1 | 6 |
| `grain` | 22 / 24 / 26 | 5 | 0 | 0.50 | Enabled | 2 | 5 |
| `clean` | 26 / 28 / 30 | 6 | 0 | 0.20 | Disabled | 0 | 0 |

Each preset maps to a `DraptoPresetValues` struct inside `drapto-core/src/config/mod.rs`. For deeper guidance (including how to edit the constants), see `docs/PRESETS.md`.

## HDR Support

Drapto automatically detects and preserves HDR content using MediaInfo for color space analysis:
- Detects HDR based on color primaries (BT.2020, BT.2100)
- Recognizes HDR transfer characteristics (PQ, HLG)
- Adapts processing parameters and metadata handling for HDR sources

## Post-Encode Validation

Validation catches mismatches before you archive or publish results:
- **Video codec**: Ensures AV1 output and 10-bit depth
- **Audio codec**: Confirms all audio streams are transcoded to Opus with the expected track count
- **Dimensions**: Validates crop detection and output dimensions
- **Duration**: Compares input and output durations
- **HDR / Color space**: Uses MediaInfo to verify HDR flags and colorimetry
- **Failure reporting**: Emits warnings/errors plus JSON events for automation

## Multi-Stream Audio Handling

- Automatically detects every audio stream and transcodes each to Opus
- Bitrate allocation per channel layout:
  - Mono: 64 kbps
  - Stereo: 128 kbps
  - 5.1: 256 kbps
  - 7.1: 384 kbps
  - Custom layouts: 48 kbps per channel

## Progress Reporting

Foreground runs show real-time progress with ETA, fps, bitrate, and reduction stats; automation can use the JSON stream (`encoding_progress`, `validation_complete`, `encoding_complete`, `warning`, `error`, `batch_complete`).

## Environment Variables

- `NO_COLOR`: Disable colored output
- `RUST_LOG`: Control logging verbosity (`debug`, `trace`, etc.)

## Debugging

```bash
# Debug-level logging
RUST_LOG=debug drapto encode -i input.mkv -o output/

# Trace-level logging
RUST_LOG=trace drapto encode --interactive -i input.mkv -o output/
```
