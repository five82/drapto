#!/usr/bin/env bash

###################
# Subtitle Functions
###################

source "${SCRIPT_DIR}/encode_formatting.sh"

# Set up subtitle options based on input file
setup_subtitle_options() {
    local input_file="$1"
    local subtitle_opts=""

    local subtitle_count
    subtitle_count=$("${FFPROBE}" -v error -select_streams s -show_entries stream=index -of csv=p=0 "${input_file}" | wc -l)

    if [ "$subtitle_count" -gt 0 ]; then
        subtitle_opts="-c:s copy"
        print_check "Found $(print_stat "$subtitle_count") subtitle stream(s)"
    else
        print_check "No subtitle streams found"
    fi

    printf "%s" "${subtitle_opts}"
} 