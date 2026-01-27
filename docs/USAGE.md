# Usage Guide

Run `drapto encode --help` for the authoritative flag list. The sections below provide practical context.

## CLI Basics

```bash
# Basic foreground encode
drapto encode -i input.mkv -o output/

# Batch encode an entire directory
drapto encode -i /videos/ -o /encoded/

# Override defaults
drapto encode -i input.mkv -o output/ --crf 25,24,29 --preset 6

# Verbose output
drapto encode -v -i input.mkv -o output/
```

## Frequently Used Options

**Required**
- `-i, --input <PATH>`: Input file or directory containing `.mkv` files
- `-o, --output <DIR>`: Output directory (or filename when single file)

**Common**
- `-v, --verbose`: Verbose output with detailed status
- `-l, --log-dir <DIR>`: Override the log directory (defaults to `~/.local/state/drapto/logs`)
- `--preset <0-13>`: SVT-AV1 encoder speed/quality (default `6`, lower is slower but higher quality)
- `--drapto-preset <grain|clean|quick>`: Project-defined bundles that set CRF, SVT preset, and AC bias
- `--crf <VALUE>`: CRF quality (0-63). Single value or SD,HD,UHD triple. Default: 25,27,29
- `--responsive`: Use `nice -n 19` so other apps stay responsive
- `--disable-autocrop`: Skip black-bar detection and cropping

CLI overrides such as `--crf` still take precedence over the preset-provided values, so you can start from a profile and tweak selectively per encode.

## Preset Profiles

| Profile | CRF (SD/HD/UHD) | SVT Preset | AC Bias | Intent |
|---------|-----------------|------------|---------|--------|
| _(defaults)_ | 25 / 27 / 29 | 6 | 0.10 | Balanced quality/size |
| `grain` | 25 / 27 / 29 | 6 | 0.10 | Placeholder for future film-grain tuning |
| `clean` | 27 / 29 / 31 | 6 | 0.05 | Already clean/animated content |
| `quick` | 32 / 35 / 36 | 8 | 0.10 | Fast, non-archival encodes |

Each preset maps to a `PresetValues` struct inside `internal/config/config.go`. For deeper guidance (including how to edit the constants), see `docs/PRESETS.md`.

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

Foreground runs show real-time progress with ETA, fps, bitrate, and reduction stats. For automation, use the library API with a custom event handler (see `docs/spindle-integration.md`).

## Environment Variables

- `NO_COLOR`: Disable colored output

## Debugging

Use the `--verbose` flag to enable detailed output:

```bash
drapto encode -v -i input.mkv -o output/
```

Log files are written to `~/.local/state/drapto/logs` by default. Use `--log-dir` to override or `--no-log` to disable.
