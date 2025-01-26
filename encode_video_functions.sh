#!/usr/bin/env bash

###################
# Video Functions
###################

source "${SCRIPT_DIR}/encode_formatting.sh"

# Detect if the input file contains Dolby Vision
detect_dolby_vision() {
    print_check "Checking for Dolby Vision..."
    local file="$1"

    local is_dv
    is_dv=$(mediainfo "$file" | grep "Dolby Vision" || true)

    if [[ -n "$is_dv" ]]; then
        print_check "Dolby Vision detected"
        IS_DOLBY_VISION=true
        return 0
    else
        print_check "Dolby Vision not detected"
        IS_DOLBY_VISION=false
        return 1
    fi
}

# Detect black bars and return crop values
detect_crop() {
    local input_file="$1"
    local disable_crop="$2"

    if [[ "$disable_crop" == "true" ]]; then
        print_check "Crop detection disabled"
        return 0
    fi

    print_check "Analyzing video for black bars..."

    # Check if input is HDR and get color properties
    local color_transfer color_primaries color_space
    color_transfer=$("${FFPROBE}" -v error -select_streams v:0 -show_entries stream=color_transfer -of default=noprint_wrappers=1:nokey=1 "$input_file")
    color_primaries=$("${FFPROBE}" -v error -select_streams v:0 -show_entries stream=color_primaries -of default=noprint_wrappers=1:nokey=1 "$input_file")
    color_space=$("${FFPROBE}" -v error -select_streams v:0 -show_entries stream=color_space -of default=noprint_wrappers=1:nokey=1 "$input_file")

    # Set initial crop threshold
    local crop_threshold=16
    local is_hdr=false

    # Check for various HDR formats
    if [[ "$color_transfer" =~ ^(smpte2084|arib-std-b67|smpte428|bt2020-10|bt2020-12)$ ]] || \
       [[ "$color_primaries" =~ ^(bt2020)$ ]] || \
       [[ "$color_space" =~ ^(bt2020nc|bt2020c)$ ]]; then
        is_hdr=true
        crop_threshold=128
        print_check "HDR content detected, adjusting detection sensitivity"
    fi

    # Get maximum pixel value to help determine black level
    if [[ "$is_hdr" == "true" ]]; then
        # Sample a few frames to find the typical black level
        local black_level
        black_level=$("${FFMPEG}" -hide_banner -i "${input_file}" \
            -vf "select='eq(n,0)+eq(n,100)+eq(n,200)',blackdetect=d=0:pic_th=0.1" \
            -f null - 2>&1 | \
            grep "black_level" | \
            awk -F: '{sum += $2; count++} END {if(count>0) print int(sum/count); else print 128}')

        # Adjust threshold based on measured black level
        crop_threshold=$((black_level * 3 / 2))  # Multiply by 1.5 using integer arithmetic
    fi

    # Ensure threshold is within reasonable bounds
    if [ "$crop_threshold" -lt 16 ]; then
        crop_threshold=16
    elif [ "$crop_threshold" -gt 256 ]; then
        crop_threshold=256
    fi

    # Sample the video at different intervals
    local duration
    duration=$("${FFPROBE}" -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 "$input_file")
    duration=$(printf "%.0f" "$duration")
    
    # Skip credits based on content length
    local credits_skip=0
    if [ "$duration" -gt 3600 ]; then
        credits_skip=180  # Movies (>1 hour): Skip 3 minutes
    elif [ "$duration" -gt 1200 ]; then
        credits_skip=60   # Long content (>20 minutes): Skip 1 minute
    elif [ "$duration" -gt 300 ]; then
        credits_skip=30   # Medium content (>5 minutes): Skip 30 seconds
    fi

    if [ "$credits_skip" -gt 0 ]; then
        if [ "$duration" -gt "$credits_skip" ]; then
            duration=$((duration - credits_skip))
        fi
    fi
    
    local interval=5  # Check every 5 seconds
    local total_samples=$((duration / interval))
    
    # Ensure we check at least 20 samples
    if [ "$total_samples" -lt 20 ]; then
        interval=$((duration / 20))
        [ "$interval" -lt 1 ] && interval=1
        total_samples=20
    fi

    print_check "Analyzing $(print_stat "${total_samples}") frames for black bars..."

    # Get the original dimensions first
    local original_width original_height
    original_width=$("${FFPROBE}" -v error -select_streams v:0 -show_entries stream=width -of default=noprint_wrappers=1:nokey=1 "${input_file}")
    original_height=$("${FFPROBE}" -v error -select_streams v:0 -show_entries stream=height -of default=noprint_wrappers=1:nokey=1 "${input_file}")

    # Then run crop detection with HDR-aware threshold
    local crop_values
    crop_values=$("${FFMPEG}" -hide_banner -i "${input_file}" \
                 -vf "select='not(mod(n,30))',cropdetect=limit=${crop_threshold}:round=2:reset=1" \
                 -frames:v $((total_samples * 2)) \
                 -f null - 2>&1 | \
                 awk '/crop/ { print $NF }' | \
                 grep "^crop=${original_width}:")  # Only consider crops that maintain original width

    # Analyze all crop heights and their frequencies
    local heights_analysis
    heights_analysis=$(echo "$crop_values" | \
        awk -F':' '{print $2}' | \
        grep -v '^$' | \
        awk -v min=100 '$1 >= min' | \
        sort | uniq -c | sort -nr)

    # Get the most common height
    local most_common_height
    most_common_height=$(echo "$heights_analysis" | head -n1 | awk '{print $2}')

    # Calculate black bar size
    local black_bar_size=$(( (original_height - most_common_height) / 2 ))
    local black_bar_percent=$(( black_bar_size * 100 / original_height ))

    if [ "$black_bar_size" -gt 0 ]; then
        print_check "Found black bars: $(print_stat "${black_bar_size} pixels") ($(print_stat "${black_bar_percent}%") of height)"
    else
        print_check "No significant black bars detected"
    fi

    # Return the crop value if black bars are significant (>1% of height)
    if [ "$black_bar_percent" -gt 1 ]; then
        echo "crop=${original_width}:${most_common_height}:0:${black_bar_size}"
    else
        echo "crop=${original_width}:${original_height}:0:0"
    fi
}

