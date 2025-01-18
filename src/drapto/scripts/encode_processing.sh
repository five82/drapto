#!/usr/bin/env bash

###################
# Encoding Functions
###################

source "${SCRIPT_DIR}/utils/formatting.sh"
source "${SCRIPT_DIR}/encode_hardware_acceleration.sh"
source "${SCRIPT_DIR}/common/audio_processing.sh"
source "${SCRIPT_DIR}/common/video_utils.sh"
source "${SCRIPT_DIR}/common/file_operations.sh"
source "${SCRIPT_DIR}/encode_strategies/chunked_encoding.sh"
source "${SCRIPT_DIR}/encode_strategies/dolby_vision.sh"

# Initialize directories and create if needed
initialize_directories() {
    # Create output directory if it doesn't exist
    if [[ ! -d "$OUTPUT_DIR" ]]; then
        mkdir -p "$OUTPUT_DIR"
    fi
    
    # Verify input directory exists and contains video files
    if [[ ! -d "$INPUT_DIR" ]]; then
        print_error "Input directory not found: $INPUT_DIR"
        exit 1
    fi

    # Create encode data directory and initialize files
    mkdir -p "${TEMP_DATA_DIR}"
    
    # Create empty files if they don't exist
    if [[ ! -f "${ENCODED_FILES_DATA}" ]]; then
        touch "${ENCODED_FILES_DATA}"
    fi
    if [[ ! -f "${ENCODING_TIMES_DATA}" ]]; then
        touch "${ENCODING_TIMES_DATA}"
    fi
    if [[ ! -f "${INPUT_SIZES_DATA}" ]]; then
        touch "${INPUT_SIZES_DATA}"
    fi
    if [[ ! -f "${OUTPUT_SIZES_DATA}" ]]; then
        touch "${OUTPUT_SIZES_DATA}"
    fi
}

# Clean up temporary files
cleanup_temp_files() {
    rm -rf "${TEMP_DIR}"
}

