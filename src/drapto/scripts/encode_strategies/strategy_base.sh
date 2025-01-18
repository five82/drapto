#!/usr/bin/env bash

###################
# Base Encoding Strategy
###################

source "${SCRIPT_DIR}/utils/formatting.sh"

# Initialize the encoding strategy with input file and options
# Args:
#   $1: Input file path
#   $2: Output file path
#   $3: Options string (optional)
initialize_encoding() {
    error "Strategy must implement initialize_encoding"
    exit 1
}

# Prepare the video for encoding (e.g., segmentation, option setup)
# Args:
#   $1: Input file path
#   $2: Output file path
#   $3: Options string (optional)
prepare_video() {
    error "Strategy must implement prepare_video"
    exit 1
}

# Perform the actual encoding
# Args:
#   $1: Input file path
#   $2: Output file path
#   $3: Options string (optional)
encode_video() {
    error "Strategy must implement encode_video"
    exit 1
}

# Finalize the encoding process (e.g., concatenation, cleanup)
# Args:
#   $1: Input file path
#   $2: Output file path
#   $3: Options string (optional)
finalize_encoding() {
    error "Strategy must implement finalize_encoding"
    exit 1
}

# Validate that the strategy can handle this input
# Returns:
#   0 if strategy can handle input, 1 otherwise
# Args:
#   $1: Input file path
can_handle() {
    error "Strategy must implement can_handle"
    exit 1
}
