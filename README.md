# Drapto

High-quality AV1 video encoding pipeline with intelligent chunked encoding and Dolby Vision support.

## Features

- **AV1 Encoding with SVT-AV1:** High-quality encoding using libsvtav1 with configurable presets.
- **Intelligent Chunked & Variable Segmentation:** Automatically segments input video based on scene detection (using PySceneDetect) and/or fixed intervals. Segments are encoded in parallel with built‐in retry logic.
- **VMAF-based Quality Analysis & Adaptive Retry:** Measures quality via VMAF (parsed from ab‑av1 output) and adjusts encoding parameters on retries (e.g. increased sample count/duration, raised min_vmaf).
- **Dolby Vision Support:** Automatic detection of Dolby Vision content with a dedicated encoding pipeline.
- **Automatic Black Bar Detection and Cropping:** Detects black bars via ffprobe/ffmpeg and applies appropriate crop filters.
- **High-Quality Opus Audio Encoding:** Dynamically determines the correct bitrate and layout for multiple audio tracks.
- **Hardware Acceleration:** Supports hardware decoding (e.g., VideoToolbox on macOS) when available.
- **Comprehensive Output Validation:** Validates video and audio streams, container integrity, crop dimensions, and quality targets.

## Requirements

- Python 3.8+
- FFmpeg with support for libsvtav1, libvmaf, and libopus
- mediainfo
- [scenedetect](https://pypi.org/project/scenedetect/) (for scene detection)
- GNU Parallel (for chunked encoding)
- ab-av1 (for quality-targeted encoding; install via Cargo: `cargo install ab-av1`)

## Installation

```bash
# Install using pipx (recommended)
pipx install .

# Or install in development mode
pipx install -e .
```

## Usage

### Usage

```bash
# Encode a single file
drapto input.mkv output.mkv

# Encode all videos in a directory
drapto input_dir/ output_dir/
```

Drapto automatically detects Dolby Vision and uses a dedicated encoding pipeline for it. For standard content, the following steps occur:
- **Segmentation:** The video is segmented using variable segmentation based on scene detection (with options for fixed chunk lengths as a fallback).
- **Parallel Encoding with Adaptive Retry:** Segments are encoded in parallel. If encoding a segment fails, the pipeline retries (up to 3 attempts) with adjusted parameters (e.g. sample count/duration and quality thresholds).
- **Concatenation & Validation:** The encoded segments are concatenated and rigorously validated (checking codec, duration, crop, and VMAF/CRF quality targets).

### Configuration

The encoder can be configured by modifying settings in `drapto/config.py`. Notable parameters include:

- `PRESET`: Encoding speed preset (0 to 13; default: 6).
- `TARGET_VMAF`: Target VMAF for quality-targeted chunked encoding.
- `VMAF_SAMPLE_COUNT` and `VMAF_SAMPLE_LENGTH`: Parameters used for quality analysis of segments.
- `SEGMENT_LENGTH` & `TARGET_SEGMENT_LENGTH`: Define fixed and target segment durations.
- Additional parameters for cropping, Dolby Vision handling, and hardware acceleration.

These settings allow you to finely tune the balance between encoding speed and output quality.

### Features

1. **Intelligent Quality Control**
   - Resolution-based CRF selection
   - VMAF-targeted chunked encoding
   - Dolby Vision preservation

2. **Performance Optimization**
   - Parallel chunk processing
   - Hardware acceleration when available
   - Efficient audio encoding

3. **Quality Preservation**
   - Black bar detection and removal
   - High-quality Opus audio encoding
   - Stream copy for subtitles

## Development & Troubleshooting

- **Logging:** Drapto employs Rich for enhanced log formatting. Detailed logs are saved in the `LOG_DIR` (default: `$HOME/drapto_logs`), which can be used for diagnosing issues.
- **Temporary Files:** Automatic cleanup is performed on temporary directories (located in `/tmp/drapto` by default) after encoding.
- **Hardware Acceleration:** On macOS, VideoToolbox is used for decoding if available.
- **Common Issues:** Check dependency versions, ensure input videos are valid, and verify sufficient disk space for temporary files.
