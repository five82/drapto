#!/usr/bin/env bash

###################
# Configuration
###################

# Paths (assumes SCRIPT_DIR is set before sourcing this file)

# Check for local ffmpeg/ffprobe first
if [[ -f "$HOME/ffmpeg/ffmpeg" ]] && [[ -f "$HOME/ffmpeg/ffprobe" ]]; then
    FFMPEG="$HOME/ffmpeg/ffmpeg"
    FFPROBE="$HOME/ffmpeg/ffprobe"
else
    # Fall back to system ffmpeg/ffprobe
    FFMPEG="/home/linuxbrew/.linuxbrew/bin/ffmpeg"
    FFPROBE="/home/linuxbrew/.linuxbrew/bin/ffprobe"
fi

# Default paths if not set by environment
INPUT_DIR="${INPUT_DIR:-${SCRIPT_DIR}/videos/input}"
OUTPUT_DIR="${OUTPUT_DIR:-${SCRIPT_DIR}/videos/output}"
LOG_DIR="${LOG_DIR:-${SCRIPT_DIR}/videos/logs}"

# Encoding settings
PRESET=6
CRF_SD=25     # For videos with width <= 1280 (720p)
CRF_HD=25     # For videos with width <= 1920 (1080p)
CRF_UHD=29    # For videos with width > 1920 (4K and above)
SVT_PARAMS="tune=0:film-grain=0:film-grain-denoise=0"
PIX_FMT="yuv420p10le"

# Hardware acceleration options (will be set during initialization)
HWACCEL_OPTS=""

# Dolby Vision detection flag
IS_DOLBY_VISION=false

# Cropping settings
DISABLE_CROP=false

# Chunked encoding settings
ENABLE_CHUNKED_ENCODING=true
SEGMENT_LENGTH=15
TARGET_VMAF=93
VMAF_SAMPLE_COUNT=3
VMAF_SAMPLE_LENGTH=1

# Temporary directories for chunked encoding
TEMP_DIR="${TEMP_DIR:-${SCRIPT_DIR}/videos/temp}"
SEGMENTS_DIR="${TEMP_DIR}/segments"
ENCODED_SEGMENTS_DIR="${TEMP_DIR}/encoded_segments"
WORKING_DIR="${TEMP_DIR}/working"

# Arrays to store encoding information
declare -a encoded_files
declare -a encoding_times
declare -a input_sizes
declare -a output_sizes
