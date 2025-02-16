#!/usr/bin/env bash

###################
# Utility Functions
###################

# Check for required dependencies
check_dependencies() {
    local use_local=false
    local use_system=false

    # Check local ffmpeg/ffprobe
    if [ -x "$FFMPEG" ] && [ -x "$FFPROBE" ]; then
        use_local=true
    else
        # If local is not available, check system ffmpeg and ffprobe
        if command -v ffmpeg >/dev/null 2>&1 && command -v ffprobe >/dev/null 2>&1; then
            use_system=true
        else
            echo "Error: Neither local ffmpeg/ffprobe nor system ffmpeg/ffprobe found."
            echo "Please install ffmpeg and ffprobe on your system or run build_ffmpeg.sh."
            return 1
        fi
    fi

    # If using local binaries
    if [ "$use_local" = true ]; then
        echo "Using local ffmpeg binary: ${FFMPEG}"
        echo "Using local ffprobe binary: ${FFPROBE}"
    fi

    # If using system binaries, update variables
    if [ "$use_system" = true ]; then
        FFMPEG="ffmpeg"
        FFPROBE="ffprobe"
        echo "Local ffmpeg/ffprobe not found."
        echo "Using system ffmpeg: $(command -v ffmpeg)"
        echo "Using system ffprobe: $(command -v ffprobe)"
    fi

    # Check for other dependencies: mediainfo and bc
    for cmd in mediainfo bc; do
        if ! command -v "$cmd" >/dev/null 2>&1; then
            echo "Error: $cmd not found. Please install $cmd first."
            return 1
        fi
    done

    return 0
}

# Initialize required directories
initialize_directories() {
    mkdir -p "${SCRIPT_DIR}/videos" "${INPUT_DIR}" "${OUTPUT_DIR}" "${LOG_DIR}"
}

# Get file size in bytes
get_file_size() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        stat -f%z "$1"
    else
        stat -c%s "$1"
    fi
}

# Get current timestamp in YYYYMMDD_HHMMSS format
get_timestamp() {
    date "+%Y%m%d_%H%M%S"
}

# Format file size for display (converts bytes to a human-readable format)
format_size() {
    local size=$1
    local scale=0
    local suffix=("B" "KiB" "MiB" "GiB" "TiB")

    while [ "$(echo "$size > 1024" | bc -l)" -eq 1 ] && [ $scale -lt 4 ]; do
        size=$(echo "scale=1; $size / 1024" | bc)
        ((scale++))
    done

    echo "$size"
}

# Print an error message
error() {
    echo -e "\e[31m✗ $1\e[0m" >&2
}

# Check if GNU Parallel is installed and provide installation instructions if needed
check_parallel_installation() {
    if ! command -v parallel >/dev/null 2>&1; then
        error "GNU Parallel is not installed"
        
        # Check if we're on macOS or Linux
        if [[ "$(uname)" == "Darwin" ]] || [[ "$(uname)" == "Linux" ]]; then
            # Check if Homebrew is installed
            if command -v brew >/dev/null 2>&1; then
                echo "You can install GNU Parallel using Homebrew:"
                echo "    brew install parallel"
            else
                echo "Homebrew is not installed. You can install it first:"
                if [[ "$(uname)" == "Darwin" ]]; then
                    echo "    /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
                else
                    echo "    /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
                    echo "    (echo; echo 'eval \"\$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)\"') >> \"$HOME/.bashrc\""
                fi
                echo "Then install GNU Parallel:"
                echo "    brew install parallel"
            fi
        else
            echo "Please install GNU Parallel using your system's package manager"
        fi
        return 1
    fi
    return 0
}