# Get video encoding options based on input file analysis
get_video_encode_options() {
    local input_file="$1"
    local IS_DOLBY_VISION="$2"
    local crop_filter="$3"
    local crf="$4"

    # Base video encoding options
    local video_opts="-c:v libsvtav1 \
        -preset ${PRESET} \
        -crf ${crf} \
        -svtav1-params tune=0:film-grain=0:film-grain-denoise=0"

    if [[ "$IS_DOLBY_VISION" == "true" ]]; then
        video_opts+=" -dolbyvision true"
    fi

    # Add crop filter if provided
    if [[ -n "$crop_filter" ]]; then
        video_opts+=" -vf $crop_filter"
    fi

    echo "$video_opts"
}

# Set up video encoding options based on input file
setup_video_options() {
    local input_file="$1"
    local disable_crop="$2"
    local video_opts=""

    # Get video width
    local width
    width=$("${FFPROBE}" -v error -select_streams v:0 -show_entries stream=width -of default=noprint_wrappers=1:nokey=1 "${input_file}")

    # Set quality based on resolution
    local crf
    if [ "$width" -ge 3840 ]; then
        crf=$CRF_UHD
        print_check "UHD quality detected ($(print_stat "${width}px") width), using CRF $(print_stat "$crf")"
    elif [ "$width" -ge 1920 ]; then
        crf=$CRF_HD
        print_check "HD quality detected ($(print_stat "${width}px") width), using CRF $(print_stat "$crf")"
    else
        crf=$CRF_SD
        print_check "SD quality detected ($(print_stat "${width}px") width), using CRF $(print_stat "$crf")"
    fi

    # Detect crop values
    local crop_filter
    crop_filter=$(detect_crop "${input_file}" "${disable_crop}")
    
    # Get video encoding options
    video_opts=$(get_video_encode_options "${input_file}" "${IS_DOLBY_VISION}" "${crop_filter}" "${crf}")

    printf "%s" "${video_opts}"
}

