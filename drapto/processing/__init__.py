"""File processing orchestration

Responsibilities:
- Process individual video files through the encoding pipeline
- Handle directory-level batch processing
- Generate and format encoding summaries
"""

import logging
import time
from pathlib import Path
from typing import Optional, Dict

from ..config import LOG_DIR
from ..exceptions import (
    DraptoError, EncodingError, ValidationError,
    ConcatenationError, SegmentEncodingError
)
from ..validation import validate_output
from .formatting import (
    print_header, print_check, print_warning,
    print_error, print_success, print_separator,
    print_info
)
from .video.detection import detect_dolby_vision
from .video.standard_encoder import encode_standard
from .audio.encoding import encode_audio_tracks
from .muxer import mux_tracks
from .utils import get_timestamp, format_size, get_file_size

logger = logging.getLogger(__name__)

from .processing.summary import setup_encode_logging, build_encode_summary

def _run_encode_pipeline(input_file: Path, output_file: Path, disable_crop: bool = False) -> None:
    """Run the main encoding pipeline stages."""
    print_check("Checking for Dolby Vision...")
    is_dolby_vision = detect_dolby_vision(input_file)
    if is_dolby_vision:
        print_success("Dolby Vision detected")
    else:
        print_check("Standard content detected")

    video_track = encode_standard(input_file, disable_crop, dv_flag=is_dolby_vision)
    audio_tracks = encode_audio_tracks(input_file)
    mux_tracks(video_track, audio_tracks, output_file)
    validate_output(input_file, output_file)


def process_file(input_file: Path, output_file: Path, disable_crop: bool = False) -> Optional[Dict]:
    """
    Process a single input file through the encoding pipeline
    
    Args:
        input_file: Path to input video file
        output_file: Path to output file
        disable_crop: Whether to disable crop detection
        
    Returns:
        Optional[dict]: Dictionary containing encoding summary if successful
    """
    file_handler, log_file = setup_encode_logging(input_file)
    
    try:
        start_time = time.time()
        print_header("Starting Encode")
        logger.info("Beginning encode of: %s", input_file.name)
        logger.info("Encode log: %s", log_file.name)
        print_check(f"Input path:  {input_file.resolve()}")
        print_check(f"Output path: {output_file.resolve()}")
        print_separator()

        # Ensure output directory exists
        output_file.parent.mkdir(parents=True, exist_ok=True)
        
        try:
            _run_encode_pipeline(input_file, output_file, disable_crop)
            
            # Clean up temporary working directories
            from .utils import cleanup_working_dirs
            cleanup_working_dirs()
            
            return build_encode_summary(input_file, output_file, start_time)
            
        except (EncodingError, ValidationError) as e:
            logger.error("Encoding failed: %s", e)
            raise DraptoError("Encoding aborted") from e
        except Exception as e:
            logger.exception("Error processing %s: %s", input_file.name, e)
            return None
    finally:
        logging.root.removeHandler(file_handler)
        file_handler.close()
        logger.info("Closed encode log: %s", log_file.name)

def process_directory(input_dir: Path, output_dir: Path, disable_crop: bool = False) -> bool:
    """
    Process all video files in input directory
    
    Args:
        input_dir: Directory containing input video files
        output_dir: Output directory for encoded files
        disable_crop: Whether to disable crop detection
        
    Returns:
        bool: True if all files processed successfully
    """
    video_files = list(input_dir.glob("*.mkv"))
    video_files.extend(input_dir.glob("*.mp4"))
    
    if not video_files:
        logger.error("No video files found in %s", input_dir)
        return False
        
    success = True
    summaries = []
    dir_start_time = time.time()
    for input_file in video_files:
        out_file = output_dir / input_file.name
        summary = process_file(input_file, out_file, disable_crop)
        if summary:
            summaries.append(summary)
        else:
            success = False

    # Final overall summary after processing all files
    total_elapsed = time.time() - dir_start_time
    total_hours = int(total_elapsed // 3600)
    total_minutes = int((total_elapsed % 3600) // 60)
    total_seconds = int(total_elapsed % 60)

    print_header("Final Encoding Summary")
    for s in summaries:
        print_separator()
        print_check(f"File: {s['filename']}")
        print_success(f"Input size:  {format_size(s['input_size'])}")
        print_success(f"Output size: {format_size(s['output_size'])}")
        print_success(f"Reduction:   {s['reduction']:.2f}%")
        enc_time = s['encoding_time']
        h = int(enc_time // 3600)
        m = int((enc_time % 3600) // 60)
        sec = int(enc_time % 60)
        print_check(f"Encode time: {h:02d}h {m:02d}m {sec:02d}s")
    print_separator()
    print_success(f"Total execution time: {total_hours:02d}h {total_minutes:02d}m {total_seconds:02d}s")
    
    return success
