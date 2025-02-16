"""High-level pipeline orchestration for video encoding"""

import logging
from pathlib import Path
from typing import Optional

from .config import (
    INPUT_DIR, OUTPUT_DIR, LOG_DIR,
    ENABLE_CHUNKED_ENCODING
)
from .video.detection import detect_dolby_vision
from .video.encoding import encode_dolby_vision, encode_standard
from .audio.encoding import encode_audio_tracks
from .muxer import mux_tracks
from .utils import get_timestamp, format_size, get_file_size

log = logging.getLogger(__name__)

def process_file(input_file: Path, output_file: Path) -> Optional[Path]:
    """
    Process a single input file through the encoding pipeline
    
    Args:
        input_file: Path to input video file
        output_file: Path to output file
        
    Returns:
        Optional[Path]: Path to output file if successful
    """
    timestamp = get_timestamp()
    log_file = LOG_DIR / f"{input_file.stem}_{timestamp}.log"

    log.info("Starting encode for: %s", input_file.name)
    log.info("Output file: %s", output_file)
    
    # Ensure output directory exists
    output_file.parent.mkdir(parents=True, exist_ok=True)
    
    # Detect Dolby Vision
    is_dolby_vision = detect_dolby_vision(input_file)
    
    try:
        if is_dolby_vision:
            log.info("Processing Dolby Vision content")
            video_track = encode_dolby_vision(input_file)
        elif ENABLE_CHUNKED_ENCODING:
            log.info("Using chunked encoding process")
            video_track = encode_standard(input_file)
        else:
            log.info("Using standard encoding process")
            video_track = encode_dolby_vision(input_file)  # Use same path as DV
            
        if not video_track:
            log.error("Video encoding failed")
            return None
            
        # Process audio
        audio_tracks = encode_audio_tracks(input_file)
        if not audio_tracks:
            log.error("Audio encoding failed")
            return None
            
        # Mux everything together
        if not mux_tracks(video_track, audio_tracks, output_file):
            log.error("Muxing failed")
            return None
            
        # Log completion info
        input_size = get_file_size(input_file)
        output_size = get_file_size(output_file)
        reduction = ((input_size - output_size) / input_size) * 100
        
        log.info("Encoding complete:")
        log.info("Input size:  %s", format_size(input_size))
        log.info("Output size: %s", format_size(output_size))
        log.info("Reduction:   %.2f%%", reduction)
        
        # Clean up temporary working directories and files in /tmp after successful encode
        from .utils import cleanup_working_dirs
        cleanup_working_dirs()
        
        return output_file
        
    except Exception as e:
        log.exception("Error processing %s: %s", input_file.name, e)
        return None

def process_directory(input_dir: Path = INPUT_DIR) -> bool:
    """
    Process all video files in input directory
    
    Args:
        input_dir: Directory containing input video files
        
    Returns:
        bool: True if all files processed successfully
    """
    video_files = list(input_dir.glob("*.mkv"))
    video_files.extend(input_dir.glob("*.mp4"))
    
    if not video_files:
        log.error("No video files found in %s", input_dir)
        return False
        
    success = True
    for input_file in video_files:
        if not process_file(input_file):
            success = False
            
    return success
