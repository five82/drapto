#!/usr/bin/env bash

###################
# Configuration
###################

# Check for local ffmpeg/ffprobe first
if [[ -f "$HOME/ffmpeg/ffmpeg" ]] && [[ -f "$HOME/ffmpeg/ffprobe" ]]; then
    FFMPEG="$HOME/ffmpeg/ffmpeg"
    FFPROBE="$HOME/ffmpeg/ffprobe"
else
    # Fall back to system ffmpeg/ffprobe
    FFMPEG="/home/linuxbrew/.linuxbrew/bin/ffmpeg"
    FFPROBE="/home/linuxbrew/.linuxbrew/bin/ffprobe"
fi

echo "Debug: Using ffmpeg: ${FFMPEG}"
echo "Debug: Using ffprobe: ${FFPROBE}"

# Verify script directory is set
if [[ -z "${SCRIPT_DIR}" ]]; then
    echo "✗ Script directory not specified" >&2
    exit 1
fi

if [[ ! -d "${SCRIPT_DIR}" ]]; then
    echo "✗ Script directory not found: ${SCRIPT_DIR}" >&2
    exit 1
fi

echo "Debug: Using script directory: ${SCRIPT_DIR}"

# Use environment variables for paths
INPUT_FILE="${DRAPTO_INPUT_FILE}"
OUTPUT_FILE="${DRAPTO_OUTPUT_FILE}"
TEMP_DIR="${DRAPTO_TEMP_DIR}"
LOG_DIR="${DRAPTO_LOG_DIR}"
TEMP_DATA_DIR="${DRAPTO_TEMP_DATA_DIR}"
SEGMENTS_DIR="${DRAPTO_SEGMENTS_DIR}"
ENCODED_SEGMENTS_DIR="${DRAPTO_ENCODED_SEGMENTS_DIR}"
WORKING_DIR="${DRAPTO_WORKING_DIR}"

echo "Debug: Paths from environment:"
echo "  INPUT_FILE=${INPUT_FILE}"
echo "  OUTPUT_FILE=${OUTPUT_FILE}"
echo "  TEMP_DIR=${TEMP_DIR}"
echo "  LOG_DIR=${LOG_DIR}"
echo "  TEMP_DATA_DIR=${TEMP_DATA_DIR}"
echo "  SEGMENTS_DIR=${SEGMENTS_DIR}"
echo "  ENCODED_SEGMENTS_DIR=${ENCODED_SEGMENTS_DIR}"
echo "  WORKING_DIR=${WORKING_DIR}"

# Verify required paths are set
for var in INPUT_FILE OUTPUT_FILE TEMP_DIR LOG_DIR TEMP_DATA_DIR SEGMENTS_DIR ENCODED_SEGMENTS_DIR WORKING_DIR; do
    if [[ -z "${!var}" ]]; then
        echo "✗ ${var} not specified" >&2
        exit 1
    fi
done

# Verify input file exists
if [[ ! -f "${INPUT_FILE}" ]]; then
    echo "✗ Input file not found: ${INPUT_FILE}" >&2
    exit 1
fi

# Verify directories exist
for dir in TEMP_DIR LOG_DIR TEMP_DATA_DIR SEGMENTS_DIR ENCODED_SEGMENTS_DIR WORKING_DIR; do
    if [[ ! -d "${!dir}" ]]; then
        echo "✗ Directory not found: ${!dir}" >&2
        exit 1
    fi
done

# Verify output directory exists
if [[ ! -d "$(dirname "${OUTPUT_FILE}")" ]]; then
    echo "✗ Output directory not found: $(dirname "${OUTPUT_FILE}")" >&2
    exit 1
fi

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

# Arrays to store encoding information
declare -a encoded_files
declare -a encoding_times
declare -a input_sizes
declare -a output_sizes
