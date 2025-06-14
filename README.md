# Drapto

Advanced ffmpeg video encoding wrapper with intelligent optimization for high-quality, efficient AV1 encodes.

## Features

* **Intelligent Analysis**: Automatic black bar cropping, HDR-aware processing
* **Optimized Encoding**: AV1 video (libsvtav1), Opus audio with multi-stream support, resolution-based quality settings
* **Conservative Denoising**: Light denoising with film grain synthesis for optimal quality/size balance
* **Hardware Acceleration**: VideoToolbox decoding support on macOS (automatically detected)
* **Flexible Workflow**: Daemon mode for background processing, interactive mode with progress bars, push notifications

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

# Encode directory (processes all .mkv files)
drapto encode -i /videos/ -o /encoded/

# Interactive mode (foreground with progress bars)
drapto encode --interactive -i input.mkv -o output/

# Custom settings
drapto encode -i input.mkv -o output/ --quality-hd 24 --preset 4

# With notifications
drapto encode -i input.mkv -o output/ --ntfy https://ntfy.sh/your_topic

# Verbose output
drapto encode -v -i input.mkv -o output/
```

## Key Options

**Required:**
* `-i, --input <PATH>`: Input file or directory containing .mkv files
* `-o, --output <DIR>`: Output directory (or filename if single file)

**Common Options:**
* `--interactive`: Run in foreground instead of daemon mode
* `-v, --verbose`: Enable verbose output with detailed information
* `--no-color`: Disable colored output
* `-l, --log-dir <DIR>`: Directory for log files (defaults to OUTPUT_DIR/logs)
* `--preset <0-13>`: SVT-AV1 encoder speed/quality (default: 4, lower = slower/better)
* `--quality-sd/hd/uhd <CRF>`: Override quality settings (defaults: SD=23, HD=25, UHD=27)
* `--no-denoise`: Disable denoising and film grain synthesis
* `--disable-autocrop`: Disable black bar cropping
* `--ntfy <URL>`: Send notifications to ntfy.sh

## Advanced Features

### Conservative Denoising

Drapto applies a fixed, conservative denoising approach that adapts to content type:

1. **HDR-Aware Denoising**:
   - HDR content: `hqdn3d=1:0.8:2.5:2` (very light denoising to preserve detail)
   - SDR content: `hqdn3d=2:1.5:3:2.5` (slightly stronger but still conservative)
2. **Film Grain Synthesis**: Adds level 4 synthetic grain to maintain natural texture
3. **Simple Control**: Enabled by default, disable with `--no-denoise`

This reduces file size while maintaining excellent visual quality. The denoising parameters are intentionally conservative to avoid visible quality loss at normal viewing distances.

### HDR Support

Automatically detects and preserves HDR content using MediaInfo for comprehensive color space analysis:
- Detects HDR based on color primaries (BT.2020, BT.2100)
- Recognizes HDR transfer characteristics (PQ, HLG)
- Adapts processing parameters for HDR content

### Post-Encode Validation

Drapto includes comprehensive validation to ensure encoding success:

* **Video Codec**: Verifies AV1 encoding and 10-bit depth
* **Audio Codec**: Confirms Opus audio encoding and track count
* **Dimensions**: Validates crop detection and output dimensions
* **Duration**: Ensures encoded duration matches input
* **HDR/Color Space**: Uses MediaInfo to verify HDR content preservation and color space accuracy
* **Failure Reporting**: Logs and notifies about validation issues

### Hardware Acceleration

* VideoToolbox hardware decoding on macOS (automatically enabled when available)
* Improves performance by hardware-accelerating video decoding

### Multi-Stream Audio

* Automatic detection of all audio streams
* Channel-based bitrate allocation:
  * Mono: 64 kbps
  * Stereo: 128 kbps
  * 5.1: 256 kbps
  * 7.1: 384 kbps
  * Custom: 48 kbps per channel

### Progress Reporting

In interactive mode:
* Real-time encoding progress with ETA
* Current encoding speed (fps)
* Bitrate monitoring
* File size reduction calculations

### Environment Variables

* `DRAPTO_NTFY_TOPIC`: Default ntfy.sh topic URL
* `NO_COLOR`: Disable colored output
* `RUST_LOG`: Control logging level (e.g., `debug`, `trace`)

### Notifications

Push notifications via ntfy.sh include:
* Encode start notification
* Encode completion with file sizes and duration
* Error notifications
* Size reduction percentage

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
  * Daemon/interactive mode handling
  * Progress reporting and user feedback
  * Terminal color support

* **drapto-core**: Core video processing library
  * Video analysis (crop detection, video properties)
  * FFmpeg/FFprobe and MediaInfo integration
  * Audio stream processing and validation
  * Post-encode validation system
  * Notification services (ntfy.sh)
  * Hardware acceleration detection
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