# Get sizes of the video and audio streams in a file
get_stream_sizes() {
    local file="$1"
    local video_size=0
    local audio_size=0

    # Create temp directory for demuxed streams
    local temp_dir
    temp_dir=$(mktemp -d)

    # Get video codec and extract
    local video_codec
    video_codec=$("${FFPROBE}" -v error -select_streams v:0 -show_entries stream=codec_name -of default=noprint_wrappers=1:nokey=1 "$file")

    case "$video_codec" in
        h264) video_ext="h264" ;;
        hevc) video_ext="hevc" ;;
        av1)  video_ext="obu" ;;
        *)    video_ext="mkv" ;;
    esac
    local video_file="${temp_dir}/video.${video_ext}"
    "${FFMPEG}" -v error -i "$file" -map 0:v:0 -c copy "${video_file}"

    # Get audio codec and extract
    local audio_codec
    audio_codec=$("${FFPROBE}" -v error -select_streams a:0 -show_entries stream=codec_name -of default=noprint_wrappers=1:nokey=1 "$file")

    case "$audio_codec" in
        aac)  audio_ext="aac" ;;
        ac3)  audio_ext="ac3" ;;
        dts)  audio_ext="dts" ;;
        opus) audio_ext="opus" ;;
        *)    audio_ext="mka" ;;
    esac
    local audio_file="${temp_dir}/audio.${audio_ext}"
    "${FFMPEG}" -v error -i "$file" -map 0:a -c copy "${audio_file}"

    # Get sizes if files exist
    if [[ -f "${video_file}" ]]; then
        video_size=$(get_file_size "${video_file}")
    fi

    if [[ -f "${audio_file}" ]]; then
        audio_size=$(get_file_size "${audio_file}")
    fi

    # Cleanup temp files
    rm -rf "${temp_dir}"

    # Return both sizes
    echo "${video_size:-0},${audio_size:-0}"
}

# Print a summary of the encoding process for a single file
print_encoding_summary() {
    local filename="$1"
    local input_size="$2"
    local output_size="$3"
    
    if [[ -z "$input_size" ]] || [[ -z "$output_size" ]]; then
        error "Missing size information for $filename"
        return 1
    fi

    # Calculate size reduction percentage
    local reduction
    reduction=$(awk "BEGIN {printf \"%.2f\", (($input_size - $output_size) / $input_size) * 100}")

    # Format sizes in human-readable format
    local input_hr
    local output_hr
    input_hr=$(numfmt --to=iec-i --suffix=B "$input_size")
    output_hr=$(numfmt --to=iec-i --suffix=B "$output_size")

    print_header "Encoding Summary"
    echo "Input size:  $input_hr"
    echo "Output size: $output_hr"
    echo "Reduction:   ${reduction}%"
}

# Configure hardware acceleration options
configure_hw_accel_options() {
    local hw_options=""
    
    case "${HW_ACCEL}" in
        "videotoolbox")
            hw_options="-hwaccel videotoolbox"
            ;;
        *)
            hw_options=""
            ;;
    esac
    
    echo "${hw_options}"
}

# Validate video segments
validate_segments() {
    print_check "Validating segments..."
    local dir="$1"
    local min_file_size=1024  # 1KB minimum file size

    local segment_count
    segment_count=$(find "$dir" -name "*.mkv" -type f | wc -l)
    [[ $segment_count -lt 1 ]] && error "No segments found in $dir"

    local invalid_segments=0
    while IFS= read -r -d $'\0' segment; do
        local file_size
        file_size=$(stat -f%z "$segment" 2>/dev/null || stat -c%s "$segment")
        
        if [[ $file_size -lt $min_file_size ]] || ! "${FFPROBE}" -v error "$segment" >/dev/null 2>&1; then
            print_warning "Invalid segment found: $segment"
            ((invalid_segments++))
        fi
    done < <(find "$dir" -name "*.mkv" -type f -print0)

    [[ $invalid_segments -gt 0 ]] && error "Found $invalid_segments invalid segments"
    print_success "Successfully validated $segment_count segments"
}

# Segment video into chunks
segment_video() {
    print_check "Segmenting video..."
    local input_file="$1"
    local output_dir="$2"

    mkdir -p "$output_dir"

    "${FFMPEG}" -hide_banner -loglevel error -i "$input_file" \
        -c:v copy \
        -an \
        -f segment \
        -segment_time "$SEGMENT_LENGTH" \
        -reset_timestamps 1 \
        "${output_dir}/%04d.mkv"

    validate_segments "$output_dir"
}

