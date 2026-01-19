# Usage Guide

Run `drapto encode --help` for the authoritative flag list. The sections below provide practical context.

## CLI Basics

```bash
# Basic encode
drapto encode -i input.mkv -o output/

# Batch encode an entire directory
drapto encode -i /videos/ -o /encoded/

# Override quality settings
drapto encode -i input.mkv -o output/ --crf 24 --preset 6

# Target Quality mode (GPU-accelerated)
drapto encode -i input.mkv -o output/ -t 70-75

# Verbose output
drapto encode -v -i input.mkv -o output/
```

## Frequently Used Options

**Required**
- `-i, --input <PATH>`: Input file or directory containing video files
- `-o, --output <DIR>`: Output directory (or filename when single file)

**Quality Settings**
- `--crf <0-63>`: CRF quality level (default `27`, lower is better quality)
- `--preset <0-13>`: SVT-AV1 encoder speed/quality (default `6`, lower is slower but higher quality)

**Target Quality** (per-chunk SSIMULACRA2 targeting)
- `-t, --target <RANGE>`: Target SSIMULACRA2 range (e.g., `70-75`)
- `--qp <RANGE>`: CRF search range (default `8-48`)
- `--metric-workers <N>`: Number of GPU metric workers (default `1`)
- `--metric-mode <MODE>`: Metric aggregation mode (`mean` or `pN`, default `mean`)

**Processing**
- `--workers <N>`: Number of parallel encoder workers (auto-detected by default)
- `--buffer <N>`: Extra chunks to buffer in memory (auto-matched to workers)
- `--scene-threshold <N>`: Scene detection threshold 0.0-1.0 (default `0.5`, higher = fewer scenes)
- `--disable-autocrop`: Skip black-bar detection and cropping
- `--responsive`: Reserve CPU threads so other apps stay responsive

**TQ Sampling** (for faster probing in Target Quality mode)
- `--sample-duration <N>`: Seconds to sample for TQ probing (default `3.0`)
- `--sample-min-chunk <N>`: Minimum chunk duration to use sampling (default `6.0`)
- `--no-tq-sampling`: Disable sample-based probing (use full chunks)

**Output**
- `-l, --log-dir <DIR>`: Override the log directory (defaults to `<output>/logs`)
- `-v, --verbose`: Verbose output with detailed status
- `--no-log`: Disable log file creation

## Parallel Chunked Encoding

Drapto splits videos at scene boundaries and encodes chunks in parallel:

```bash
# Auto-detected parallelism (1 worker per 8 CPU cores, max 4)
drapto encode -i input.mkv -o output/

# Manual worker count
drapto encode -i input.mkv -o output/ --workers 8 --buffer 8
```

Scene detection is controlled by `--scene-threshold`:
- Lower values (0.1-0.3): More scenes, smaller chunks
- Default (0.5): Balanced scene splitting
- Higher values (0.7-0.9): Fewer scenes, larger chunks

## Target Quality Mode

Instead of fixed CRF, Target Quality mode finds the optimal CRF for each chunk to achieve consistent perceptual quality. Requires an NVIDIA GPU with libvship installed.

```bash
# Target SSIMULACRA2 score of 70-75
drapto encode -i input.mkv -o output/ -t 70-75

# With more workers for high-end GPUs
drapto encode -i input.mkv -o output/ -t 70-75 --metric-workers 2
```

See [docs/target-quality.md](target-quality.md) for detailed guidance on quality targets.

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
- **Duration**: Compares input and output durations (±1 second tolerance)
- **HDR / Color space**: Uses MediaInfo to verify HDR flags and colorimetry
- **Audio sync**: Verifies audio drift is within 100ms tolerance

## Multi-Stream Audio Handling

- Automatically detects every audio stream and transcodes each to Opus
- Bitrate allocation per channel layout:
  - Mono: 64 kbps
  - Stereo: 128 kbps
  - 5.1: 256 kbps
  - 7.1: 384 kbps
  - Custom layouts: 48 kbps per channel

## Progress Reporting

Foreground runs show real-time progress with ETA, fps, and reduction stats. For automation, use the library API with a custom event handler (see [docs/spindle-integration.md](spindle-integration.md)).

## Environment Variables

- `NO_COLOR`: Disable colored output

## Debugging

```bash
# Verbose logging
drapto encode -v -i input.mkv -o output/

# Check log files in output directory
ls output/logs/
```
