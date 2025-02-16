#!/usr/bin/env bash

###################
# Encoding Functions
###################

source "${SCRIPT_DIR}/encode_formatting.sh"

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

    # Process based on content type and chunked encoding setting
    if [[ "$IS_DOLBY_VISION" == true ]]; then
        print_check "Processing Dolby Vision content..."
    fi

    if [[ "$IS_DOLBY_VISION" == false ]] && [[ "$ENABLE_CHUNKED_ENCODING" == true ]]; then
        # Detect crop values once for the entire video
        local crop_filter
        crop_filter=$(detect_crop "$input_file" "$DISABLE_CROP")

        # Create temporary directories
        cleanup_temp_files

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
    print_encoding_summary "$filename" "$input_size" "$output_size"
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

# Process a single audio track
process_audio_track() {
    local input_file="$1"
    local track_index="$2"
    local output_file="$3"

    print_check "Processing audio track ${track_index}..."

    # Get number of channels for this track
    local num_channels
    num_channels=$("${FFPROBE}" -v error -select_streams "a:${track_index}" \
        -show_entries stream=channels -of csv=p=0 "$input_file")
    
    print_check "Found ${num_channels} audio channels"

    # Determine bitrate based on channel count
    local bitrate
    local layout
    case $num_channels in
        1)  bitrate=64; layout="mono" ;;
        2)  bitrate=128; layout="stereo" ;;
        6)  bitrate=256; layout="5.1" ;;
        8)  bitrate=384; layout="7.1" ;;
        *)  bitrate=$((num_channels * 48)); layout="custom" ;;
    esac

    print_check "Configured audio stream ${track_index}: ${num_channels} channels, ${layout} layout, ${bitrate}k bitrate"
    print_check "Using codec: libopus (VBR mode, compression level 10)"

    # Encode audio track
    if ! "${FFMPEG}" -hide_banner -loglevel warning \
        -i "$input_file" \
        -map "a:${track_index}" \
        -c:a libopus \
        -af "aformat=channel_layouts=7.1|5.1|stereo|mono" \
        -application audio \
        -vbr on \
        -compression_level 10 \
        -frame_duration 20 \
        -b:a "${bitrate}k" \
        -avoid_negative_ts make_zero \
        -y "$output_file"; then
        error "Failed to encode audio track ${track_index}"
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

# Handle processing of a single input file
process_single_file() {
    local input_file="$1"
    local filename
    filename=$(basename "${input_file}")
    local filename_noext="${filename%.*}"
    local output_file="${OUTPUT_DIR}/${filename}"
    local timestamp
    timestamp=$(get_timestamp)
    local log_file="${LOG_DIR}/${filename_noext}_${timestamp}.log"

    local file_start_time
    file_start_time=$(date +%s)

    # Process the file
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
    local file_start_time="$2"
    local input_file="$3"
    local output_file="$4"

    local file_end_time
    file_end_time=$(date +%s)
    local file_elapsed_time=$((file_end_time - file_start_time))

    # Store encoding information
    encoded_files+=("$filename")
    encoding_times+=("$file_elapsed_time")
    input_sizes+=("$(get_file_size "${input_file}")")
    output_sizes+=("$(get_file_size "${output_file}")")

    print_completion_message "$filename" "$file_elapsed_time"
}

# Print completion message after encoding a file
print_completion_message() {
    local filename="$1"
    local elapsed_time="$2"

    local hours minutes seconds
    hours=$((elapsed_time / 3600))
    minutes=$(( (elapsed_time % 3600) / 60 ))
    seconds=$((elapsed_time % 60))

    print_separator
    echo "Completed: ${filename}"
    printf "Encoding time: %02dh %02dm %02ds\n" "$hours" "$minutes" "$seconds"
    echo "Finished encode at $(date)"
    print_separator
}

# Print the final summary after processing all files
print_final_summary() {
    local total_start_time="$1"
    local total_end_time
    total_end_time=$(date +%s)
    local total_elapsed_time=$((total_end_time - total_start_time))

    local total_hours total_minutes total_seconds
    total_hours=$((total_elapsed_time / 3600))
    total_minutes=$(( (total_elapsed_time % 3600) / 60 ))
    total_seconds=$((total_elapsed_time % 60))

    print_header "Final Encoding Summary"
    
    # Print individual file summaries
    for i in "${!encoded_files[@]}"; do
        local filename="${encoded_files[$i]}"
        local input_size="${input_sizes[$i]}"
        local output_size="${output_sizes[$i]}"
        local encoding_time="${encoding_times[$i]}"
        local encode_seconds=$((encoding_time))
        local encode_hours=$((encode_seconds / 3600))
        local encode_minutes=$(( (encode_seconds % 3600) / 60 ))
        local encode_seconds=$((encode_seconds % 60))

        print_separator
        echo "File: $filename"
        echo "Input size:  $(numfmt --to=iec-i --suffix=B "$input_size")"
        echo "Output size: $(numfmt --to=iec-i --suffix=B "$output_size")"
        local reduction
        reduction=$(awk "BEGIN {printf \"%.2f\", (($input_size - $output_size) / $input_size) * 100}")
        echo "Reduction:   ${reduction}%"
        printf "Encode time: %02dh %02dm %02ds\n" "$encode_hours" "$encode_minutes" "$encode_seconds"
    done
    
    print_separator
    printf "Total execution time: %02dh %02dm %02ds\n" "$total_hours" "$total_minutes" "$total_seconds"
}

# Main processing function
main() {
    if ! check_dependencies; then
        exit 1
    fi

    initialize_directories

    # Start total timing
    local total_start_time
    total_start_time=$(date +%s)

    # Get input files
    local files
    IFS=$'\n' read -r -d '' -a files < <(find "${INPUT_DIR}" -type f \( -iname "*.mkv" -o -iname "*.mp4" \) ! -name "._*" -print0 | xargs -0 -n1 echo)

    if [ ${#files[@]} -eq 0 ]; then
        echo "No video files found in ${INPUT_DIR}"
        exit 1
    fi

    # Process each file
    for input_file in "${files[@]}"; do
        process_single_file "$input_file"
    done

    print_final_summary "$total_start_time"
}

# Process a single video file
process_video() {
    local input_file="$1"
    local output_file="$2"
    
    echo "Processing: $(basename "$input_file")"
    
    # Detect Dolby Vision
    detect_dolby_vision "$input_file"
    
    # Check for hardware acceleration only on macOS
    if [[ "$OSTYPE" == "darwin"* ]]; then
        echo "macOS detected, checking hardware acceleration."
        check_hardware_acceleration
        HWACCEL_OPTS=$(configure_hw_accel_options)
    else
        HWACCEL_OPTS=""
    fi
    
    # Configure audio options
    setup_audio_options "$input_file"
    
    # Configure subtitle options
    setup_subtitle_options "$input_file"
    
    # Configure video options
    VIDEO_OPTS=$(setup_video_options "$input_file")
    
    # Run the encoding
    run_encoding "$input_file" "$output_file"
} 