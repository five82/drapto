# Drapto Configuration Guide

Drapto provides a flexible configuration system that allows you to customize its behavior through multiple methods. This guide explains how to configure drapto to meet your specific needs.

## Configuration Methods

Drapto supports three ways to configure its behavior, in order of precedence (highest to lowest):

1. **Command-line arguments**: Directly passed when running drapto
2. **Environment variables**: Set in your shell environment
3. **Configuration file**: A TOML file containing your settings
4. **Default values**: Built-in defaults used if no other value is specified

When multiple configuration methods are used, higher precedence settings override lower ones.

## Configuration File

The simplest way to manage multiple settings is through a configuration file in TOML format. Create a file named `drapto.toml` in your working directory or specify a path with the `--config` argument.

Here's an example configuration file with common settings:

```toml
# Basic input/output paths
input = "input.mkv"
output = "output.mp4"

# Scene detection settings
[scene_detection]
# Content threshold for scene detection (0-100)
scene_threshold = 40.0
# Threshold for HDR content (lower to create more scenes)
hdr_scene_threshold = 30.0
# Minimum segment length in seconds
min_segment_length = 5.0
# Maximum segment length before forcing split
max_segment_length = 15.0

# Video encoding settings
[video]
# Target VMAF quality (0-100)
target_quality = 93.0
# Target quality for HDR content
target_quality_hdr = 95.0
# Encoder preset (0-13, lower is slower but better quality)
preset = 6
# SVT-AV1 encoder parameters
svt_params = "tune=0:film-grain=0:film-grain-denoise=0"
# Pixel format
pix_fmt = "yuv420p10le"
# Enable/disable automatic crop detection
disable_crop = false
# Enable/disable scene-based segmentation
use_segmentation = true
# VMAF quality sampling
vmaf_sample_count = 3
vmaf_sample_length = 1.0
# Hardware acceleration for decoding
hardware_acceleration = true

# Audio encoding settings
[audio]
# Audio codec
codec = "aac"
# Audio bitrate in kbps
bitrate = 128
# Enable audio normalization
normalize = true
# Target loudness in LUFS
target_loudness = -23.0

# Resource management
[resources]
# Number of parallel encoding jobs (0 = auto-detect based on CPU cores)
parallel_jobs = 0
# Memory threshold as fraction of system RAM
memory_threshold = 0.7
# Maximum parallel memory-intensive operations
max_memory_tokens = 8
# Delay between task submissions in seconds
task_stagger_delay = 0.2
# Memory limit per encoding job in MB
memory_per_job = 2048

# File paths and directories
[directories]
# Base working directory
temp_dir = "/tmp/drapto"
# Keep intermediate files (useful for debugging)
keep_temp_files = false

# Logging settings
[logging]
# Enable verbose output
verbose = false
# Log level (DEBUG, INFO, WARNING, ERROR)
log_level = "INFO"
# Log directory
log_dir = "~/drapto_logs"
```

## Environment Variables

All configuration options can be set through environment variables with the `DRAPTO_` prefix. For nested configuration options, use uppercase with underscores.

Examples:

```bash
# Set scene detection threshold
export DRAPTO_SCENE_THRESHOLD=35.0

# Set target VMAF quality
export DRAPTO_TARGET_VMAF=90.0

# Set parallel jobs count
export DRAPTO_PARALLEL_JOBS=4

# Set working directory
export DRAPTO_WORKDIR=/path/to/temp/directory

# Disable segmentation
export DRAPTO_USE_SEGMENTATION=false
```

## Command-line Arguments

Most common settings can be configured directly via command-line arguments (overrides both config file and environment variables):

```bash
drapto encode \
  --input input.mkv \
  --output output.mp4 \
  --scene-threshold 35.0 \
  --target-quality 90.0 \
  --preset 6 \
  --parallel-jobs 4 \
  --disable-crop \
  --no-segmentation
```

Run `drapto encode --help` to see all available command-line options.

## Common Configuration Scenarios

### Faster Encoding (Lower Quality)

```toml
[video]
preset = 8
target_quality = 85.0

[resources]
parallel_jobs = 8
```

