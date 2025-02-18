"""Standard encoding implementation for non-Dolby Vision content.

This module implements the standard encoding pipeline used for standard content,
performing segmentation, parallel encoding, and concatenation.
"""

import logging
import shutil
from pathlib import Path
from typing import Optional

from ..config import WORKING_DIR
from ..utils import run_cmd
from ..formatting import print_check
from .detection import detect_crop
from .segmentation import segment_video
from .segment_encoding import encode_segments
from .concatenation import concatenate_segments

log = logging.getLogger(__name__)

def encode_standard(input_file: Path) -> Optional[Path]:
    """
    Encode standard (non-Dolby Vision) content using the standard encoding pipeline.
    
    Args:
        input_file: Path to input video file
        
    Returns:
        Optional[Path]: Path to encoded video file if successful
    """
    # Ensure the working directory exists
    WORKING_DIR.mkdir(parents=True, exist_ok=True)
    output_file = WORKING_DIR / "video.mkv"

    # Remove any pre-existing output file
    if output_file.exists():
        output_file.unlink()
    
    # Detect crop values
    crop_filter = detect_crop(input_file)
    
    try:
        # Step 1: Segment video
        print_check("Segmenting video...")
        if not segment_video(input_file):
            return None
        print_check("Successfully segmented video")
            
        # Step 2: Encode segments
        print_check("Encoding segments in parallel...")
        if not encode_segments(crop_filter):
            return None
            
        # Step 3: Concatenate segments
        print_check("Concatenating segments...")
        if not concatenate_segments(output_file):
            return None
        print_check("Segments concatenated successfully")
            
        return output_file
        
    except Exception as e:
        log.error("Failed to encode standard content: %s", e)
        return None
    finally:
        print_check("Cleaning up temporary files...")
        for temp_dir in ["segments", "encoded_segments"]:
            temp_path = WORKING_DIR / temp_dir
            if temp_path.exists():
                shutil.rmtree(temp_path)