# Print section header with nice formatting
print_header() {
    local title="$1"
    local width=80
    local padding=$(( (width - ${#title}) / 2 ))
    
    echo
    printf '%*s' "$width" | tr ' ' '='
    echo
    printf "%*s%s%*s\n" "$padding" "" "$title" "$padding" ""
    printf '%*s' "$width" | tr ' ' '='
    echo
    echo
}

# Print section separator
print_separator() {
    echo "----------------------------------------"
}

# Process a single file (perform encoding)
process_file() {
    local input_file="$1"
    local output_file="$2"
    local log_file="$3"
    local filename="$4"

    print_header "Starting Encode"
    
    echo "Input file:  $(print_path "${input_file}")"
    echo "Output file: $(print_path "${output_file}")"
    print_separator
    echo "Processing: $(print_path "${filename}")"

    # Reset global variables
    IS_DOLBY_VISION=false
    HWACCEL_OPTS=""

    # Detect Dolby Vision and set hardware acceleration options
    detect_dolby_vision "${input_file}"
    HWACCEL_OPTS=$(configure_hw_accel_options)

    # Get number of audio tracks
    local num_audio_tracks
    num_audio_tracks=$("${FFPROBE}" -v error -select_streams a -show_entries stream=index -of csv=p=0 "$input_file" | wc -l)

    # Load the appropriate encoding strategy based on content type
    load_encoding_strategy

    # Process based on content type and chunked encoding setting
    if [[ "$IS_DOLBY_VISION" == true ]]; then
        print_check "Processing Dolby Vision content..."
    fi

    if [[ "$IS_DOLBY_VISION" == false ]] && [[ "$ENABLE_CHUNKED_ENCODING" == true ]]; then
        # Detect crop values once for the entire video
        local crop_filter
        crop_filter=$(detect_crop "$input_file" "$DISABLE_CROP")

        # Create temporary directories
        mkdir -p "$TEMP_DIR" "$SEGMENTS_DIR" "$ENCODED_SEGMENTS_DIR" "$WORKING_DIR"

        # Initialize chunked encoding
        if ! initialize_encoding "$input_file" "$output_file"; then
            error "Failed to initialize chunked encoding"
            cleanup_temp_files
            return 1
        fi

        # Step 1: Segment video
        segment_video "$input_file" "$SEGMENTS_DIR"

        # Step 2: Encode segments
        if ! encode_segments "$SEGMENTS_DIR" "$ENCODED_SEGMENTS_DIR" "$TARGET_VMAF" "$DISABLE_CROP" "$crop_filter"; then
            error "Failed to encode segments"
            cleanup_temp_files "$TEMP_DIR"
            return 1
        fi

        # Step 3: Concatenate encoded segments
        if ! concatenate_segments "${WORKING_DIR}/video.mkv"; then
            error "Failed to concatenate segments"
            cleanup_temp_files
            return 1
        fi

        # Step 4: Process audio tracks
        for ((i=0; i<num_audio_tracks; i++)); do
            process_audio_track "$input_file" "$i" "${WORKING_DIR}/audio-${i}.mkv"
        done

        # Step 5: Mux everything together
        mux_tracks "${WORKING_DIR}/video.mkv" "$output_file" "$num_audio_tracks"

        # Cleanup
        cleanup_temp_files
    else
        print_check "Using standard encoding process..."
        # Process audio tracks separately for consistency
        for ((i=0; i<num_audio_tracks; i++)); do
            process_audio_track "$input_file" "$i" "${WORKING_DIR}/audio-${i}.mkv"
        done

        # Process video track
        process_video_track "$input_file" "${WORKING_DIR}/video.mkv"

        # Mux everything together
        mux_tracks "${WORKING_DIR}/video.mkv" "$output_file" "$num_audio_tracks"

        # Cleanup
        cleanup_temp_files
    fi

    # Record sizes and validate output
    local input_size
    input_size=$(get_file_size "$input_file")
    local output_size
    output_size=$(get_file_size "$output_file")
}

# Process a single video track
process_video_track() {
    local input_file="$1"
    local output_file="$2"

    print_check "Processing video track..."

    # Setup video encoding options
    local video_opts
    video_opts=$(setup_video_options "${input_file}" "${DISABLE_CROP}")

    # Execute ffmpeg command
    if ! "${FFMPEG}" -hide_banner -loglevel warning ${HWACCEL_OPTS} \
        -i "${input_file}" \
        -map 0:v:0 \
        ${video_opts} \
        -stats \
        -y "${output_file}"; then

        # If hardware acceleration fails, retry with software decoding
        if [[ -n "${HWACCEL_OPTS}" ]]; then
            print_warning "Hardware acceleration failed, falling back to software decoding..."
            "${FFMPEG}" -hide_banner -loglevel warning \
                -i "${input_file}" \
                -map 0:v:0 \
                ${video_opts} \
                -stats \
                -y "${output_file}" || error "Video encoding failed"
        else
            error "Video encoding failed"
        fi
    fi
}

# Mux video and audio tracks together
mux_tracks() {
    local video_file="$1"
    local output_file="$2"
    local num_audio_tracks="$3"

    print_check "Muxing tracks..."

    # Build ffmpeg command
    local -a cmd=("${FFMPEG}" -hide_banner -loglevel warning)
    
    # Add video input
    cmd+=(-i "$video_file")

    # Add audio inputs
    for ((i=0; i<num_audio_tracks; i++)); do
        cmd+=(-i "${WORKING_DIR}/audio-${i}.mkv")
    done

    # Add mapping
    cmd+=(-map "0:v:0")  # Video track
    for ((i=1; i<=num_audio_tracks; i++)); do
        cmd+=(-map "${i}:a:0")  # Audio tracks
    done

    # Add output file
    cmd+=(-c copy -y "$output_file")

    # Execute command
    if ! "${cmd[@]}"; then
        error "Failed to mux tracks"
    fi

    print_success "Successfully muxed all tracks"
}

# Concatenate segments
concatenate_segments() {
    # Ensure working directory exists
    mkdir -p "${WORKING_DIR}"
    
    local concat_file="${WORKING_DIR}/concat.txt"
    > "$concat_file"
    
    # Add each segment to concat file in order
    for segment in "${ENCODED_SEGMENTS_DIR}"/*.mkv; do
        if [[ -f "$segment" ]]; then
            echo "file '${segment}'" >> "$concat_file"
        fi
    done
    
    if [[ ! -f "$concat_file" ]]; then
        print_error "Failed to create concat file"
        return 1
    fi
    
    # Run ffmpeg concat
    "$FFMPEG" -y -f concat -safe 0 -i "$concat_file" -c copy "${WORKING_DIR}/video.mkv"
    local status=$?
    
    if [[ $status -ne 0 ]]; then
        print_error "Failed to concatenate segments"
        return $status
    fi
    
    return 0
}

# Handle processing of a single input file
process_single_file() {
    local input_file="$1"
    local filename
    filename=$(basename "$input_file")
    local output_file="${OUTPUT_DIR}/${filename}"
    local log_file="${LOG_DIR}/${filename%.*}.log"

    # Start timing this file
    local file_start_time
    file_start_time=$(date +%s)

    process_file "${input_file}" "${output_file}" "${log_file}" "${filename}" 2>&1 | tee "${log_file}"

    if [ ${PIPESTATUS[0]} -eq 0 ]; then
        handle_successful_encode "$filename" "$file_start_time" "$input_file" "$output_file"
    else
        echo "Error encoding ${filename}"
    fi
}

# Handle actions after a successful encode
handle_successful_encode() {
    local filename="$1"
    local start_time="$2"
    local input_file="$3"
    local output_file="$4"

    # Calculate encoding time
    local end_time
    end_time=$(date +%s)
    local encoding_time=$((end_time - start_time))

    # Get file sizes
    local input_size
    input_size=$(get_file_size "$input_file")
    local output_size
    output_size=$(get_file_size "$output_file")

    # Store the results for final summary
    encoded_files+=("$filename")
    encoding_times+=("$encoding_time")
    input_sizes+=("$input_size")
    output_sizes+=("$output_size")

    # Save arrays immediately after updating
    save_arrays

    # Print individual encode summary
    print_header "Individual File Encoding Summary"
    printf "\n"
    printf "File: %s\n" "$filename"
    printf "Input size:  %s\n" "$(format_size "$input_size")"
    printf "Output size: %s\n" "$(format_size "$output_size")"
    printf "Reduction:   %.2f%%\n" "$(calculate_reduction "$input_size" "$output_size")"
    print_separator
    printf "Encoding time: %s\n" "$(format_time "$encoding_time")"
    printf "Finished encode at %s\n" "$(date)"
    print_separator
}

# Save arrays to files
save_arrays() {
    # Create temp directory if it doesn't exist
    mkdir -p "$TEMP_DATA_DIR"

    # Save each array to a temporary file first
    local temp_files=("${ENCODED_FILES_DATA}.tmp" "${ENCODING_TIMES_DATA}.tmp" "${INPUT_SIZES_DATA}.tmp" "${OUTPUT_SIZES_DATA}.tmp")
    local final_files=("$ENCODED_FILES_DATA" "$ENCODING_TIMES_DATA" "$INPUT_SIZES_DATA" "$OUTPUT_SIZES_DATA")
    local arrays=("${encoded_files[*]}" "${encoding_times[*]}" "${input_sizes[*]}" "${output_sizes[*]}")

    for i in "${!temp_files[@]}"; do
        # Use printf to ensure proper handling of special characters and ensure newline
        printf "%s\n" "${arrays[$i]}" > "${temp_files[$i]}"
        sync "${temp_files[$i]}" # Force write to disk
        
        # Move temp file to final file
        mv "${temp_files[$i]}" "${final_files[$i]}"
        sync "${final_files[$i]}" # Force write to disk
    done
}

# Load arrays from files if they exist
load_arrays() {
    # Check if the main file exists and has content
    if [ ! -s "$ENCODED_FILES_DATA" ]; then
        # Initialize empty arrays
        encoded_files=()
        encoding_times=()
        input_sizes=()
        output_sizes=()
    else
        # Read arrays from files
        mapfile -t encoded_files < "$ENCODED_FILES_DATA"
        mapfile -t encoding_times < "$ENCODING_TIMES_DATA"
        mapfile -t input_sizes < "$INPUT_SIZES_DATA"
        mapfile -t output_sizes < "$OUTPUT_SIZES_DATA"
    fi
}

# Print the final encoding summary
print_final_summary() {
    local total_start_time="$1"
    local total_end_time
    total_end_time=$(date +%s)
    local total_elapsed_time=$((total_end_time - total_start_time))

    print_header "Final Encoding Summary"
    
    # Print individual file summaries
    for i in "${!encoded_files[@]}"; do
        local filename="${encoded_files[$i]}"
        local input_size="${input_sizes[$i]}"
        local output_size="${output_sizes[$i]}"
        local encoding_time="${encoding_times[$i]}"

        print_separator
        echo "File: $filename"
        echo "Input size:  $(format_size "$input_size")"
        echo "Output size: $(format_size "$output_size")"
        local reduction
        reduction=$(calculate_reduction "$input_size" "$output_size")
        echo "Reduction:   ${reduction}%"
        echo "Encode time: $(format_time "$encoding_time")"
    done
    
    print_separator
    
    # Calculate totals
    local total_input_size=0
    local total_output_size=0
    for i in "${!input_sizes[@]}"; do
        total_input_size=$((total_input_size + input_sizes[i]))
        total_output_size=$((total_output_size + output_sizes[i]))
    done

    echo "Total files processed: ${#encoded_files[@]}"
    echo "Total input size:  $(format_size "$total_input_size")"
    echo "Total output size: $(format_size "$total_output_size")"
    local total_reduction
    total_reduction=$(calculate_reduction "$total_input_size" "$total_output_size")
    echo "Total reduction:   ${total_reduction}%"
    echo "Total execution time: $(format_time "$total_elapsed_time")"
}

# Format file size in human readable format
format_size() {
    local size="$1"
    numfmt --to=iec-i --suffix=B "$size"
}

# Format time in HH:MM:SS format
format_time() {
    local seconds="$1"
    local hours=$((seconds / 3600))
    local minutes=$(( (seconds % 3600) / 60 ))
    local seconds=$((seconds % 60))
    printf "%02dh %02dm %02ds" "$hours" "$minutes" "$seconds"
}

# Calculate reduction percentage
calculate_reduction() {
    local input_size="$1"
    local output_size="$2"
    awk "BEGIN {printf \"%.2f\", (($input_size - $output_size) / $input_size) * 100}"
}

# Load the appropriate encoding strategy based on content type
load_encoding_strategy() {
    if [[ "$IS_DOLBY_VISION" == true ]]; then
        source "${SCRIPT_DIR}/encode_strategies/dolby_vision.sh"
    else
        source "${SCRIPT_DIR}/encode_strategies/chunked_encoding.sh"
    fi
}

# Initialize array storage
TEMP_DATA_DIR="${TEMP_DIR}/encode_data"
mkdir -p "${TEMP_DATA_DIR}"

# Files to store array data
ENCODED_FILES_DATA="${TEMP_DATA_DIR}/encoded_files.txt"
ENCODING_TIMES_DATA="${TEMP_DATA_DIR}/encoding_times.txt"
INPUT_SIZES_DATA="${TEMP_DATA_DIR}/input_sizes.txt"
OUTPUT_SIZES_DATA="${TEMP_DATA_DIR}/output_sizes.txt"

# Main processing function
main() {
    if ! check_dependencies; then
        exit 1
    fi

    initialize_directories

    # Check for local ffmpeg/ffprobe
    if [[ -f "$HOME/ffmpeg/ffmpeg" ]] && [[ -f "$HOME/ffmpeg/ffprobe" ]]; then
        echo "Using local ffmpeg/ffprobe from $HOME/ffmpeg/"
    else
        echo "Local ffmpeg/ffprobe not found in $HOME/ffmpeg/"
        echo "Using system ffmpeg: $FFMPEG"
        echo "Using system ffprobe: $FFPROBE"
    fi

    # Start total timing
    local total_start_time
    total_start_time=$(date +%s)

    # Load existing array data
    load_arrays

    # Get input files
    local files=()
    if [[ -n "${INPUT_FILE}" ]]; then
        # Single file mode
        if [[ -f "${INPUT_DIR}/${INPUT_FILE}" ]]; then
            files=("${INPUT_DIR}/${INPUT_FILE}")
        else
            print_error "Input file not found: ${INPUT_DIR}/${INPUT_FILE}"
            exit 1
        fi
    else
        # Directory mode - find all video files
        while IFS= read -r -d '' file; do
            files+=("$file")
        done < <(find "${INPUT_DIR}" -type f \( -iname "*.mkv" -o -iname "*.mp4" \) ! -name "._*" -print0)
    fi

    if [ ${#files[@]} -eq 0 ]; then
        print_error "No video files found in ${INPUT_DIR}"
        exit 1
    fi

    # Process each file
    for input_file in "${files[@]}"; do
        process_single_file "$input_file"
        # Save arrays after each file is processed
        save_arrays
    done

    # Print the final summary only if this is the last file
    if [[ "${PRINT_FINAL_SUMMARY}" == "1" ]]; then
        print_final_summary "$total_start_time"
    fi
    
    # Clean up
    cleanup_temp_files
}

# Process a single video file
process_video() {
    local input_file="$1"
    local output_file="$2"
    
    echo "Processing: $(basename "$input_file")"
    
    # Select encoding strategy based on input file
    load_encoding_strategy
    
    # Check if strategy can handle this input
    if ! can_handle "$input_file"; then
        error "Selected strategy cannot handle this input file"
        return 1
    fi
    
    # Initialize encoding
    if ! initialize_encoding "$input_file" "$output_file"; then
        error "Failed to initialize encoding"
        return 1
    fi
    
    # Prepare video
    if ! prepare_video "$input_file" "$output_file"; then
        error "Failed to prepare video"
        return 1
    fi
    
    # Encode video
    if ! encode_video "$input_file" "$output_file"; then
        error "Failed to encode video"
        return 1
    fi
    
    # Finalize encoding
    if ! finalize_encoding "$input_file" "$output_file"; then
        error "Failed to finalize encoding"
        return 1
    fi
    
    print_success "Video processing completed successfully"
    return 0
}