# drapto-cli

Command-line interface for the Drapto video encoding tool, providing a user-friendly way to access the functionality of the `drapto-core` library.

## Features

- Video encoding with quality-based rate control
- Media validation
- System information commands

## Usage

```bash
# Encode a video file
drapto encode --input video.mp4 --output encoded.mp4 --quality 90

# Validate a media file
drapto validate --input video.mp4

# Check FFmpeg availability
drapto ffmpeg-info
```