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

See the [example configuration file](../drapto.toml.example) for a comprehensive example with all available settings and their default values.

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

# Set comma-separated list of standard audio sample rates
export DRAPTO_STANDARD_SAMPLE_RATES=44100,48000,96000
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
target_vmaf = 85.0

[resources]
parallel_jobs = 8
```

### Highest Quality Encoding

```toml
[video]
preset = 4
target_vmaf = 95.0
target_vmaf_hdr = 97.0
svt_params = "tune=0:enable-qm=1:enable-overlays=1:film-grain=0"
```

### Low-Memory System

```toml
[resources]
parallel_jobs = 2
memory_threshold = 0.5
max_memory_tokens = 4
memory_per_job = 1024
memory_reserve_percent = 0.3
```

### HDR Content

```toml
[scene_detection]
hdr_scene_threshold = 25.0

[video]
target_vmaf_hdr = 95.0
pixel_format = "yuv420p10le"
```

### Opus Audio Configuration

```toml
[audio]
compression_level = 10
frame_duration = 20
vbr = true
application = "audio"

[audio.bitrates]
mono = 64
stereo = 128
surround_5_1 = 256
surround_7_1 = 384
per_channel = 48
```

## Configuration Reference

Below is a complete reference of all available configuration options, their descriptions, default values, and corresponding environment variables.

### Core Options

| Config Option | Description | Default | Environment Variable |
|---------------|-------------|---------|---------------------|
| `input` | Input file path | | `DRAPTO_INPUT` |
| `output` | Output file path | | `DRAPTO_OUTPUT` |

### Directory Configuration

| Config Option | Description | Default | Environment Variable |
|---------------|-------------|---------|---------------------|
| `directories.temp_dir` | Base temporary directory | /tmp/drapto | `DRAPTO_WORKDIR` |
| `directories.working_dir` | Directory for temporary processing | /tmp/drapto/working | `DRAPTO_WORKING_DIR` |
| `directories.segments_dir` | Directory for segmented files | /tmp/drapto/segments | `DRAPTO_SEGMENTS_DIR` |
| `directories.encoded_segments_dir` | Directory for encoded segments | /tmp/drapto/encoded_segments | `DRAPTO_ENCODED_SEGMENTS_DIR` |
| `directories.keep_temp_files` | Keep temporary files after encoding | false | `DRAPTO_KEEP_TEMP_FILES` |

### Video Encoding Configuration

| Config Option | Description | Default | Environment Variable |
|---------------|-------------|---------|---------------------|
| `video.preset` | Encoder preset (0-13, lower = slower/better quality) | 6 | `DRAPTO_PRESET` |
| `video.svt_params` | SVT-AV1 encoder parameters | tune=0:enable-qm=1:enable-overlays=1:film-grain=0:film-grain-denoise=0 | `DRAPTO_SVT_PARAMS` |
| `video.encoder` | Encoder to use | libsvtav1 | `DRAPTO_ENCODER` |
| `video.keyframe_interval` | Keyframe interval | 10s | `DRAPTO_KEYFRAME_INTERVAL` |
| `video.pixel_format` | Pixel format | yuv420p10le | `DRAPTO_PIXEL_FORMAT` |
| `video.disable_crop` | Disable automatic crop detection | false | `DRAPTO_DISABLE_CROP` |
| `video.use_segmentation` | Use scene-based segmentation and parallel encoding | true | `DRAPTO_USE_SEGMENTATION` |
| `video.target_vmaf` | Target VMAF score (0-100) for SDR content | 93.0 | `DRAPTO_TARGET_VMAF` |
| `video.target_vmaf_hdr` | Target VMAF score (0-100) for HDR content | 95.0 | `DRAPTO_TARGET_VMAF_HDR` |
| `video.vmaf_sample_count` | Number of samples to use for VMAF analysis | 3 | `DRAPTO_VMAF_SAMPLE_COUNT` |
| `video.vmaf_sample_duration` | Duration of each VMAF sample in seconds | 1.0 | `DRAPTO_VMAF_SAMPLE_DURATION` |
| `video.vmaf_options` | VMAF analysis options | n_subsample=8:pool=perc5_min | `DRAPTO_VMAF_OPTIONS` |
| `video.hardware_acceleration` | Enable hardware acceleration for decoding | true | `DRAPTO_HARDWARE_ACCELERATION` |
| `video.hw_accel_option` | Hardware acceleration options for FFmpeg | | `DRAPTO_HW_ACCEL_OPTION` |
| `video.max_retries` | Maximum number of retries for failed encoding | 2 | `DRAPTO_MAX_RETRIES` |
| `video.force_quality_score` | Quality score for final retry attempt | 95.0 | `DRAPTO_FORCE_QUALITY_SCORE` |

### Audio Encoding Configuration

| Config Option | Description | Default | Environment Variable |
|---------------|-------------|---------|---------------------|
| `audio.compression_level` | Opus encoding compression level (0-10) | 10 | `DRAPTO_AUDIO_COMPRESSION_LEVEL` |
| `audio.frame_duration` | Opus frame duration in milliseconds | 20 | `DRAPTO_AUDIO_FRAME_DURATION` |
| `audio.vbr` | Use variable bitrate | true | `DRAPTO_AUDIO_VBR` |
| `audio.application` | Application type (voip, audio, lowdelay) | audio | `DRAPTO_AUDIO_APPLICATION` |
| `audio.bitrates.mono` | Bitrate for mono audio (1 channel) in kbps | 64 | Set in code |
| `audio.bitrates.stereo` | Bitrate for stereo audio (2 channels) in kbps | 128 | Set in code |
| `audio.bitrates.surround_5_1` | Bitrate for 5.1 surround (6 channels) in kbps | 256 | Set in code |
| `audio.bitrates.surround_7_1` | Bitrate for 7.1 surround (8 channels) in kbps | 384 | Set in code |
| `audio.bitrates.per_channel` | Bitrate per channel for other configurations in kbps | 48 | Set in code |

### Scene Detection Configuration

| Config Option | Description | Default | Environment Variable |
|---------------|-------------|---------|---------------------|
| `scene_detection.scene_threshold` | Scene detection threshold for SDR content (0-100) | 40.0 | `DRAPTO_SCENE_THRESHOLD` |
| `scene_detection.hdr_scene_threshold` | Scene detection threshold for HDR content (0-100) | 30.0 | `DRAPTO_HDR_SCENE_THRESHOLD` |
| `scene_detection.scene_tolerance` | Scene validation tolerance in seconds | 0.5 | `DRAPTO_SCENE_TOLERANCE` |
| `scene_detection.min_segment_length` | Minimum segment length in seconds | 5.0 | `DRAPTO_MIN_SEGMENT_LENGTH` |
| `scene_detection.max_segment_length` | Maximum segment length in seconds | 15.0 | `DRAPTO_MAX_SEGMENT_LENGTH` |

### Crop Detection Configuration

| Config Option | Description | Default | Environment Variable |
|---------------|-------------|---------|---------------------|
| `crop_detection.sdr_threshold` | Base crop detection threshold for SDR content | 16 | `DRAPTO_CROP_SDR_THRESHOLD` |
| `crop_detection.hdr_threshold` | Base crop detection threshold for HDR content | 128 | `DRAPTO_CROP_HDR_THRESHOLD` |
| `crop_detection.hdr_black_level_multiplier` | Multiplier applied to analyzed black levels in HDR content | 1.5 | `DRAPTO_HDR_BLACK_MULTIPLIER` |
| `crop_detection.min_threshold` | Minimum allowed crop threshold | 16 | `DRAPTO_CROP_MIN_THRESHOLD` |
| `crop_detection.max_threshold` | Maximum allowed crop threshold | 256 | `DRAPTO_CROP_MAX_THRESHOLD` |
| `crop_detection.min_black_bar_percent` | Minimum percentage of height that black bars must occupy to be cropped | 1 | `DRAPTO_MIN_BLACK_BAR_PERCENT` |
| `crop_detection.min_height` | Minimum height in pixels for a cropped frame | 100 | `DRAPTO_CROP_MIN_HEIGHT` |
| `crop_detection.sampling_interval` | Sampling interval in seconds between analyzed frames | 5.0 | `DRAPTO_CROP_SAMPLING_INTERVAL` |
| `crop_detection.min_sample_count` | Minimum number of samples to analyze regardless of duration | 20 | `DRAPTO_CROP_MIN_SAMPLES` |
| `crop_detection.frame_selection` | Frame selection pattern for ffmpeg select filter | not(mod(n,30)) | `DRAPTO_CROP_FRAME_SELECTION` |
| `crop_detection.credits_skip_movie` | Skip duration for movies (content > 1 hour) | 180.0 | `DRAPTO_CREDITS_SKIP_MOVIE` |
| `crop_detection.credits_skip_episode` | Skip duration for TV episodes (content > 20 minutes) | 60.0 | `DRAPTO_CREDITS_SKIP_EPISODE` |
| `crop_detection.credits_skip_short` | Skip duration for short content (content > 5 minutes) | 30.0 | `DRAPTO_CREDITS_SKIP_SHORT` |

### Validation Configuration

| Config Option | Description | Default | Environment Variable |
|---------------|-------------|---------|---------------------|
| `validation.sync_threshold_ms` | Maximum allowed audio/video sync difference in milliseconds | 100 | `DRAPTO_SYNC_THRESHOLD_MS` |
| `validation.duration_tolerance` | Absolute tolerance for duration differences in seconds | 0.2 | `DRAPTO_DURATION_TOLERANCE` |
| `validation.duration_relative_tolerance` | Relative tolerance for duration differences as a fraction (0.0-1.0) | 0.05 | `DRAPTO_DURATION_RELATIVE_TOLERANCE` |
| `validation.audio.short_audio_threshold` | Threshold below which audio streams are considered too short (seconds) | 0.5 | `DRAPTO_SHORT_AUDIO_THRESHOLD` |
| `validation.audio.standard_sample_rates` | Standard acceptable audio sample rates | [8000, 16000, 22050, 24000, 32000, 44100, 48000, 96000] | `DRAPTO_STANDARD_SAMPLE_RATES` |
| `validation.audio.preferred_codecs` | List of preferred audio codecs | ["opus"] | Set in code |
| `validation.audio.acceptable_codecs` | List of acceptable audio codecs | ["aac", "vorbis"] | Set in code |
| `validation.video.min_video_bitrate` | Minimum acceptable video bitrate in kbps | 500 | `DRAPTO_MIN_VIDEO_BITRATE` |
| `validation.video.min_quality_score` | Minimum acceptable quality score (VMAF) for validation | 80.0 | `DRAPTO_MIN_QUALITY_SCORE` |
| `validation.video.min_width` | Minimum acceptable video width in pixels | 16 | `DRAPTO_MIN_VIDEO_WIDTH` |
| `validation.video.min_height` | Minimum acceptable video height in pixels | 16 | `DRAPTO_MIN_VIDEO_HEIGHT` |
| `validation.video.min_framerate` | Minimum acceptable video framerate | 10.0 | `DRAPTO_MIN_FRAMERATE` |
| `validation.video.max_framerate` | Maximum acceptable video framerate | 120.0 | `DRAPTO_MAX_FRAMERATE` |

### Resource Management Configuration

| Config Option | Description | Default | Environment Variable |
|---------------|-------------|---------|---------------------|
| `resources.parallel_jobs` | Number of parallel encoding jobs | CPU cores | `DRAPTO_PARALLEL_JOBS` |
| `resources.task_stagger_delay` | Task stagger delay in seconds | 0.2 | `DRAPTO_TASK_STAGGER_DELAY` |
| `resources.memory_threshold` | Memory threshold as a fraction of total system memory | 0.7 | `DRAPTO_MEMORY_THRESHOLD` |
| `resources.max_memory_tokens` | Maximum memory tokens for concurrent operations | 8 | `DRAPTO_MAX_MEMORY_TOKENS` |
| `resources.memory_per_job` | Memory limit per encoding job in MB (0 = auto) | 2048 | `DRAPTO_MEMORY_PER_JOB` |
| `resources.memory_reserve_percent` | Reserve percentage of system memory (0.0-1.0) | 0.2 | `DRAPTO_MEMORY_RESERVE_PERCENT` |
| `resources.memory_token_size` | Default memory token size in MB | 512 | `DRAPTO_MEMORY_TOKEN_SIZE` |
| `resources.memory_allocation_percent` | Memory allocation percentage of available memory (0.0-1.0) | 0.6 | `DRAPTO_MEMORY_ALLOCATION_PERCENT` |
| `resources.min_memory_tokens` | Minimum allowed memory tokens | 1 | `DRAPTO_MIN_MEMORY_TOKENS` |
| `resources.max_memory_tokens_limit` | Maximum allowed memory tokens | 16 | `DRAPTO_MAX_MEMORY_TOKENS_LIMIT` |

### Logging Configuration

| Config Option | Description | Default | Environment Variable |
|---------------|-------------|---------|---------------------|
| `logging.verbose` | Enable verbose logging | false | `DRAPTO_VERBOSE` |
| `logging.log_level` | Log level (DEBUG, INFO, WARNING, ERROR) | INFO | `DRAPTO_LOG_LEVEL` |
| `logging.log_dir` | Log directory | ~/drapto_logs | `DRAPTO_LOG_DIR` |