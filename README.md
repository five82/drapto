# Drapto

Optimized AV1 video encoding tool with intelligent quality control, scene-based segmentation, and parallel processing.

Drapto automatically detects HDR/SDR content and applies the most appropriate encoding strategy - using CRF mode for HDR content and VMAF-targeted quality for SDR content.

## Features

- **AV1 Encoding with SVT-AV1:** High-quality encoding using libsvtav1 with configurable presets
- **Content-Aware Quality Control:** Automatically selects optimal encoding mode:
  - CRF (Constant Rate Factor) for HDR content
  - VMAF-targeted quality for SDR content
- **Adaptive Scene Detection:** Intelligently segments videos at scene boundaries for better compression
- **Parallel Processing:** Encodes multiple segments concurrently with memory-aware scheduling
- **Comprehensive Validation:** Ensures quality, integrity, sync, and format compliance
- **Automatic Black Bar Detection:** Detects and removes black bars for optimal viewing experience
- **High-Quality Audio:** Uses Opus codec with channel-adaptive bitrates
- **Hardware Acceleration:** Supports hardware-accelerated decoding when available

## Requirements

- Rust 1.76+
- FFmpeg 7.0+ with support for libsvtav1, libvmaf, and libopus
- MediaInfo (for detailed media analysis and HDR detection)
- ab-av1 (for quality-targeted encoding)

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/drapto.git
cd drapto

# Install dependencies (Ubuntu/Debian example)
sudo apt install ffmpeg mediainfo
cargo install ab-av1

# Build and install drapto
cargo build --release
cargo install --path .
```

## Usage

```bash
# Basic encoding (automatically detects HDR/SDR and uses appropriate mode)
drapto encode --input input.mkv --output output.mkv

# Analyze a media file
drapto info input.mkv

# Validate an encoded file
drapto validate --input encoded.mp4 --reference original.mkv

# Advanced encoding options
drapto encode --input input.mkv --output output.mkv --preset 4 --parallel-jobs 4 --target-quality 95

# Force CRF mode for all content
drapto encode --input input.mkv --output output.mkv --use-crf
```

### Encoding Pipeline

Drapto processes videos through an intelligent pipeline:

1. **Media Analysis:** Detects HDR/SDR content, dimensions, and formats
2. **Content-Aware Mode Selection:** CRF for HDR, VMAF for SDR (customizable)
3. **Scene Detection:** Segments video at scene boundaries for optimal compression
4. **Parallel Encoding:** Processes segments concurrently with retry capability
5. **Final Assembly:** Concatenates segments and validates the output

## Configuration

Drapto provides multiple ways to configure its behavior, in order of precedence:

1. **Command-line arguments:** Directly passed when running drapto (highest priority)
2. **Environment variables:** Set variables with `DRAPTO_` prefix
3. **Configuration file:** TOML format (drapto.toml)
4. **Default values:** Built-in defaults (lowest priority)

Configuration is organized into logical sections (video, audio, validation, resources, etc.) with sensible defaults that adapt to your content.

### Example Configuration

Create a file named `drapto.toml` in your working directory:

```toml
# Basic input/output paths (can be overridden by command-line arguments)
input = "input.mkv"
output = "output.mp4"

[video]
# Quality settings for different content types
# HDR automatically uses CRF mode, SDR uses VMAF by default
target_vmaf = 93.0        # Target quality for SDR content
target_vmaf_hdr = 95.0    # Only used if CRF is disabled for HDR

# CRF values (used by default for HDR content)
target_crf_sd = 25        # For SD (<1280p)
target_crf_hd = 28        # For HD (1280-3839p)
target_crf_4k = 28        # For 4K (≥3840p)

# Encoding settings
preset = 6                # 0-13, lower = better quality but slower
encoder = "libsvtav1"     # AV1 encoder implementation
pixel_format = "yuv420p10le"  # 10-bit for better HDR support
keyframe_interval = "10s" # Keyframe interval

# Processing options
use_segmentation = true   # Enable scene-based segmentation
disable_crop = false      # Enable automatic black bar detection

[scene_detection]
# Scene detection tuning
scene_threshold = 40.0    # Content difference threshold for SDR
hdr_scene_threshold = 30.0 # Threshold for HDR (lower for better detection)
min_segment_length = 5.0  # Minimum segment duration in seconds

[audio]
# Opus audio encoding
compression_level = 10    # Maximum quality
vbr = true                # Variable bitrate
application = "audio"     # Optimized for music/movies

[audio.bitrates]
# Channel-specific bitrates (kbps)
stereo = 128
surround_5_1 = 256

[resources]
# Resource management
parallel_jobs = 0         # Auto-detect based on CPU cores
memory_threshold = 0.7    # Maximum memory fraction to use
memory_per_job = 2048     # MB per encoding job
```

For detailed configuration options, see the [Configuration Guide](docs/configuration.md).

## Advanced Usage

### Common Scenarios

- **Maximum Quality:** Use `--preset 4 --target-quality 95` for highest quality SDR encoding
- **Faster Encoding:** Use `--preset 8 --target-quality 90` for faster encoding with good quality
- **HDR Content:** HDR content automatically uses CRF mode by default
- **Force Encoding Mode:** Use `--use-crf` to enforce CRF mode for all content

### Tips & Troubleshooting

- **Logging:** Detailed logs are saved in `~/drapto_logs` (customizable)
- **Temporary Files:** Working files are stored in `/tmp/drapto` by default and automatically cleaned up
- **Hardware Acceleration:** Automatically used for decoding when available
- **Memory Management:** Configure `memory_per_job` if encoding fails due to memory limits
- **Common Issues:** 
  - Ensure FFmpeg 7.0+ with libsvtav1 and libvmaf is installed
  - MediaInfo is required for accurate HDR detection
  - Verify input files are valid and readable
  - Provide sufficient disk space for temporary segments

For complete documentation, see the [Configuration Guide](docs/configuration.md).
