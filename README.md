# drapto

FFmpeg video encoding wrapper with intelligent optimization for high-quality, efficient AV1 encodes. The purpose is to apply sane defaults to encodes so you can run and forget or incorporate drapto into an automated workflow.

## Features

* **Intelligent Analysis**: Automatic black bar cropping, HDR-aware processing, adaptive noise analysis
* **Optimized Encoding**: AV1 video (libsvtav1), intelligent audio processing with spatial audio preservation and Opus transcoding, resolution-based quality settings
* **Adaptive Denoising**: Intelligent noise detection with tailored denoising and film grain synthesis
* **Hardware Acceleration**: VideoToolbox and VAAPI decoding support (automatically detected)
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

# Encode directory (processes all video files)
drapto encode -i /videos/ -o /encoded/

# Foreground mode (with progress bars and terminal output)
drapto encode --foreground -i input.mkv -o output/

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
* `--foreground`: Run in foreground instead of daemon mode
* `-v, --verbose`: Enable verbose output with detailed information
* `--no-color`: Disable colored output
* `-l, --log-dir <DIR>`: Directory for log files (defaults to OUTPUT_DIR/logs)
* `--preset <0-13>`: SVT-AV1 encoder speed/quality (default: 4, lower = slower/better)
* `--quality-sd/hd/uhd <CRF>`: Override quality settings (defaults: SD=23, HD=25, UHD=27)
* `--no-denoise`: Disable denoising and film grain synthesis
* `--responsive`: Reserve a few CPU threads so other applications stay responsive
* `--disable-autocrop`: Disable black bar cropping
* `--ntfy <URL>`: Send notifications to ntfy.sh

## Advanced Features

### Adaptive Denoising

Drapto uses intelligent noise analysis to apply optimal denoising settings for each video:

1. **Noise Analysis**: Automatically analyzes video noise levels using FFmpeg's bitplanenoise filter
2. **Adaptive Parameters**: Selects appropriate denoising strength based on detected noise:
   - Very clean content: Minimal denoising (preserves pristine quality)
   - Slightly noisy content: Very light denoising
   - Somewhat noisy content: Light denoising
   - Noisy content: Moderate denoising (still conservative)
3. **HDR-Aware Processing**: Uses lighter denoising parameters for HDR content to preserve detail
4. **Dynamic Film Grain**: Scales film grain synthesis (levels 4-16) to compensate for denoising artifacts
5. **Quality-First Approach**: All parameters remain conservative to avoid visible quality loss

This system reduces file size while maintaining visual quality by applying a conservative amount of denoising for each video's noise characteristics.

### HDR Support

Automatically detects and preserves HDR content using MediaInfo for comprehensive color space analysis:
- Detects HDR based on color primaries (BT.2020, BT.2100)
- Recognizes HDR transfer characteristics (PQ, HLG)
- Adapts processing parameters for HDR content

### Post-Encode Validation

Includes comprehensive validation to ensure encoding success:

* **Video Codec**: Verifies AV1 encoding and 10-bit depth
* **Audio Codec**: Validates audio processing based on stream type:
  * Spatial audio streams: Confirms preservation of original codecs (TrueHD/DTS)
  * Non-spatial streams: Confirms Opus transcoding
  * Mixed content: Validates each stream according to its processing method
* **Dimensions**: Validates crop detection and output dimensions
* **Duration**: Ensures encoded duration matches input
* **HDR/Color Space**: Uses MediaInfo to verify HDR content preservation and color space accuracy
* **Failure Reporting**: Logs and notifies about validation issues

### Hardware Acceleration

* VideoToolbox and VAAPI hardware decoding support (automatically enabled when available)
* Improves performance by hardware-accelerating video decoding

### Multi-Stream Audio

* Automatic detection of all audio streams with intelligent spatial audio preservation
* **Spatial Audio Support**: Automatically detects and preserves spatial audio formats:
  * **Dolby Atmos**: TrueHD + Atmos and E-AC-3 + JOC (Joint Object Coding)
  * **DTS:X**: DTS with DTS:X profile variations
  * Spatial audio streams are copied unchanged to preserve object-based metadata
* **Mixed Audio Processing**: Handles files with both spatial and non-spatial audio streams
  * Spatial streams: Copied to preserve quality and metadata
  * Non-spatial streams: Transcoded to Opus with optimized bitrates
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
