# drapto

FFmpeg video encoding wrapper with intelligent optimization for high-quality, efficient AV1 encodes. The purpose is to apply sane defaults to encodes so you can run and forget or incorporate drapto into an automated workflow.

## Features

* **Intelligent Analysis**: Automatic black bar cropping and HDR-aware processing
* **Optimized Encoding**: AV1 video (libsvtav1) and Opus audio transcoding with resolution-based quality settings
* **Automation-Friendly Reporting**: Structured progress JSON stream plus rich TUI output for manual runs
* **Foreground Workflow**: Always runs in-process with progress bars; designed to be driven by Spindle or other orchestration tools

## Installation

1. **Prerequisites**: Install ffmpeg (with libsvtav1 and libopus support) and mediainfo
   ```bash
   # Ubuntu/Debian
   sudo apt install ffmpeg mediainfo

   # macOS
   brew install ffmpeg media-info
   ```

2. **Install Drapto**:
   ```bash
   cargo install --git https://github.com/five82/drapto
   ```

## Usage

```bash
# Basic usage (runs in background by default)
drapto encode -i input.mkv -o output/

# Encode directory (processes all video files)
drapto encode -i /videos/ -o /encoded/

# Custom settings
drapto encode -i input.mkv -o output/ --quality-hd 24 --preset 6

# Verbose output
drapto encode -v -i input.mkv -o output/
```

## Key Options

**Required:**
* `-i, --input <PATH>`: Input file or directory containing .mkv files
* `-o, --output <DIR>`: Output directory (or filename if single file)

**Common Options:**
* `-v, --verbose`: Enable verbose output with detailed information
* `--no-color`: Disable colored output
* `-l, --log-dir <DIR>`: Directory for log files (defaults to OUTPUT_DIR/logs)
* `--preset <0-13>`: SVT-AV1 encoder speed/quality (default: 6, lower = slower/better)
* `--drapto-preset <grain|clean>`: Apply a bundled profile that sets CRF, SVT preset/tune, AC bias, and variance boost values together (omit to keep the baseline defaults)
* `--quality-sd/hd/uhd <CRF>`: Override quality settings (defaults: SD=24, HD=26, UHD=28)
* `--responsive`: Reserve a few CPU threads so other applications stay responsive (disabled by default)
* `--disable-autocrop`: Disable black bar cropping (auto-crop is enabled by default)
* `--progress-json`: Emit structured progress events for external tools (also controls Spindle integration)

## Advanced Features

### Preset Profiles

`--drapto-preset` lets you switch between project-defined bundles without manually passing every quality/tuning flag. Each preset maps to a `DraptoPresetValues` struct inside `drapto-core/src/config/mod.rs`, so you can adjust the exact numbers in one place. Current presets ship with the following values:

| Profile | CRF (SD/HD/UHD) | SVT Preset | Tune | AC Bias | Variance Boost | Boost Strength | Octile |
|---------|-----------------|------------|------|---------|----------------|----------------|--------|
| _Base defaults (no preset)_ | 24 / 26 / 28 | 6 | 0 | 0.30 | Enabled | 1 | 6 |
| `grain` | 22 / 24 / 26 | 5 | 0 | 0.50 | Enabled | 2 | 5 |
| `clean` | 26 / 28 / 30 | 6 | 0 | 0.20 | Disabled | 0 | 0 |

CLI overrides such as `--quality-hd` or `--preset` still win over the preset-provided values, so you can start from a profile and tweak selectively per encode. For deeper guidance (including how to edit the constants), see [`docs/PRESETS.md`](docs/PRESETS.md).

### HDR Support

Automatically detects and preserves HDR content using MediaInfo for comprehensive color space analysis:
- Detects HDR based on color primaries (BT.2020, BT.2100)
- Recognizes HDR transfer characteristics (PQ, HLG)
- Adapts processing parameters for HDR content

### Post-Encode Validation

Includes comprehensive validation to ensure encoding success:

* **Video Codec**: Verifies AV1 encoding and 10-bit depth
* **Audio Codec**: Confirms all audio streams are transcoded to Opus with expected track count
* **Dimensions**: Validates crop detection and output dimensions
* **Duration**: Ensures encoded duration matches input
* **HDR/Color Space**: Uses MediaInfo to verify HDR content preservation and color space accuracy
* **Failure Reporting**: Logs and notifies about validation issues

### Multi-Stream Audio

* Automatic detection of all audio streams; every track is transcoded to Opus
* Channel-based bitrate allocation for Opus transcoding:
  * Mono: 64 kbps
  * Stereo: 128 kbps
  * 5.1: 256 kbps
  * 7.1: 384 kbps
  * Custom: 48 kbps per channel

### Progress Reporting

In foreground mode:
* Real-time encoding progress with ETA
* Current encoding speed (fps)
* Bitrate monitoring
* File size reduction calculations

### Environment Variables

* `NO_COLOR`: Disable colored output
* `RUST_LOG`: Control logging level (e.g., `debug`, `trace`)

## Building from Source

```bash
git clone https://github.com/five82/drapto.git
cd drapto
cargo build --release
./target/release/drapto --help
```

## Architecture

Drapto is built as a Rust workspace with two main components:

* **drapto-cli**: Command-line interface
  * Argument parsing and validation
  * Interactive progress reporting and JSON output selection
  * Progress reporting and user feedback
  * Terminal color support

* **drapto-core**: Core video processing library
  * Video analysis (crop detection, video properties)
  * FFmpeg/FFprobe and MediaInfo integration
  * Audio stream processing and validation
  * Post-encode validation system
  * Event-based progress reporting
  * System information collection
  * Temporary file management

## Debugging

Enable detailed logging for troubleshooting:

```bash
# Debug level
RUST_LOG=debug drapto encode -i input.mkv -o output/

# Trace level (very detailed)
RUST_LOG=trace drapto encode --interactive -i input.mkv -o output/
```

For all available options and examples, use `drapto encode --help`.
