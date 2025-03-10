# Drapto

AV1 video encoding ab-av1 wrapper with chunked encoding, parallel processing, and Dolby Vision support.

This is a vibe coding experiment to see how far LLM tools can take this. Pull requests are welcome.

## Features

- **AV1 Encoding with SVT-AV1:** High-quality encoding using libsvtav1 with configurable presets.
- **Intelligent Scene-Based Segmentation:** Automatically segments the input video using adaptive scene detection. Fixed segmentation modes have been removed—now only dynamic, scene-based segmentation is supported. (Configure parameters such as SCENE_THRESHOLD, HDR_SCENE_THRESHOLD, TARGET_MIN_SEGMENT_LENGTH, and MAX_SEGMENT_LENGTH in config.py.)
- **VMAF-based Quality Analysis & Adaptive Retry:** Measures quality via VMAF (parsed from ab‑av1 output) and adjusts encoding parameters on retries (e.g. increased sample count/duration, raised min_vmaf).
- **Standard Encoding Pipeline:** Encodes segments in parallel using a dynamic memory‐aware scheduler that performs a warm-up analysis to optimally balance resource usage with adaptive retries. (See parameters MEMORY_THRESHOLD, MAX_MEMORY_TOKENS, and TASK_STAGGER_DELAY in config.py.)
- **Enhanced Output Validation:** Performs comprehensive validation of video/audio streams, container integrity, crop dimensions, and VMAF-based quality metrics, outputting a detailed validation report.
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
- [GNU Parallel](https://www.gnu.org/software/parallel/) (legacy—parallel encoding is now managed internally with Python's concurrent.futures)
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

Drapto automatically detects Dolby Vision and routes such content through a dedicated encoding pipeline. For standard (non‑Dolby Vision) content, the pipeline performs:
- **Segmentation:** The video is segmented using dynamic, scene-based detection.
- **Dynamic, Memory-Aware Parallel Encoding:** Segments are encoded in parallel utilizing a warm-up phase to gauge resource usage. Adaptive retry strategies are applied on failures.
- **Concatenation & Enhanced Validation:** The encoded segments are concatenated and thoroughly validated (including checks on codec, duration, crop, and VMAF quality metrics), with a detailed report produced.

### Configuration

The encoder can be configured by modifying settings in `drapto/config.py`. Notable parameters include:

- `PRESET`: Encoding speed preset (0 to 13; default: 6).
- `TARGET_VMAF`: Target VMAF for quality-targeted standard encoding.
- `VMAF_SAMPLE_COUNT` and `VMAF_SAMPLE_LENGTH`: Parameters used for quality analysis of segments.
- `TARGET_SEGMENT_LENGTH`: Target segment duration (in seconds) used as a guideline by the scene detection algorithm.
- (Note: A fixed segmentation mode is no longer supported.)
- Additional parameters for cropping, Dolby Vision handling, and hardware acceleration.

These settings allow you to finely tune the balance between encoding speed and output quality.

- **Memory Management:** Configure `MEMORY_THRESHOLD`, `MAX_MEMORY_TOKENS`, and `TASK_STAGGER_DELAY` to control resource allocation during parallel encoding.
- **Scene Detection Tuning:** Adjust `SCENE_THRESHOLD`, `MIN_SCENE_INTERVAL`, `CLUSTER_WINDOW`, `TARGET_SEGMENT_LENGTH`, and `MAX_SEGMENT_LENGTH` to fine-tune segmentation performance.

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