# Encode video segments using ab-av1
encode_segments() {
    local input_dir="$1"
    local output_dir="$2"
    local target_vmaf="$3"
    local disable_crop="$4"
    local crop_filter="$5"

    # Check if GNU Parallel is installed
    if ! check_parallel_installation; then
        error "Cannot proceed with parallel encoding without GNU Parallel"
        return 1
    fi

    # Create output directory if it doesn't exist
    mkdir -p "$output_dir"

    # Create a temporary file to store segment encoding commands
    local cmd_file=$(mktemp)
    
    # Build the command list
    local segment_count=0
    for segment in "$input_dir"/*.mkv; do
        if [[ -f "$segment" ]]; then
            local segment_name=$(basename "$segment")
            local output_segment="${output_dir}/${segment_name}"
            
            # Skip if output segment already exists and has non-zero size
            if [[ -f "$output_segment" ]] && [[ -s "$output_segment" ]]; then
                print_check "Segment already encoded successfully: ${segment_name}"
                continue
            fi

            # Only pass crop filter if it's not empty
            local vfilter_args=""
            if [[ -n "$crop_filter" ]]; then
                vfilter_args="--vfilter $crop_filter"
            fi

            # Build the encoding command
            echo "encode_single_segment '$segment' '$output_segment' '$target_vmaf' '$vfilter_args'" >> "$cmd_file"
            ((segment_count++))
        fi
    done

    if [[ $segment_count -eq 0 ]]; then
        error "No segments found to encode in $input_dir"
        rm "$cmd_file"
        return 1
    fi

    # Define the single segment encoding function
    encode_single_segment() {
        local segment="$1"
        local output_segment="$2"
        local target_vmaf="$3"
        local vfilter_args="$4"
        
        # First attempt with default settings
        if ab-av1 auto-encode \
            --input "$segment" \
            --output "$output_segment" \
            --encoder libsvtav1 \
            --min-vmaf "$target_vmaf" \
            --preset "$PRESET" \
            --svt "tune=0:film-grain=0:film-grain-denoise=0" \
            --keyint 10s \
            --samples "$VMAF_SAMPLE_COUNT" \
            --sample-duration "${VMAF_SAMPLE_LENGTH}s" \
            --vmaf "n_subsample=8:pool=harmonic_mean" \
            $vfilter_args \
            --quiet; then
            return 0
        fi
        
        # Second attempt with more samples
        if ab-av1 auto-encode \
            --input "$segment" \
            --output "$output_segment" \
            --encoder libsvtav1 \
            --min-vmaf "$target_vmaf" \
            --preset "$PRESET" \
            --svt "tune=0:film-grain=0:film-grain-denoise=0" \
            --keyint 10s \
            --samples 6 \
            --sample-duration "2s" \
            --vmaf "n_subsample=8:pool=harmonic_mean" \
            $vfilter_args \
            --quiet; then
            return 0
        fi
        
        # Final attempt with lower VMAF target
        local lower_vmaf=$((target_vmaf - 2))
        if ab-av1 auto-encode \
            --input "$segment" \
            --output "$output_segment" \
            --encoder libsvtav1 \
            --min-vmaf "$lower_vmaf" \
            --preset "$PRESET" \
            --svt "tune=0:film-grain=0:film-grain-denoise=0" \
            --keyint 10s \
            --samples 6 \
            --sample-duration "2s" \
            --vmaf "n_subsample=8:pool=harmonic_mean" \
            $vfilter_args \
            --quiet; then
            return 0
        fi
        
        error "Failed to encode segment after all attempts: $(basename "$segment")"
        return 1
    }
    export -f encode_single_segment
    export -f print_check
    export -f error
    export PRESET
    export VMAF_SAMPLE_COUNT
    export VMAF_SAMPLE_LENGTH

    # Use GNU Parallel to process segments
    # --jobs determines how many parallel jobs to run (0 means number of CPU cores)
    # --halt soon,fail=1 stops all jobs if any job fails
    # --no-notice suppresses citation notice
    # --line-buffer ensures output lines are not mixed
    if ! parallel --no-notice --line-buffer --halt soon,fail=1 --jobs 0 :::: "$cmd_file"; then
        rm "$cmd_file"
        return 1
    fi

    rm "$cmd_file"
    return 0
}

# Concatenate encoded segments back into a single file
concatenate_segments() {
    local output_file="$1"
    local concat_file="${WORKING_DIR}/concat.txt"

    # Create concat file with proper format
    > "$concat_file"
    for segment in "${ENCODED_SEGMENTS_DIR}"/*.mkv; do
        if [[ -f "$segment" ]]; then
            echo "file '$(realpath "$segment")'" >> "$concat_file"
        fi
    done

    if [[ ! -s "$concat_file" ]]; then
        error "No segments found to concatenate"
        return 1
    fi

    # Concatenate video segments directly to output file
    if ! "$FFMPEG" -hide_banner -loglevel error -f concat -safe 0 -i "$concat_file" -c copy "$output_file"; then
        error "Failed to concatenate video segments"
        return 1
    fi

    return 0
}

# Clean up temporary files
cleanup_temp_files() {
    print_check "Cleaning up temporary files..."
    rm -rf "$SEGMENTS_DIR" "$ENCODED_SEGMENTS_DIR" "$WORKING_DIR"
    mkdir -p "$WORKING_DIR"
}