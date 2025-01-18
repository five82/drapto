#!/usr/bin/env bash

###################
# File Operations
###################

source "${SCRIPT_DIR}/utils/formatting.sh"

# Clean up temporary files
cleanup_temp_files() {
    print_check "Cleaning up temporary files..."
    rm -rf "$SEGMENTS_DIR" "$ENCODED_SEGMENTS_DIR" "$WORKING_DIR"
    mkdir -p "$WORKING_DIR"
}