### Highest Quality Encoding

```toml
[video]
preset = 4
target_quality = 95.0
target_quality_hdr = 97.0
```

### Low-Memory System

```toml
[resources]
parallel_jobs = 2
memory_threshold = 0.5
max_memory_tokens = 4
memory_per_job = 1024
```

### HDR Content

```toml
[scene_detection]
hdr_scene_threshold = 25.0

[video]
target_quality_hdr = 95.0
pix_fmt = "yuv420p10le"
```

## Configuration Reference

Below is a complete reference of all available configuration options, their descriptions, default values, and corresponding environment variables.

| Config Option | Description | Default | Environment Variable |
|---------------|-------------|---------|---------------------|
| `input` | Input file path | | `DRAPTO_INPUT` |
| `output` | Output file path | | `DRAPTO_OUTPUT` |
| `scene_detection.scene_threshold` | Scene detection threshold (0-100) | 40.0 | `DRAPTO_SCENE_THRESHOLD` |
| `scene_detection.hdr_scene_threshold` | HDR content scene threshold | 30.0 | `DRAPTO_HDR_SCENE_THRESHOLD` |
| `scene_detection.min_segment_length` | Minimum segment length (seconds) | 5.0 | `DRAPTO_MIN_SEGMENT_LENGTH` |
| `scene_detection.max_segment_length` | Maximum segment length (seconds) | 15.0 | `DRAPTO_MAX_SEGMENT_LENGTH` |
| `video.target_quality` | Target VMAF quality (0-100) | 93.0 | `DRAPTO_TARGET_VMAF` |
| `video.target_quality_hdr` | Target VMAF for HDR (0-100) | 95.0 | `DRAPTO_TARGET_VMAF_HDR` |
| `video.preset` | Encoder preset (0-13) | 6 | `DRAPTO_PRESET` |
| `video.svt_params` | SVT-AV1 parameters | tune=0:film-grain=0:film-grain-denoise=0 | `DRAPTO_SVT_PARAMS` |
| `video.pix_fmt` | Pixel format | yuv420p10le | `DRAPTO_PIX_FMT` |
| `video.disable_crop` | Disable crop detection | false | `DRAPTO_DISABLE_CROP` |
| `video.use_segmentation` | Enable segmentation | true | `DRAPTO_USE_SEGMENTATION` |
| `video.hardware_acceleration` | Use hardware acceleration | true | `DRAPTO_HARDWARE_ACCELERATION` |
| `audio.codec` | Audio codec | aac | `DRAPTO_AUDIO_CODEC` |
| `audio.bitrate` | Audio bitrate in kbps | 128 | `DRAPTO_AUDIO_BITRATE` |
| `audio.normalize` | Enable audio normalization | true | `DRAPTO_AUDIO_NORMALIZE` |
| `audio.target_loudness` | Target loudness in LUFS | -23.0 | `DRAPTO_TARGET_LOUDNESS` |
| `resources.parallel_jobs` | Parallel encoding jobs | auto | `DRAPTO_PARALLEL_JOBS` |
| `resources.memory_threshold` | Memory threshold (0.0-1.0) | 0.7 | `DRAPTO_MEMORY_THRESHOLD` |
| `resources.max_memory_tokens` | Max memory tokens | 8 | `DRAPTO_MAX_MEMORY_TOKENS` |
| `resources.task_stagger_delay` | Task stagger delay (seconds) | 0.2 | `DRAPTO_TASK_STAGGER_DELAY` |
| `resources.memory_per_job` | Memory per job (MB) | 2048 | `DRAPTO_MEMORY_PER_JOB` |
| `directories.temp_dir` | Temporary directory | /tmp/drapto | `DRAPTO_WORKDIR` |
| `directories.keep_temp_files` | Keep temporary files | false | `DRAPTO_KEEP_TEMP_FILES` |
| `logging.verbose` | Verbose logging | false | `DRAPTO_VERBOSE` |
| `logging.log_level` | Log level | INFO | `DRAPTO_LOG_LEVEL` |
| `logging.log_dir` | Log directory | ~/drapto_logs | `DRAPTO_LOG_DIR` |