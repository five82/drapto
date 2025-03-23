# Drapto

ab-av1 video encoding wrapper with scene-based segmentation, parallel processing, and Dolby Vision support.

This is a vibe coding experiment to see how far LLM tools can take this

## Features

- **AV1 Encoding with SVT-AV1:** High-quality encoding using libsvtav1 with configurable presets
- **Intelligent Scene-Based Segmentation:** Automatically segments videos using adaptive scene detection
- **Quality-Targeted Encoding:** Uses ab-av1 to achieve consistent quality with VMAF metrics
- **Parallel Encoding Pipeline:** Encodes segments concurrently with memory-aware scheduling
- **Enhanced Output Validation:** Performs comprehensive validation of video/audio streams, container integrity, and quality targets
- **Dolby Vision Support:** Automatic detection and handling of Dolby Vision content
- **Automatic Black Bar Detection and Cropping:** Detects black bars and applies appropriate crop filters
- **Quality Audio Encoding:** Intelligent audio encoding for optimal quality
- **Hardware Acceleration:** Supports hardware decoding when available

## Requirements

- Rust 1.76+
- FFmpeg 7.0+ with support for libsvtav1, libvmaf, and libopus
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

Drapto automatically detects Dolby Vision and routes such content through a dedicated encoding pipeline. For standard content, the pipeline performs:
- **Segmentation:** The video is segmented using dynamic, scene-based detection
- **Memory-Aware Parallel Encoding:** Segments are encoded in parallel with adaptive retry strategies
- **Concatenation & Validation:** The encoded segments are concatenated and thoroughly validated

## Configuration

Drapto supports a flexible configuration system with multiple methods:

1. A TOML configuration file (`drapto.toml`)
2. Environment variables
3. Command-line arguments

### Example Configuration

```toml
# Basic input/output paths
input = "input.mkv"
output = "output.mp4"

[video]
# Target VMAF quality (0-100)
target_quality = 93.0
# Encoder preset (0-13, lower is slower but better quality)
preset = 6
# Enable/disable scene-based segmentation
use_segmentation = true

[scene_detection]
# Content threshold for scene detection (0-100)
scene_threshold = 40.0
# Minimum segment length in seconds
min_segment_length = 5.0

[resources]
# Number of parallel encoding jobs (0 = auto-detect CPU cores)
parallel_jobs = 0
# Memory threshold as fraction of system RAM
memory_threshold = 0.7
```

For detailed configuration options, see the [Configuration Guide](docs/configuration.md).

## Development & Troubleshooting

- **Logging:** Detailed logs are saved in the configured log directory
- **Temporary Files:** Temporary directories are automatically cleaned up after encoding (unless keep_temp_files is enabled)
- **Hardware Acceleration:** Automatically detected if available and enabled
- **Common Issues:** Check dependency versions, ensure input videos are valid, and verify sufficient disk space for temporary files
