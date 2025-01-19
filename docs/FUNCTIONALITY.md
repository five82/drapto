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
4. [Encoding Strategy System](#encoding-strategy-system)
5. [Parallel Processing](#parallel-processing)
6. [Muxing Process](#muxing-process)
7. [Audio Processing](#audio-processing)
8. [Crop Detection](#crop-detection)
9. [Codec Usage](#codec-usage)
10. [Validation and Quality Control](#validation-and-quality-control)
11. [Default Settings](#default-settings)
12. [User Configuration](#user-configuration)
13. [Hardware Acceleration](#hardware-acceleration)
14. [Validation Process](#validation-process)
15. [Error Recovery and Fallback Mechanisms](#error-recovery-and-fallback-mechanisms)
16. [Progress Tracking and Logging](#progress-tracking-and-logging)
17. [Directory Structure](#directory-structure)

## Input Video Processing Flow

When a video is input to drapto, it undergoes the following analysis steps:

1. **Directory Initialization**
   - Creates output directory if it doesn't exist
   - Verifies input directory exists and contains video files
   - Sets up temporary directory structure:
     ```
     TEMP_DIR/
     ├── logs/           # Processing logs
     ├── encode_data/    # Encoding state and metadata
     ├── segments/       # Video segments for chunked encoding
     ├── encoded/        # Encoded segments
     └── working/        # Temporary processing files
     ```
   - Initializes tracking files:
     - `encoded_files.txt`: List of processed files
     - `encoding_times.txt`: Processing duration data
     - `input_sizes.txt`: Original file sizes
     - `output_sizes.txt`: Encoded file sizes

2. **Initial Analysis**
   - Checks for Dolby Vision content using `mediainfo`
   - Analyzes video resolution to determine quality settings
   - Detects video and audio codecs
   - Performs crop detection if enabled
   - Validates input file integrity

3. **Path Determination**
   - Selects between standard or Dolby Vision encoding path
   - Determines if chunked encoding should be used
   - Sets up appropriate encoding parameters based on content type
   - Configures hardware acceleration for decoding

4. **Stream Analysis**
   - Identifies number and type of audio streams
   - Determines video color space and HDR characteristics
   - Analyzes frame rate and duration
   - Maps input streams to output configuration

5. **Processing Pipeline**
   - **Video Processing**
     1. Decoding phase (hardware-accelerated if available)
     2. Crop detection and application (if enabled)
     3. Video encoding with SVT-AV1:
        - **Standard Path**:
          * Direct FFmpeg encoding
          * CRF-based quality control
          * Single-pass encoding
          * Resolution-dependent CRF values
        - **Chunked Path** (when enabled):
          * Segments video into chunks
          * VMAF-based quality targeting with ab-av1
          * Multi-tier encoding strategy
          * Parallel processing of segments
     4. Quality validation
   - **Audio Processing**
     1. Track discovery and analysis
     2. Channel detection and bitrate assignment
     3. Per-track processing
     4. Failure recovery
     5. Quality control
     6. Track management
     7. Error handling
     8. Performance optimization
     9. Recovery procedures
     10. Logging and diagnostics
   - **Subtitle Processing**
     1. Extract subtitle tracks
     2. Preserve formatting and timing
     3. Copy to output without re-encoding

6. **Output Assembly**
   - Muxes encoded video track
   - Adds processed audio tracks
   - Includes subtitle tracks
   - Preserves chapters and metadata
   - Validates final container structure

7. **Cleanup Process**
   - Removes temporary segment files
   - Cleans up working directory
   - Preserves logs for debugging
   - Updates tracking files with results
   - Verifies output file integrity

8. **Error Handling**
   - Validates each pipeline stage
   - Retries failed operations when possible
   - Preserves partial progress on failure
   - Maintains detailed logs for debugging
   - Cleans up temporary files on failure

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
1. **Core Characteristics**
   - Direct FFmpeg encoding with SVT-AV1
   - Single-pass, CRF-based quality control
   - No segmentation or chunking
   - Continuous encoding process

2. **Quality Control**
   - Resolution-dependent CRF values:
     ```bash
     # Resolution breakpoints
     SD  (≤720p):  CRF 25
     HD  (≤1080p): CRF 25
     UHD (>1080p): CRF 29
     ```
   - SVT-AV1 preset 6 by default
   - Film grain synthesis disabled
   - 10-bit depth processing

3. **FFmpeg Command Structure**
   ```bash
   ffmpeg -hide_banner -loglevel warning \
     ${HWACCEL_OPTS} \                     # Hardware decode if available
     -i "${input_file}" \
     -map 0:v:0 \                          # Select first video stream
     -c:v libsvtav1 \                      # SVT-AV1 codec
     -preset ${PRESET} \                   # Default: 6
     -crf ${CRF} \                         # Based on resolution
     -pix_fmt yuv420p10le \               # 10-bit color
     -svtav1-params \
       "tune=0:film-grain=0:film-grain-denoise=0" \
     -y "${output_file}"
   ```

4. **Performance Characteristics**
   - Memory usage proportional to resolution
   - CPU utilization based on preset
   - Single continuous encoding process
   - Progress tracking via FFmpeg stats

5. **Use Cases**
   - Shorter content (< 30 minutes)
   - Lower resolution content
   - Quick encoding requirements
   - Limited system resources
   - Simple processing needs

6. **Quality Assurance**
   - Input stream validation
   - Output codec verification
   - Duration comparison
   - File size validation
   - Stream integrity check

7. **Error Handling**
   - Hardware decode fallback
   - Progress monitoring
   - Resource management
   - Detailed error logging
   - Process recovery support

### Chunked Path (VMAF-based)
1. **Core Characteristics**
   - Uses ab-av1 for encoding
   - VMAF-based quality targeting
   - Multi-tier encoding strategy
   - Parallel processing of segments

2. **Quality Control**
   - Target VMAF score: 93 (default)
   - Three-tier strategy:
     ```bash
     # Tier 1: Default Strategy
     --samples 3 --sample-duration 1s --vmaf-target 93
     
     # Tier 2: More Samples
     --samples 6 --sample-duration 2s --vmaf-target 93
     
     # Tier 3: Lower VMAF
     --samples 6 --sample-duration 2s --vmaf-target 91
     ```
   - VMAF settings:
     - Subsample rate: 8
     - Pool method: harmonic mean

3. **ab-av1 Command Structure**
   ```bash
   # First tier attempt
   ab-av1 \
     --input "${input_file}" \
     --output "${output_file}" \
     --encoder svtav1 \
     --preset ${PRESET} \                  # Default: 6
     --vmaf-target ${TARGET_VMAF} \        # Default: 93
     --samples ${VMAF_SAMPLE_COUNT} \      # Default: 3
     --sample-duration "${VMAF_SAMPLE_LENGTH}" \  # Default: 1s
     --keyint 10s \
     --pix-fmt yuv420p10le \
     --svtav1-params \
       "tune=0:film-grain=0:film-grain-denoise=0" \
     --vmaf "n_subsample=8:pool=harmonic_mean" \
     ${vfilter_args} \                     # Crop filter if enabled
     --quiet

   # Second tier attempt (on failure)
   ab-av1 \
     # ... same base options ...
     --samples 6 \
     --sample-duration "2s" \
     --quiet

   # Third tier attempt (on failure)
   ab-av1 \
     # ... same base options ...
     --samples 6 \
     --sample-duration "2s" \
     --vmaf-target $((TARGET_VMAF - 2)) \  # Reduce target by 2
     --quiet
   ```

4. **Performance Characteristics**
   - Memory usage controlled per segment
   - CPU utilization distributed across segments
   - Parallel processing via GNU Parallel
   - Progress tracking per segment

5. **Use Cases**
   - Longer content (> 30 minutes)
   - High resolution content
   - Quality-critical encodes
   - Systems with many CPU cores
   - Consistent quality requirements

6. **Quality Assurance**
   - VMAF score validation per segment
   - Segment integrity verification
   - Transition smoothness checks
   - Size and duration validation
   - Codec compliance verification

7. **Error Handling**
   - Per-segment retry logic
   - Multi-tier fallback strategy
   - Parallel job monitoring
   - Failed segment tracking
   - Resource usage monitoring

## Encoding Strategy System

drapto implements a modular strategy system for encoding, allowing different approaches based on content type:

1. **Strategy Architecture**
   ```
   encode_strategies/
   ├── strategy_base.sh      # Base strategy interface
   ├── chunked_encoding.sh   # Chunked encoding implementation
   ├── dolby_vision.sh       # Dolby Vision handling
   └── json_helper.py        # Strategy configuration
   ```

2. **Base Strategy Interface**
   Every strategy must implement these core functions:
   ```bash
   # Initialize encoding process
   initialize_encoding() {
     # Setup working directories
     # Initialize state tracking
     # Validate input parameters
   }

   # Prepare video for encoding
   prepare_video() {
     # Segment if needed
     # Configure encoding options
     # Setup quality targets
   }

   # Perform encoding
   encode_video() {
     # Execute encoding process
     # Track progress
     # Handle failures
   }

   # Finalize encoding
   finalize_encoding() {
     # Concatenate if needed
     # Cleanup temporary files
     # Validate output
   }

   # Validate strategy compatibility
   can_handle() {
     # Check if strategy can process input
     # Verify requirements are met
   }
   ```

3. **Strategy Selection**
   - Automatic selection based on:
     - Content type (Dolby Vision, HDR, SDR)
     - User preferences (chunked vs. standard)
     - Input file characteristics
     - System capabilities
   - Selection process:
     1. Check for Dolby Vision content
     2. Verify chunked encoding setting
     3. Load appropriate strategy
     4. Validate strategy compatibility

4. **Available Strategies**
   - **Standard Encoding**
     - Direct FFmpeg processing
     - CRF-based quality control
     - No segmentation
     - Suitable for most content

   - **Chunked Encoding**
     - Segments video for parallel processing
     - VMAF-based quality targeting
     - Multi-tier encoding approach
     - Better for longer content

   - **Dolby Vision**
     - Specialized parameters for DV content
     - HDR metadata preservation
     - Quality-focused encoding
     - Strict format compliance

5. **Extending the System**
   To create a new strategy:
   1. Create new script in `encode_strategies/`
   2. Source `strategy_base.sh`
   3. Implement required functions:
      ```bash
      #!/usr/bin/env bash
      source "${SCRIPT_DIR}/encode_strategies/strategy_base.sh"

      initialize_encoding() {
          # Your initialization code
      }

      prepare_video() {
          # Your preparation code
      }

      encode_video() {
          # Your encoding code
      }

      finalize_encoding() {
          # Your finalization code
      }

      can_handle() {
          # Your validation code
      }
      ```
   4. Add strategy loading in main script
   5. Update selection logic if needed

6. **Configuration System**
   - JSON-based configuration
   - Per-strategy settings
   - Override capabilities:
     - Quality parameters
     - Processing options
     - Resource allocation
     - Output preferences

7. **Error Handling**
   - Strategy-specific error recovery
   - Fallback mechanisms
   - Progress preservation
   - State recovery support
   - Detailed error logging

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

## Muxing Process

drapto employs a sophisticated track-by-track muxing system to ensure proper handling of all streams:

1. **Working Directory Structure**
   ```
   WORKING_DIR/
   ├── video.mkv          # Processed video track
   ├── audio-0.mkv        # First audio track
   ├── audio-1.mkv        # Second audio track (if present)
   ├── audio-N.mkv        # Additional audio tracks
   ├── concat.txt         # Segment list for chunked encoding
   └── temp/              # Temporary processing files
   ```

2. **Track Processing Order**
   1. **Video Track**
      - Processed first and stored as `video.mkv`
      - Uses hardware-accelerated decoding if available
      - Encoded with SVT-AV1 using selected quality settings
      - Validated before muxing

   2. **Audio Tracks**
      - Processed individually in sequence
      - Each track stored as `audio-N.mkv`
      - Channel layout analysis per track
      - Opus encoding with track-specific bitrates
      - Metadata preserved per track

   3. **Subtitle Tracks**
      - Stream copied without re-encoding
      - Format and timing preserved
      - Stored temporarily before final mux

3. **Muxing Command Construction**
   ```bash
   ffmpeg -hide_banner -loglevel warning \
     -i video.mkv \                    # Video input
     -i audio-0.mkv \                  # First audio
     -i audio-1.mkv \                  # Second audio
     -map 0:v:0 \                      # Map video
     -map 1:a:0 \                      # Map first audio
     -map 2:a:0 \                      # Map second audio
     -c copy \                         # Stream copy
     output.mkv
   ```

4. **Track Mapping**
   - Video track mapped from primary input
   - Audio tracks mapped sequentially
   - Stream indexes preserved
   - Track languages maintained
   - Track metadata retained

5. **Quality Control**
   - Pre-mux validation of all tracks
   - Stream presence verification
   - Codec validation
   - Duration checks
   - Size verification

6. **Error Handling**
   - Individual track failure recovery
   - Muxing process monitoring
   - Temporary file cleanup
   - Detailed error logging
   - Progress tracking

7. **Cleanup Process**
   - Temporary tracks removed after successful mux
   - Working directory cleaned
   - Logs preserved
   - Final output validated
   - Resource cleanup

## Audio Processing

drapto implements granular audio track processing with individual track handling and failure recovery:

1. **Track Discovery and Analysis**
   ```bash
   # Get number of audio tracks
   audio_stream_count=$("${FFPROBE}" -v error \
     -select_streams a \
     -show_entries stream=index \
     -of csv=p=0 "${input_file}" | wc -l)
   
   # For each track, analyze characteristics
   IFS=$'\n' read -r -d '' -a audio_channels < <("${FFPROBE}" -v error \
     -select_streams a \
     -show_entries stream=channels \
     -of csv=p=0 "${input_file}" && printf '\0')
   ```

2. **Channel Detection and Bitrate Assignment**
   ```bash
   # Standardize channel layouts and bitrates
   case $num_channels in
       1)  bitrate="64k"; layout="mono" ;;
       2)  bitrate="128k"; layout="stereo" ;;
       6)  bitrate="256k"; layout="5.1" ;;
       8)  bitrate="384k"; layout="7.1" ;;
       *)  print_warning "Unsupported channel count, defaulting to stereo"
           num_channels=2
           bitrate="128k"
           layout="stereo"
           ;;
   esac
   ```

3. **Per-Track Processing**
   ```bash
   # Apply consistent audio encoding settings
   audio_opts+=" -map 0:a:${stream_index}"
   audio_opts+=" -c:a:${stream_index} libopus"
   audio_opts+=" -b:a:${stream_index} ${bitrate}"
   audio_opts+=" -ac:${stream_index} ${num_channels}"
   
   # Apply consistent channel layout filter
   audio_opts+=" -filter:a:${stream_index} aformat=channel_layouts=7.1|5.1|stereo|mono"
   
   # Set consistent opus-specific options
   audio_opts+=" -application:a:${stream_index} audio"
   audio_opts+=" -frame_duration:a:${stream_index} 20"
   audio_opts+=" -vbr:a:${stream_index} on"
   audio_opts+=" -compression_level:a:${stream_index} 10"
   ```

4. **Track Metadata Preservation**
   - Language tags
   - Track titles
   - Delay information
   - Channel layout
   - Stream metadata

5. **Quality Control**
   - Track integrity verification
   - Channel count validation
   - Bitrate confirmation
   - Duration matching
   - Sample rate checking
   - Gap detection
   - Encoding parameter validation

6. **Error Handling**
   - Track-specific error codes
   - Granular error reporting
   - Track processing status tracking
   - Failure cause identification
   - Recovery action logging

7. **Performance Optimization**
   - Sequential track processing
   - Resource allocation per track
   - Progress monitoring
   - Track-specific timing
   - Memory usage control
   - I/O optimization

8. **Recovery Procedures**
   - Track processing resume
   - Partial progress preservation
   - Failed track isolation
   - Alternative processing paths
   - Quality compromise options

9. **Logging and Diagnostics**
   - Per-track log files
   - Processing statistics
   - Error condition details
   - Performance metrics
   - Quality measurements
   - Recovery attempts

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

## Validation and Quality Control

drapto implements comprehensive validation and quality control throughout the encoding process:

1. **Output File Validation**
   ```bash
   # Core validation checks
   - File existence and size verification
   - AV1 video stream presence
   - Opus audio stream count
   - Duration comparison (±1 second tolerance)
   - Stream integrity verification
   ```

2. **Size and Performance Metrics**
   - Input vs output size comparison
   - Compression ratio calculation
   - Processing time tracking
   - Resource utilization monitoring
   - Performance statistics:
     ```bash
     # Example metrics
     Input size:  15.7 GiB
     Output size: 4.2 GiB
     Reduction:   73.25%
     Encode time: 02h 15m 30s
     ```

3. **Segment Validation** (Chunked Mode)
   - Minimum segment size check (1KB)
   - Video stream presence in each segment
   - Segment ordering verification
   - Concatenation integrity checks
   - VMAF score validation per segment

4. **Track-Level Validation**
   - Video track codec verification
   - Audio track codec and count validation
   - Stream mapping verification
   - Metadata preservation checks
   - Channel layout validation

5. **Quality Metrics**
   - Resolution-specific CRF validation
   - VMAF score tracking (chunked mode)
   - Audio bitrate verification
   - Frame rate consistency
   - Color space validation

6. **Error Detection and Recovery**
   - Hardware acceleration fallback detection
   - Encoding failure identification
   - Resource exhaustion monitoring
   - Stream corruption detection
   - Process interruption handling

7. **Progress and Results Tracking**
   ```bash
   # Tracking data structure
   TEMP_DIR/encode_data/
   ├── encoded_files.txt     # Successfully processed files
   ├── encoding_times.txt    # Processing duration data
   ├── input_sizes.txt      # Original file sizes
   ├── output_sizes.txt     # Encoded file sizes
   ├── segments.json        # Segment tracking data
   └── encoding.json        # Encoding state information
   ```

8. **Final Validation Summary**
   - Overall compression statistics
   - Processing time analysis
   - Success/failure ratio
   - Resource usage summary
   - Quality metrics report

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

Hardware acceleration in drapto is specifically focused on video decoding to improve input processing performance:

1. **Platform Support**
   - macOS: VideoToolbox (decoding only)
     ```bash
     # Detection via FFmpeg
     ffmpeg -hide_banner -hwaccels | grep -q videotoolbox
     ```
   - Other platforms: Currently not supported
   - Future platforms can be added by extending the detection logic

2. **Detection Process**
   - Automatic platform detection using `$OSTYPE`
   - FFmpeg capability check for supported accelerators
   - Sets `HW_ACCEL` environment variable:
     - `videotoolbox` for macOS with VideoToolbox support
     - `none` when no acceleration is available
   - Logs detection results for debugging

3. **Implementation Details**
   - Applied only during video decoding phase
   - No hardware acceleration for encoding (always uses software SVT-AV1)
   - FFmpeg options set via `HWACCEL_OPTS` variable
   - Special handling for Dolby Vision content
   - Can be manually disabled via configuration

4. **Fallback Mechanism**
   - Primary attempt: Uses configured hardware acceleration
   - On failure:
     1. Logs failure with warning message
     2. Clears hardware acceleration options
     3. Automatically retries with software decoding
     4. Maintains all other encoding parameters
   - Fallback is transparent to the user
   - Performance impact is logged

5. **Configuration**
   - Hardware acceleration state tracked in `HWACCEL_OPTS`
   - Default: Enabled if available
   - Can be disabled through user configuration
   - Status logged during initialization
   - Applied consistently across all processing modes

6. **Logging and Diagnostics**
   - Hardware support detection logged
   - Acceleration mode changes tracked
   - Fallback events recorded
   - Performance metrics maintained
   - Error conditions documented in logs

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

## Error Recovery and Fallback Mechanisms

drapto implements a robust error recovery system, particularly focused on hardware-accelerated decoding failures and encoding issues:

1. **Hardware-Accelerated Decoding Fallback**
   - Automatically detects hardware decoding capabilities
   - Attempts hardware-accelerated decoding first (e.g., VideoToolbox on macOS)
   - On failure, gracefully falls back to software decoding
   - Maintains encoding parameters during fallback (SVT-AV1 software encoding is always used)
   - Logs hardware acceleration failures for diagnostics

2. **Decoding Recovery Process**
   - Primary attempt: Hardware-accelerated decoding
   - Final fallback: Pure software decoding
   - Encoding always uses software SVT-AV1 regardless of decoding method
   - Each stage maintains identical quality settings

3. **Error Reporting**
   - Detailed error logging for hardware decoding failures
   - Progress tracking during fallback attempts
   - Clear user feedback on decoding mode changes
   - Diagnostic information for troubleshooting

4. **Performance Implications**
   - Hardware-accelerated decoding: Faster input processing
   - Software decoding: Reduced performance but maximum compatibility
   - Encoding performance unaffected (always uses software SVT-AV1)
   - Automatic selection of optimal decoding path based on system capabilities

5. **Recovery Triggers**
   - Hardware decoder initialization failures
   - Memory allocation errors
   - Driver compatibility issues
   - Resource exhaustion
   - Codec support limitations

## Progress Tracking and Logging

drapto maintains comprehensive progress tracking and logging through a structured data file system:

1. **Tracking File Structure**
   ```
   TEMP_DIR/encode_data/
   ├── encoded_files.txt     # List of successfully processed files
   ├── encoding_times.txt    # Processing duration for each file
   ├── input_sizes.txt       # Original file sizes
   ├── output_sizes.txt      # Encoded file sizes
   ├── segments.json         # Segment tracking for chunked encoding
   └── encoding.json         # Encoding state and progress
   ```

2. **File Contents and Format**
   - **encoded_files.txt**
     ```
     /path/to/file1.mkv
     /path/to/file2.mkv
     ```
   - **encoding_times.txt**
     ```
     file1.mkv,3600      # Duration in seconds
     file2.mkv,7200
     ```
   - **segments.json**
     ```json
     {
       "total_segments": 120,
       "total_duration": 3600,
       "segments": [
         {
           "index": 0,
           "path": "segments/0001.mkv",
           "size": 15728640,
           "start_time": 0.0,
           "duration": 15.0,
           "created_at": "2024-01-18T00:00:00Z"
         }
       ]
     }
     ```

3. **Progress Monitoring**
   - Real-time tracking of:
     - Overall progress
     - Current file status
     - Segment processing status
     - Encoding performance
     - Resource utilization

4. **Performance Metrics**
   - Encoding speed (fps)
   - Compression ratio
   - Processing time per file
   - Size reduction statistics
   - Resource usage trends

5. **State Management**
   - Tracks encoding state per file
   - Maintains segment processing state
   - Records encoding strategy used
   - Preserves quality metrics
   - Stores error conditions

6. **Recovery Support**
   - Enables resume after interruption
   - Tracks partially completed files
   - Maintains segment completion status
   - Records failed attempts
   - Preserves encoding parameters

7. **Analysis Features**
   - Size reduction analysis
   - Processing time statistics
   - Quality metrics tracking
   - Error pattern detection
   - Performance optimization data

8. **Cleanup Policies**
   - Preserves logs for debugging
   - Maintains historical data
   - Removes temporary files
   - Archives completed job data
   - Manages disk space usage

## Directory Structure

drapto uses a structured directory layout to organize processing files and enable effective debugging:

1. **Root Structure**
   ```
   $HOME/projects/drapto/
   ├── input/                  # Source video files
   ├── output/                 # Encoded output files
   └── temp/                   # Temporary processing directory
       ├── logs/              # Processing logs
       ├── encode_data/       # State tracking
       ├── segments/          # Video segments
       ├── encoded/           # Encoded segments
       └── working/           # Active processing
   ```

2. **Temporary Directory Contents**
   - **logs/**
     - `*.log`: Per-file processing logs
     - `error_*.log`: Error condition logs
     - `debug_*.log`: Detailed debug information
   
   - **encode_data/**
     - `encoded_files.txt`: Successfully processed files
     - `encoding_times.txt`: Processing durations
     - `input_sizes.txt`: Original file sizes
     - `output_sizes.txt`: Encoded file sizes
     - `segments.json`: Segment tracking data
     - `encoding.json`: Encoding state information
   
   - **segments/**
     - `0001.mkv`, `0002.mkv`, etc.: Raw video segments
     - Preserved until successful encoding
     - Used for chunked encoding mode only
   
   - **encoded/**
     - `0001.mkv`, `0002.mkv`, etc.: Encoded segments
     - Intermediate files before final mux
     - Validated before concatenation
   
   - **working/**
     - `video.mkv`: Current video track
     - `audio-*.mkv`: Audio tracks
     - `concat.txt`: Segment list
     - `temp/`: Additional temporary files

3. **Debugging Tips**
   - Check `logs/` for detailed error information
   - Inspect `encode_data/` for progress tracking
   - Verify segment integrity in `segments/` and `encoded/`
   - Monitor active processing in `working/`
   - Use log files to track encoding decisions

4. **Cleanup Guidelines**
   - Temporary files auto-cleaned on successful completion
   - Manual cleanup may be needed after failures:
     ```bash
     # Clean temporary files
     rm -rf temp/working/* temp/segments/* temp/encoded/*
     
     # Preserve logs and tracking data
     rm -rf temp/working temp/segments temp/encoded
     
     # Full cleanup (including logs)
     rm -rf temp/*
     ```
   - Preserve logs for debugging failed encodes
   - Archive important logs before cleanup
   - Maintain tracking files for analysis

5. **Storage Management**
   - Monitor disk usage in temporary directories
   - Regular cleanup of old log files
   - Segment files can be large
   - Consider space requirements:
     - Source file size × 1.5 for temporary files
     - Additional space for encoded output
     - Log and tracking data (typically minimal)

6. **Recovery Procedures**
   - Interrupted jobs: Check `encode_data/` state
   - Failed segments: Inspect `encoded/` contents
   - Verify partial progress in tracking files
   - Resume from last successful segment
   - Preserve logs for troubleshooting