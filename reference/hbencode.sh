#!/usr/bin/env bash

# Encodes videos with HandBrakeCLI

# Required:
# HandBrakeCLI
# ffmpeg

# Variables
INPUT_DIR="${HOME}/Videos/input"
OUTPUT_DIR="${HOME}/Videos/output"
LOG_DIR="${HOME}/Videos/logs"

# Arrays to store encoding information
declare -a encoded_files
declare -a encoding_times
declare -a input_sizes
declare -a output_sizes

# Function to get file size portably
get_file_size() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        stat -f%z "$1"
    else
        stat -c%s "$1"
    fi
}

# Function to get timestamp
get_timestamp() {
    date "+%Y%m%d_%H%M%S"
}

# Create directories if they don't exist
mkdir -p "${INPUT_DIR}"
mkdir -p "${OUTPUT_DIR}"
mkdir -p "${LOG_DIR}"

# Start total timing
total_start_time=$(date +%s)

# Store files in an array
mapfile -d $'\0' files < <(find "${INPUT_DIR}" -type f -iname "*.mkv" -print0)

# Process each file
for input_file in "${files[@]}"; do
    # Get the filename without path and extension
    filename=$(basename "${input_file}")
    filename_noext="${filename%.*}"
    output_file="${OUTPUT_DIR}/${filename}"
    timestamp=$(get_timestamp)
    log_file="${LOG_DIR}/${filename_noext}_${timestamp}.log"
    
    # Start timing for this file
    file_start_time=$(date +%s)
    
    # Start logging
    {
        echo "Starting encode at $(date)"
        echo "Input file: ${input_file}"
        echo "Output file: ${output_file}"
        echo "----------------------------------------"
        
        echo "Processing: ${filename}"

        # Get audio channel information for all audio streams
        mapfile -t audio_channels < <(ffprobe -v error -select_streams a -show_entries stream=channels -of csv=p=0 "${input_file}")

        echo "Detected audio channels: ${audio_channels[@]}"

        # Build the audio bitrate options
        audio_bitrate_opts=""
        stream_index=0
        for num_channels in "${audio_channels[@]}"; do
            case $num_channels in
                1)  # Mono
                    bitrate=64
                    ;;
                2)  # Stereo
                    bitrate=128
                    ;;
                6)  # 5.1
                    bitrate=256
                    ;;
                8)  # 7.1
                    bitrate=384
                    ;;
                *)  # Default fallback
                    bitrate=$((num_channels * 48))
                    ;;
            esac
            
            # Append bitrate option for each audio track
            audio_bitrate_opts+=" --ab ${bitrate}"
            echo "Added bitrate for audio stream $stream_index ($num_channels channels): ${bitrate}kbps"
            ((stream_index++))
        done

        echo "Final audio bitrate options: ${audio_bitrate_opts}"
        echo "----------------------------------------"
        echo "Starting HandBrakeCLI encode..."

        HandBrakeCLI \
          --encoder svt_av1_10bit \
          --encoder-preset 6 \
          --encoder-tune 0 \
          --quality 28 \
	  --encopts film-grain=8:film-grain-denoise=1 \
	  --auto-anamorphic \
          --all-subtitles \
          --aencoder opus \
          --all-audio \
          --mixdown none \
	  --enable-hw-decoding \
	  --no-comb-detect \
          --no-deinterlace \
          --no-bwdif \
          --no-decomb \
          --no-detelecine \
          --no-hqdn3d \
          --no-nlmeans \
          --no-chroma-smooth \
          --no-unsharp \
          --no-lapsharp \
          --no-deblock \
          ${audio_bitrate_opts} \
          -i "${input_file}" \
          -o "${output_file}"
          
    } 2>&1 | tee "${log_file}"  # Capture all output to log file and display on screen
    
    # Check if HandBrakeCLI was successful
    if [ ${PIPESTATUS[0]} -eq 0 ]; then
        file_end_time=$(date +%s)
        file_elapsed_time=$((file_end_time - file_start_time))
        hours=$((file_elapsed_time / 3600))
        minutes=$(( (file_elapsed_time % 3600) / 60 ))
        seconds=$((file_elapsed_time % 60))
        
        # Store filename, encoding time, and file sizes
        encoded_files+=("$filename")
        encoding_times+=("$file_elapsed_time")
        input_sizes+=("$(get_file_size "${input_file}")")
        output_sizes+=("$(get_file_size "${output_file}")")
        
        echo "----------------------------------------"
        echo "Completed: ${filename}"
        echo "Encoding time: ${hours}h ${minutes}m ${seconds}s"
        echo "Finished encode at $(date)"
    else
        echo "Error encoding ${filename}"
    fi
    
    echo "----------------------------------------"
done

# Check if any files were processed
if [ ${#encoded_files[@]} -eq 0 ]; then
    echo "No MKV files found in ${INPUT_DIR}"
    exit 1
fi

# Calculate and display total elapsed time
total_end_time=$(date +%s)
total_elapsed_time=$((total_end_time - total_start_time))
total_hours=$((total_elapsed_time / 3600))
total_minutes=$(( (total_elapsed_time % 3600) / 60 ))
total_seconds=$((total_elapsed_time % 60))

echo "All files processed successfully!"
echo "----------------------------------------"
echo "Encoding Summary:"
echo "----------------------------------------"
for i in "${!encoded_files[@]}"; do
    time_seconds=${encoding_times[$i]}
    h=$((time_seconds / 3600))
    m=$(( (time_seconds % 3600) / 60 ))
    s=$((time_seconds % 60))
    
    # Convert sizes to human readable format
    input_hr=$(numfmt --to=iec-i --suffix=B "${input_sizes[$i]}")
    output_hr=$(numfmt --to=iec-i --suffix=B "${output_sizes[$i]}")
    
    # Calculate reduction percentage
    reduction=$(( 100 - (output_sizes[$i] * 100 / input_sizes[$i]) ))
    
    echo "${encoded_files[$i]}"
    echo "  Encode time: ${h}h ${m}m ${s}s"
    echo "  Input size:  ${input_hr}"
    echo "  Output size: ${output_hr}"
    echo "  Reduced by:  ${reduction}%"
    echo "----------------------------------------"
done
echo "Total execution time: ${total_hours}h ${total_minutes}m ${total_seconds}s"

