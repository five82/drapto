# Drapto

High-quality AV1 video encoding pipeline with intelligent chunked encoding and Dolby Vision support.

## Features

- AV1 encoding using SVT-AV1
- Intelligent chunked encoding for faster processing
- Dolby Vision content detection and handling
- Automatic black bar detection and cropping
- High-quality Opus audio encoding
- Hardware acceleration support (VideoToolbox on macOS)

## Requirements

- Python 3.8+
- FFmpeg with libsvtav1 and libopus
- mediainfo
- GNU Parallel (for chunked encoding)
- ab-av1 (for quality-targeted encoding)

## Installation

```bash
# Install using pipx (recommended)
pipx install .

# Or install in development mode
pipx install -e .
```

## Usage

### Command Line

```bash
# Encode a single file
drapto input.mkv output.mkv

# Encode all videos in a directory
drapto input_dir/ output_dir/
```

### Configuration

The encoder can be configured by modifying settings in `drapto/config.py`:

- `PRESET`: Encoding speed preset (0-13, default: 6)
- `CRF_*`: Quality settings for different resolutions
- `TARGET_VMAF`: Target quality for chunked encoding
- `ENABLE_CHUNKED_ENCODING`: Enable/disable parallel encoding
- `SEGMENT_LENGTH`: Length of chunks in seconds

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

## Development

```bash
# Install development dependencies
pip install -e ".[dev]"

# Run tests
pytest

# Build distribution
python -m build
```

## Troubleshooting

1. **Missing Dependencies**
   ```bash
   # Install FFmpeg with required codecs
   brew install ffmpeg

   # Install other dependencies
   brew install mediainfo parallel
   cargo install ab-av1
   ```

2. **Common Issues**
   - Check logs in `videos/logs/` for detailed error information
   - Ensure input files are valid video files
   - Verify sufficient disk space for temporary files

## License

MIT License
