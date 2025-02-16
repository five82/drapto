#!/usr/bin/env bash

# Add cargo bin to PATH
export PATH="/home/ken/.cargo/bin:${PATH}"

# Add near the top of the script, after the shebang
export LD_LIBRARY_PATH="/home/linuxbrew/.linuxbrew/lib:${LD_LIBRARY_PATH}"

# Determine the script directory
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
else
    # Linux and others
    SCRIPT_DIR="$( cd "$( dirname "$(readlink -f "${BASH_SOURCE[0]}")" )" && pwd )"
fi

# Source configuration and function files with consistent naming
source "${SCRIPT_DIR}/encode_config.sh"
source "${SCRIPT_DIR}/encode_utilities.sh"
source "${SCRIPT_DIR}/encode_video_functions.sh"
source "${SCRIPT_DIR}/encode_audio_functions.sh"
source "${SCRIPT_DIR}/encode_subtitle_functions.sh"
source "${SCRIPT_DIR}/encode_hardware_acceleration.sh"
source "${SCRIPT_DIR}/encode_validation.sh"
source "${SCRIPT_DIR}/encode_processing.sh"

###################
# Main Processing
###################

# Start the encoding process
main
