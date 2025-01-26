#!/usr/bin/env bash

###################
# Validation Functions
###################

# Validate the output file to ensure encoding was successful
validate_output() {
    local output_file="$1"
    local error=0

    echo "Validating output file..."

    # Check if file exists and has size
    if [ ! -s "$output_file" ]; then
        echo "Error: Output file is empty or doesn't exist"
        return 1
    fi

    # Check video stream
    local video_stream
    video_stream=$("${FFPROBE}" -v error -select_streams v -show_entries stream=codec_name -of default=noprint_wrappers=1:nokey=1 "$output_file")
    if [ "$video_stream" != "av1" ]; then
        echo "Error: No AV1 video stream found in output"
        error=1
    else
        local duration
        duration=$("${FFPROBE}" -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 "$output_file")
        echo "Video stream: AV1, Duration: ${duration}s"
    fi

    # Check audio streams
    local audio_count
    audio_count=$("${FFPROBE}" -v error -select_streams a -show_entries stream=codec_name -of default=noprint_wrappers=1:nokey=1 "$output_file" | grep -c "opus" || true)
    if [ "$audio_count" -eq 0 ]; then
        echo "Error: No Opus audio streams found in output"
        error=1
    else
        echo "Audio streams: $audio_count Opus stream(s)"
    fi

    # Compare input and output duration
    local input_duration output_duration
    input_duration=$("${FFPROBE}" -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 "${input_file}")
    output_duration=$("${FFPROBE}" -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 "$output_file")

    # Allow 1 second difference
    local duration_diff
    duration_diff=$(awk "BEGIN {print sqrt(($input_duration - $output_duration)^2)}")
    if (( $(echo "$duration_diff > 1" | bc -l) )); then
        echo "Error: Output duration ($output_duration) differs significantly from input ($input_duration)"
        error=1
    fi

    # Check if ab-av1 is available when chunked encoding is enabled
    if [[ "$ENABLE_CHUNKED_ENCODING" == "true" ]]; then
        print_check "Checking for ab-av1..."
        if ! command -v ab-av1 >/dev/null 2>&1; then
            error "ab-av1 is required for chunked encoding but not found. Install with: cargo install ab-av1"
            error=1
        fi
        print_check "ab-av1 found"
    fi

    if [ $error -eq 0 ]; then
        echo "Output validation successful"
        return 0
    else
        echo "Output validation failed"
        return 1
    fi
} 

# Check if ab-av1 is available when chunked encoding is enabled
validate_ab_av1() {
    if [[ "$ENABLE_CHUNKED_ENCODING" == "true" ]]; then
        print_check "Checking for ab-av1..."
        if ! command -v ab-av1 >/dev/null 2>&1; then
            error "ab-av1 is required for chunked encoding but not found. Install with: cargo install ab-av1"
            return 1
        fi
        print_check "ab-av1 found"
    fi
    return 0
}