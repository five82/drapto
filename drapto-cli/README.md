# drapto-cli

Command-line interface for the Drapto video encoding system.

## Commands

### encode

Encode a video file with quality-targeted AV1 encoding.

```bash
drapto encode --input video.mkv --output video.mp4
```

Options:
- `--input <FILE>`: Input video file
- `--output <FILE>`: Output video file
- `--config <FILE>`: Use specific configuration file
- `--target-quality <QUALITY>`: Target VMAF quality (0-100)
- `--preset <PRESET>`: Encoder preset (0-13, lower is slower/better)
- `--no-segmentation`: Disable scene-based segmentation
- `--parallel-jobs <NUM>`: Number of parallel encoding jobs
- `--verbose`: Enable verbose output

### info

Display information about a media file.

```bash
drapto info video.mkv
```

Options:
- `--json`: Output in JSON format

### validate

Validate an encoded file against its original.

```bash
drapto validate --input encoded.mp4 --reference original.mkv
```

Options:
- `--input <FILE>`: Encoded video file to validate
- `--reference <FILE>`: Original video file for comparison
- `--json`: Output validation report in JSON format

## Configuration

Drapto can be configured through multiple methods, in order of precedence:

1. Command-line arguments
2. Environment variables
3. Configuration file
4. Default values

### Configuration File

Create a `drapto.toml` file in your working directory or specify with `--config`:

```toml
# Basic settings
input = "input.mkv"
output = "output.mp4"

[video]
target_quality = 93.0
preset = 6
use_segmentation = true

[scene_detection]
scene_threshold = 40.0
min_segment_length = 5.0

[resources]
parallel_jobs = 4
```

### Environment Variables

All configuration options can be set with environment variables:

```bash
export DRAPTO_TARGET_VMAF=90.0
export DRAPTO_PRESET=8
export DRAPTO_SCENE_THRESHOLD=35.0
```

For a complete list of configuration options, see the [Configuration Guide](../docs/configuration.md).