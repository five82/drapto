# Drapto

ab-av1 video encoding wrapper with scene-based segmentation and parallel processing.

This is a vibe coding experiment to see how far LLM tools can take this. Pull requests are welcome.

## Features

- **AV1 Encoding with SVT-AV1:** High-quality encoding using libsvtav1 with configurable presets
- **Intelligent Scene-Based Segmentation:** Automatically segments videos using adaptive scene detection
- **Quality-Targeted Encoding:** Uses ab-av1 to achieve consistent quality with target VMAF
- **Parallel Encoding Pipeline:** Encodes segments concurrently with memory-aware scheduling
- **Enhanced Output Validation:** Performs comprehensive validation of video/audio streams, container integrity, sync, sample rates, and quality targets
- **Automatic Black Bar Detection and Cropping:** Detects black bars and applies appropriate crop filters
- **Quality Audio Encoding:** High quality Opus encoding with channel-adaptive bitrate selection
- **Hardware Acceleration:** Supports hardware decoding when available

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

# Build with cargo
cargo build --release

# Install locally
cargo install --path .
```

## Usage

```bash
# Encode a single file
drapto encode --input input.mkv --output output.mkv

# Get information about a media file
drapto info input.mkv

# Validate an encoded file
drapto validate --input encoded.mp4 --reference original.mkv
```

Drapto's encoding pipeline performs:
- **Segmentation:** The video is segmented using dynamic, scene-based detection
- **Memory-Aware Parallel Encoding:** Segments are encoded in parallel with adaptive retry strategies
- **Concatenation & Validation:** The encoded segments are concatenated and thoroughly validated

## Configuration

Drapto has a comprehensive, modular configuration system with multiple methods:

1. A TOML configuration file (`drapto.toml`)
2. Environment variables with the `DRAPTO_` prefix
3. Command-line arguments

Configuration is organized into logical sections (video, audio, validation, resources, etc.) with sensible defaults that can be overridden as needed.

### Example Configuration

```toml
# Basic input/output paths
input = "input.mkv"
output = "output.mp4"

[video]
# Target VMAF quality for SDR content (0-100)
target_vmaf = 93.0
# Target VMAF for HDR content
target_vmaf_hdr = 95.0
# Encoder preset (0-13, lower is slower but better quality)
preset = 6
# Encoder to use
encoder = "libsvtav1"
# Enable/disable scene-based segmentation
use_segmentation = true

[scene_detection]
# Content threshold for scene detection (0-100)
scene_threshold = 40.0
# Minimum segment length in seconds
min_segment_length = 5.0

[audio]
# Opus encoding compression level (0-10)
compression_level = 10
# Use variable bitrate
vbr = true
# Application type (voip, audio, lowdelay)
application = "audio"

[audio.bitrates]
# Bitrate for stereo audio (2 channels) in kbps
stereo = 128
# Bitrate for 5.1 surround (6 channels) in kbps
surround_5_1 = 256

[resources]
# Number of parallel encoding jobs (0 = auto-detect CPU cores)
parallel_jobs = 0
# Memory threshold as fraction of system RAM
memory_threshold = 0.7
# Memory limit per encoding job in MB
memory_per_job = 2048
```

For detailed configuration options, see the [Configuration Guide](docs/configuration.md).

## Development & Troubleshooting

- **Logging:** Detailed logs are saved in the configured log directory
- **Temporary Files:** Temporary directories are automatically cleaned up after encoding (unless keep_temp_files is enabled)
- **Hardware Acceleration:** Automatically detected if available and enabled
- **Common Issues:** Check dependency versions, ensure input videos are valid, and verify sufficient disk space for temporary files
