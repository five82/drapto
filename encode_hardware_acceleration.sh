#!/usr/bin/env bash

##############################
# Hardware Acceleration Functions
##############################

# Check for hardware acceleration support
check_hardware_acceleration() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # Check for macOS VideoToolbox support using local ffmpeg binary
        if "${FFMPEG}" -hide_banner -hwaccels | grep -q videotoolbox; then
            echo "Found VideoToolbox hardware acceleration"
            export HW_ACCEL="videotoolbox"
            return 0
        fi
    fi

    # No supported hardware acceleration found
    echo "No supported hardware acceleration found, using software decoding"
    export HW_ACCEL="none"
    return 1
} 