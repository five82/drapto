# Drapto

Advanced ffmpeg video encoding wrapper with intelligent optimization for high-quality, efficient AV1 encodes.

## Features

* **Intelligent Analysis**: Automatic black bar cropping, film grain detection with XPSNR quality metrics, HDR-aware processing
* **Optimized Encoding**: AV1 video (libsvtav1), Opus audio with multi-stream support, resolution-based quality settings
* **Hardware Acceleration**: VideoToolbox decoding support on macOS (automatically detected)
* **Flexible Workflow**: Daemon mode for background processing, interactive mode with progress bars, push notifications

## Installation

1. **Prerequisites**: Install ffmpeg with libsvtav1 and libopus support
   ```bash
   # Ubuntu/Debian
   sudo apt install ffmpeg

   # macOS
   brew install ffmpeg
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
drapto encode -i input.mkv -o output/ --quality-hd 24 --preset 6

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
* `--preset <0-13>`: SVT-AV1 encoder speed/quality (default: 6, lower = slower/better)
* `--quality-sd/hd/uhd <CRF>`: Override quality settings (defaults: SD=25, HD=27, UHD=27)
* `--no-denoise`: Disable grain analysis and denoising
* `--disable-autocrop`: Disable black bar cropping
* `--ntfy <URL>`: Send notifications to ntfy.sh

**Advanced Grain Analysis Options:**
* `--grain-sample-duration <SECONDS>`: Sample duration for analysis (default: 10)
* `--grain-knee-threshold <0.0-1.0>`: Knee point detection threshold (default: 0.8)
* `--grain-max-level <LEVEL>`: Maximum denoising level constraint

## Advanced Features

### Intelligent Grain Processing

Drapto uses a sophisticated multi-step approach for optimal compression:

1. **Multi-Sample Analysis**: Extracts random samples from the video
2. **XPSNR Quality Metrics**: Measures quality at different denoising levels
3. **Knee Point Detection**: Finds optimal balance between compression and quality
4. **Adaptive Denoising**: Applies hqdn3d filter with calculated strength
5. **Synthetic Grain**: Adds controlled grain during encoding for natural appearance

Grain levels detected: Baseline, VeryLight, Light, LightModerate, Moderate, Elevated

This achieves 20-40% file size reduction while maintaining visual quality.

### HDR Support

Automatically detects and preserves HDR content (BT.2020 color space) with adapted processing parameters.

### Hardware Acceleration

* VideoToolbox hardware decoding on macOS (automatically enabled when available)
* Disabled during grain analysis for consistency

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
  * Video analysis (crop detection, grain analysis)
  * FFmpeg/FFprobe integration
  * Audio stream processing
  * Notification services
  * Hardware acceleration detection
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
