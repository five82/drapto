# drapto Functionality Documentation

This document provides a detailed overview of how drapto processes and encodes videos, including its detection mechanisms, encoding paths, and configuration options.

drapto is designed to work specifically with MKV video files sourced from DVD, Blu-ray, and 4K UHD Blu-ray sources. The output is strictly standardized:
- Container format is always MKV
- Video is always encoded using SVT-AV1
- Audio is always encoded using Opus

## Table of Contents
1. [Input Video Processing Flow](#input-video-processing-flow)
2. [Dolby Vision Detection](#dolby-vision-detection)
3. [Encoding Paths](#encoding-paths)
4. [Parallel Processing](#parallel-processing)
5. [Audio Processing](#audio-processing)
6. [Crop Detection](#crop-detection)
7. [Codec Usage](#codec-usage)
8. [Quality Control](#quality-control)
9. [Default Settings](#default-settings)
10. [User Configuration](#user-configuration)
11. [Hardware Acceleration](#hardware-acceleration)
12. [Validation Process](#validation-process)

## Input Video Processing Flow

When a video is input to drapto, it undergoes the following analysis steps:

1. **Initial Analysis**
   - Checks for Dolby Vision content using `mediainfo`
   - Analyzes video resolution to determine quality settings
   - Detects video and audio codecs
   - Performs crop detection if enabled

2. **Path Determination**
   - Selects between standard or Dolby Vision encoding path
   - Determines if chunked encoding should be used
   - Sets up appropriate encoding parameters based on content type

3. **Stream Analysis**
   - Identifies number and type of audio streams
   - Determines video color space and HDR characteristics
   - Analyzes frame rate and duration

## Dolby Vision Detection

Dolby Vision detection is performed using the following process:

1. **Detection Method**
   - Uses `mediainfo` to check for Dolby Vision metadata
   - Sets internal flag `IS_DOLBY_VISION=true` when detected

2. **Special Handling**
   - Activates Dolby Vision-specific encoding path
   - Uses specialized SVT-AV1 parameters for DV content
   - Maintains HDR metadata throughout the encoding process

## Encoding Paths

drapto supports two main encoding paths:

### Standard Path
- Uses direct FFmpeg encoding with SVT-AV1
- Quality control through CRF:
  - SD (≤720p): CRF 25
  - HD (≤1080p): CRF 25
  - UHD (>1080p): CRF 29
- Uses preset 6 by default
- Direct encoding without segmentation

### Chunked Encoding Path
Activated when `ENABLE_CHUNKED_ENCODING=true`:
- Uses ab-av1 for encoding
- Segments video into manageable chunks
- VMAF-based quality targeting (default target: 93)
- Three-tier encoding strategy:
  1. **Default Strategy**
     - 3 samples of 1 second each
     - Default VMAF target
     - Keyframe interval: 10s
  2. **More Samples Strategy**
     - 6 samples of 2 seconds each
     - Same VMAF target
     - Used if default strategy fails
  3. **Lower VMAF Strategy**
     - 6 samples of 2 seconds each
     - Reduces target VMAF by 2 points
     - Last resort option
- VMAF settings:
  - Subsample rate: 8
  - Pool method: harmonic mean
  - Default sample count: 3
  - Default sample duration: 1s

## Parallel Processing

The chunked encoding path utilizes GNU Parallel for efficient parallel processing:

1. **Segmentation**
   - Input video is split into fixed-duration segments (default: 15 seconds)
   - Each segment maintains keyframe alignment
   - Segments are stored in a temporary directory
   - FFmpeg segmentation process:
     ```
     ffmpeg -i input.mkv \
       -c:v copy \
       -an \
       -f segment \
       -segment_time 15 \
       -reset_timestamps 1 \
       segments/%04d.mkv
     ```
   - Uses stream copy (`-c:v copy`) to avoid re-encoding during split
   - Segments are numbered sequentially (0001.mkv, 0002.mkv, etc.)
   - Audio is excluded during segmentation (`-an`) and processed separately

2. **Parallel Encoding**
   - GNU Parallel distributes encoding jobs across CPU cores
   - Each segment is encoded independently
   - Job allocation adapts to available system resources
   - Progress tracking for each segment

3. **Encoding Process**
   ```   Input Video → Segments → Parallel Encoding → Concatenation
   [full video] → [seg1, seg2, ...] → [parallel encode] → [final video]
   ```
4. **Failure Handling**
   - Failed segments are automatically retried
   - Three-tier strategy applied independently to each segment
   - Failed segments don't affect other parallel jobs

5. **Resource Management**
   - Automatic CPU core allocation
   - Memory usage controlled per segment
   - Disk I/O balanced across parallel jobs

6. **Validation**
   - Each segment validated after encoding
   - VMAF scores checked per segment
   - Seamless transitions verified between segments

7. **Concatenation**
   - After all segments are encoded, they are concatenated using FFmpeg
   - Process:
     1. Generate concat file listing all segments in order:
        ```
        file 'encoded_segments/0001.mkv'
        file 'encoded_segments/0002.mkv'
        ...
        ```
     2. FFmpeg concatenation command:
        ```
        ffmpeg -f concat \
          -safe 0 \
          -i concat.txt \
          -c copy \
          output.mkv
        ```
   - Uses direct stream copy for lossless joining
   - Maintains frame accuracy at segment boundaries
   - Verifies segment order using numerical prefixes
   - Validates segment integrity before concatenation

## Audio Processing

Audio processing is handled with the following approach:

1. **Channel Detection and Bitrate Assignment**
   | Channels | Layout  | Bitrate |
   |----------|---------|---------|
   | 1        | Mono    | 64k     |
   | 2        | Stereo  | 128k    |
   | 6        | 5.1     | 256k    |
   | 8        | 7.1     | 384k    |
   | Other    | Custom  | 48k/ch  |

2. **Encoding Settings**
   - Codec: libopus
   - Mode: VBR (Variable Bit Rate)
   - Compression Level: 10
   - Frame Duration: 20ms
   - Channel Layout Filter: Standardized to 7.1/5.1/stereo/mono

3. **Multi-track Handling**
   - Processes each audio track independently
   - Maintains original track count
   - Preserves track languages and metadata

## Crop Detection

Crop detection is sophisticated and content-aware:

1. **Detection Parameters**
   - Standard Content: Base threshold of 24
   - HDR Content: Dynamic threshold 128-256
   - Black level analysis for HDR content

2. **HDR Detection**
   Identifies HDR through:
   - Color transfer characteristics
   - Color primaries
   - Color space information

3. **Threshold Adjustment**
   - Analyzes black levels in HDR content
   - Adjusts threshold dynamically (1.5x measured black level)
   - Maintains bounds between 16 and 256

4. **Safety Measures**
   - Maintains original aspect ratio
   - Can be disabled with `DISABLE_CROP=true`
   - Validates crop values before applying

## Codec Usage

drapto employs a strict set of codecs:

1. **Video Codec**
   - SVT-AV1 exclusively
   - No support for other encoders (x264, x265, etc.)
   - Supports hardware acceleration for decoding
   - Maintains 10-bit depth with yuv420p10le

2. **Audio Codec**
   - libopus exclusively
   - No support for other codecs (AAC, MP3, etc.)
   - VBR mode with high compression
   - Standardized channel layouts

3. **Container Format**
   - MKV exclusively for both input and output
   - Input sources: DVD, Blu-ray, and 4K UHD Blu-ray rips
   - Preserves chapter information
   - Maintains track metadata

4. **Processing Tools**
   - FFmpeg: General processing and muxing
   - ab-av1: Chunked encoding path
   - mediainfo: Content analysis

## Quality Control

Quality control varies by encoding path:

### Standard Path (CRF-based)
- Resolution-dependent CRF:
  - SD (≤720p): 25
  - HD (≤1080p): 25
  - UHD (>1080p): 29

### Chunked Path (VMAF-based)
- Target VMAF score
- Multiple encoding attempts with different strategies
- Quality validation per segment

## Default Settings

Key default settings include:

1. **Video Encoding**
   ```   PRESET=6
   SVT_PARAMS="tune=0:film-grain=0:film-grain-denoise=0"
   PIX_FMT="yuv420p10le"
   ```2. **CRF Values**
   ```   CRF_SD=25
   CRF_HD=25
   CRF_UHD=29
   ```

3. **Audio Encoding**
   - Codec: libopus
   - Compression Level: 10
   - Frame Duration: 20ms

## User Configuration

Users can customize the following settings:

1. **Quality Settings**
   - `PRESET`
   - `CRF_SD`, `CRF_HD`, `CRF_UHD`

2. **Processing Options**
   - `DISABLE_CROP`
   - `ENABLE_CHUNKED_ENCODING`

3. **Directory Settings**
   - Input directory
   - Output directory
   - Log directory

4. **Hardware Acceleration**
   - Automatically detected and configured
   - Can be manually disabled

5. **Audio Settings**
   - Channel layouts
   - Bitrates per channel configuration

## Hardware Acceleration

1. **Detection**
   - Automatically checks for supported hardware acceleration
   - Currently supports:
     - macOS: VideoToolbox
   - Falls back to software encoding if hardware acceleration fails

2. **Implementation**
   - Applied during video decoding phase
   - Automatic fallback to software decoding on failure
   - Can be manually disabled
   - Hardware acceleration status logged during processing

## Validation Process

1. **Output Validation**
   - Verifies file existence and size
   - Checks for AV1 video stream presence
   - Validates all audio streams are Opus
   - Compares input/output duration (allows 1 second difference)

2. **Segment Validation**
   - Minimum segment size check (1KB)
   - Verifies video stream in each segment
   - Validates segment ordering
   - Checks segment integrity before concatenation

3. **Credits Detection**
   - Smart credits handling for crop detection:
     - Movies (>1 hour): Skips last 3 minutes
     - Long content (>20 min): Skips last 1 minute
     - Medium content (>5 min): Skips last 30 seconds
   - Prevents false crop detection during credits

4. **HDR Content**
   - Detects various HDR formats:
     - SMPTE 2084 (PQ)
     - ARIB STD-B67 (HLG)
     - BT.2020
   - Adjusts crop detection thresholds:
     - Standard content: 24
     - HDR content: Dynamic 128-256 based on black levels
   - Black level analysis for optimal crop detection