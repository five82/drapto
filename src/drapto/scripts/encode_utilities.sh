#!/usr/bin/env bash

###################
# Utility Functions
###################

# Print error message in red
print_error() {
    echo -e "\033[31mError: $1\033[0m" >&2
}

# Check for required dependencies
check_dependencies() {
    # Check that ffmpeg/ffprobe exist and are executable
    if [ ! -x "$FFMPEG" ] || [ ! -x "$FFPROBE" ]; then
        echo "Error: ffmpeg/ffprobe not found or not executable at:"
        echo "FFMPEG=$FFMPEG"
        echo "FFPROBE=$FFPROBE"
        return 1
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

# Initialize base directories and create if needed
initialize_base_directories() {
    local base_dirs=(
        "${SCRIPT_DIR}/videos"
        "${LOG_DIR}"
        "${TEMP_DIR}"
        "${TEMP_DATA_DIR}"
        "${SEGMENTS_DIR}"
        "${ENCODED_SEGMENTS_DIR}"
    )

    for dir in "${base_dirs[@]}"; do
        if [[ ! -d "$dir" ]]; then
            mkdir -p "$dir" || {
                print_error "Failed to create directory: $dir"
                return 1
            }
            chmod 775 "$dir" || {
                print_error "Failed to set permissions on directory: $dir"
                return 1
            }
        fi
    done

    # Initialize JSON tracking files if they don't exist
    local segments_json="${TEMP_DATA_DIR}/segments.json"
    local encoding_json="${TEMP_DATA_DIR}/encoding.json"

    if [[ ! -f "$segments_json" ]]; then
        echo '{"segments":[],"total_segments":0,"total_duration":0.0,"created_at":null,"updated_at":null}' > "$segments_json" || {
            print_error "Failed to create segments.json"
            return 1
        }
        chmod 664 "$segments_json"
    fi

    if [[ ! -f "$encoding_json" ]]; then
        echo '{"segments":{},"created_at":null,"updated_at":null}' > "$encoding_json" || {
            print_error "Failed to create encoding.json"
            return 1
        }
        chmod 664 "$encoding_json"
    fi

    # Update timestamps in tracking files
    local current_time
    current_time=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    
    # Set up Python environment
    local parent_dir
    parent_dir="$(cd "$(dirname "${SCRIPT_DIR}")" && pwd)"
    export PYTHONPATH="${parent_dir}:${PYTHONPATH:-}"
    
    # Debug output
    echo "Debug: SCRIPT_DIR = ${SCRIPT_DIR}"
    echo "Debug: Parent dir = ${parent_dir}"
    echo "Debug: Using PYTHONPATH = ${PYTHONPATH}"
    echo "Debug: Running json_helper.py from ${SCRIPT_DIR}/encode_strategies/json_helper.py"
    
    # Update timestamps in tracking files using direct script execution with full path
    cd "${SCRIPT_DIR}/encode_strategies" && \
    python3 ./json_helper.py update_timestamps "${TEMP_DATA_DIR}" "${current_time}" || {
        print_error "Failed to update timestamps in tracking files"
        return 1
    }

    return 0
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