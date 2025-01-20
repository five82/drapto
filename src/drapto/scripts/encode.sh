#!/usr/bin/env bash

# Set up environment
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
export LD_LIBRARY_PATH="/home/linuxbrew/.linuxbrew/lib:"

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Validate script directory
[[ -z "$SCRIPT_DIR" ]] && { echo "Error: Could not determine script directory"; exit 1; }
[[ ! -d "$SCRIPT_DIR" ]] && { echo "Error: Script directory not found: $SCRIPT_DIR"; exit 1; }

echo "Debug: Using script directory: $SCRIPT_DIR"
echo "Debug: Script files:"
ls -la "$SCRIPT_DIR"
echo "Debug: Common files:"
ls -la "$SCRIPT_DIR/common"

# Source required files
source "$SCRIPT_DIR/common/config.sh"

# Initialize base directories first
source "$SCRIPT_DIR/encode_utilities.sh"
initialize_base_directories

# Source remaining files
source "$SCRIPT_DIR/encode_video_functions.sh"
source "$SCRIPT_DIR/common/audio_processing.sh"
source "$SCRIPT_DIR/encode_subtitle_functions.sh"
source "$SCRIPT_DIR/encode_hardware_acceleration.sh"
source "$SCRIPT_DIR/encode_validation.sh"
source "$SCRIPT_DIR/encode_processing.sh"

# Run main processing function
main
