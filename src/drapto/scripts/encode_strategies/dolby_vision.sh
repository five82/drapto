#!/usr/bin/env bash

###################
# Dolby Vision Strategy
###################

source "${SCRIPT_DIR}/utils/formatting.sh"
source "${SCRIPT_DIR}/common/video_utils.sh"
source "${SCRIPT_DIR}/encode_strategies/strategy_base.sh"

# Implementation of strategy interface

# Initialize Dolby Vision encoding
# Args:
#   $1: Input file path
#   $2: Output file path
#   $3: Options string (optional)
initialize_encoding() {
    local input_file="$1"
    local output_file="$2"
    
    print_check "Initializing Dolby Vision encoding strategy..."
    
    # Create working directory
    mkdir -p "$WORKING_DIR"
    
    # Check for hardware acceleration only on macOS
    if [[ "$OSTYPE" == "darwin"* ]]; then
        print_check "macOS detected, checking hardware acceleration..."
        check_hardware_acceleration
        HWACCEL_OPTS=$(configure_hw_accel_options)
    else
        HWACCEL_OPTS=""
    fi
    
    return 0
}

# Prepare video by setting up DV options
# Args:
#   $1: Input file path
#   $2: Output file path
#   $3: Options string (optional)
prepare_video() {
    local input_file="$1"
    local output_file="$2"
    
    print_check "Preparing video for Dolby Vision encoding..."
    
    # Configure video options
    VIDEO_OPTS=$(setup_video_options "$input_file" "$DISABLE_CROP")
    if [[ $? -ne 0 ]]; then
        error "Failed to configure video options"
        return 1
    fi
    
    return 0
}

# Encode video with DV settings
# Args:
#   $1: Input file path
#   $2: Output file path
#   $3: Options string (optional)
encode_video() {
    local input_file="$1"
    local output_file="$2"
    
    print_check "Encoding video with Dolby Vision..."
    
    # Create temporary file for video track
    local temp_video="${WORKING_DIR}/temp_video.mkv"
    
    # Encode video track
    if ! "${FFMPEG}" -hide_banner -y -i "$input_file" \
        ${HWACCEL_OPTS} \
        ${VIDEO_OPTS} \
        -an \
        "$temp_video"; then
        error "Failed to encode video track"
        return 1
    fi
    
    # Move temp file to output
    mv "$temp_video" "$output_file"
    
    return 0
}

# Finalize encoding process
# Args:
#   $1: Input file path
#   $2: Output file path
#   $3: Options string (optional)
finalize_encoding() {
    local input_file="$1"
    local output_file="$2"
    
    print_check "Finalizing Dolby Vision encoding..."
    
    # Cleanup temporary files
    cleanup_temp_files
    
    return 0
}

# Check if this strategy can handle the input
# Args:
#   $1: Input file path
can_handle() {
    local input_file="$1"
    
    # Only handle Dolby Vision content
    if detect_dolby_vision "$input_file"; then
        return 0
    fi
    return 1
}

# Get video encoding options based on input file analysis
get_video_encode_options() {
    local input_file="$1"
    local IS_DOLBY_VISION="$2"
    local crop_filter="$3"
    local crf="$4"

    # Base video encoding options
    local video_opts="-c:v libsvtav1 \
        -preset ${PRESET} \
        -crf ${crf} \
        -svtav1-params tune=0:film-grain=0:film-grain-denoise=0"

    if [[ "$IS_DOLBY_VISION" == "true" ]]; then
        video_opts+=" -dolbyvision true"
    fi

    # Add crop filter if provided
    if [[ -n "$crop_filter" ]]; then
        video_opts+=" -vf $crop_filter"
    fi

    echo "$video_opts"
}

# Set up video encoding options based on input file
setup_video_options() {
    local input_file="$1"
    local disable_crop="$2"
    local video_opts=""

    # Get video width
    local width
    width=$("${FFPROBE}" -v error -select_streams v:0 -show_entries stream=width -of default=noprint_wrappers=1:nokey=1 "${input_file}")

    # Set quality based on resolution
    local crf
    if [ "$width" -ge 3840 ]; then
        crf=$CRF_UHD
        print_check "UHD quality detected ($(print_stat "${width}px") width), using CRF $(print_stat "$crf")"
    elif [ "$width" -ge 1920 ]; then
        crf=$CRF_HD
        print_check "HD quality detected ($(print_stat "${width}px") width), using CRF $(print_stat "$crf")"
    else
        crf=$CRF_SD
        print_check "SD quality detected ($(print_stat "${width}px") width), using CRF $(print_stat "$crf")"
    fi

    # Detect crop values
    local crop_filter
    crop_filter=$(detect_crop "${input_file}" "${disable_crop}")
    
    # Get video encoding options
    video_opts=$(get_video_encode_options "${input_file}" "${IS_DOLBY_VISION}" "${crop_filter}" "${crf}")

    printf "%s" "${video_opts}"
}
