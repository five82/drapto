"""High-level pipeline orchestration for video encoding

Responsibilities:
  - Parse command-line arguments and configure logging.
  - Orchestrate the processing of individual files or directories.
  - Trigger the various encoding stages (segmentation, encoding, muxing).
  - Aggregate and present a final summary of the encoding process.
"""

import logging
import sys
import time
from pathlib import Path
from typing import Optional

from .config import LOG_DIR
from .exceptions import (
    DraptoError, EncodingError, ValidationError,
    ConcatenationError, SegmentEncodingError
)
from .validation import validate_output

logger = logging.getLogger(__name__)
from .formatting import (
    print_header, print_check, print_warning,
    print_error, print_success, print_separator,
    print_info
)
from .video.detection import detect_dolby_vision
from .video.standard_encoder import encode_standard  # for standard encoding
from .audio.encoding import encode_audio_tracks
from .muxer import mux_tracks
from .utils import get_timestamp, format_size, get_file_size

logger = logging.getLogger(__name__)

def _setup_encode_logging(input_file: Path) -> tuple[logging.FileHandler, Path]:
    """Setup logging for an encode session."""
    timestamp = get_timestamp()
    log_file = LOG_DIR / f"{input_file.stem}_{timestamp}.log"
    
    file_handler = logging.FileHandler(log_file)
    file_handler.setFormatter(logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s'))
    logging.root.addHandler(file_handler)
    
    return file_handler, log_file

def _run_encode_pipeline(input_file: Path, output_file: Path) -> None:
    """Run the main encoding pipeline stages."""
    # Override crop detection if disabled via command line
    args = sys.argv
    disable_crop = "--disable-crop" in args
    
    print_check("Checking for Dolby Vision...")
    is_dolby_vision = detect_dolby_vision(input_file)
    if is_dolby_vision:
        print_success("Dolby Vision detected")
    else:
        print_check("Standard content detected")

    # Replace boolean checks with exception handling
    video_track = encode_standard(input_file, disable_crop, dv_flag=is_dolby_vision)
    
    # Process audio - will raise on error
    audio_tracks = encode_audio_tracks(input_file)
    
    # Mux everything together - raises on error
    mux_tracks(video_track, audio_tracks, output_file)
    
    # Validate output - raises ValidationError
    validate_output(input_file, output_file)

def _build_encode_summary(input_file: Path, output_file: Path, start_time: float) -> dict:
    """Build the encoding summary dictionary."""
    input_size = get_file_size(input_file)
    output_size = get_file_size(output_file)
    reduction = ((input_size - output_size) / input_size) * 100
    
    end_time = time.time()
    elapsed = end_time - start_time
    hours = int(elapsed // 3600)
    minutes = int((elapsed % 3600) // 60)
    seconds = int(elapsed % 60)
    finished_time = time.strftime("%a %b %d %H:%M:%S %Z %Y", time.localtime(end_time))
    
    print_header("Encoding Summary")
    print_success(f"Input size:  {format_size(input_size)}")
    print_success(f"Output size: {format_size(output_size)}")
    print_success(f"Reduction:   {reduction:.2f}%")
    print_check(f"Completed: {input_file.name}")
    print_check(f"Encoding time: {hours:02d}h {minutes:02d}m {seconds:02d}s")
    print_check(f"Finished encode at {finished_time}")
    print_separator()
    
    return {
        "output_file": output_file,
        "filename": input_file.name,
        "input_size": input_size,
        "output_size": output_size,
        "reduction": reduction,
        "encoding_time": elapsed
    }

def process_file(input_file: Path, output_file: Path) -> Optional[dict]:
    """
    Process a single input file through the encoding pipeline
    
    Args:
        input_file: Path to input video file
        output_file: Path to output file
        
    Returns:
        Optional[dict]: Dictionary containing encoding summary if successful
    """
    file_handler, log_file = _setup_encode_logging(input_file)
    
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
            _run_encode_pipeline(input_file, output_file)
            
            # Clean up temporary working directories
            from .utils import cleanup_working_dirs
            cleanup_working_dirs()
            
            return _build_encode_summary(input_file, output_file, start_time)
            
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

def process_directory(input_dir: Path, output_dir: Path) -> bool:
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
        logger.error("No video files found in %s", input_dir)
        return False
        
    success = True
    summaries = []
    dir_start_time = time.time()
    for input_file in video_files:
        out_file = output_dir / input_file.name
        summary = process_file(input_file, out_file)
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
