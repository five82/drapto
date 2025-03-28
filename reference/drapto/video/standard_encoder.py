"""Standard encoding implementation for non-Dolby Vision content.

This module implements the standard encoding pipeline used for standard content,
performing segmentation, parallel encoding, and concatenation.
"""

import logging
import shutil
from pathlib import Path
from typing import Optional

logger = logging.getLogger(__name__)

from ..config import WORKING_DIR
from ..utils import run_cmd
from ..formatting import print_check
from .detection import detect_crop
from .segmentation.segmentation_main import segment_video
from .segment_encoding import encode_segments
from .concatenation import concatenate_segments
from ..exceptions import (
    EncodingError, SegmentationError,
    SegmentEncodingError, ConcatenationError
)

def encode_standard(input_file: Path, disable_crop: bool = False, dv_flag: bool = False) -> Path:
    """
    Encode standard (non-Dolby Vision) content using the standard encoding pipeline.
    
    Args:
        input_file: Path to input video file
        
    Returns:
        Path: Path to encoded video file
        
    Raises:
        EncodingError: If any stage of encoding fails
    """
    # Ensure the working directory exists
    WORKING_DIR.mkdir(parents=True, exist_ok=True)
    output_file = WORKING_DIR / "video.mkv"

    # Remove any pre-existing output file
    if output_file.exists():
        output_file.unlink()
    
    try:
        # Detect crop values and HDR status; disable if requested
        crop_filter, is_hdr = detect_crop(input_file, disable_crop)
        
        # Step 1: Segment video 
        print_check("Segmenting video...")
        segment_video(input_file)  # Now raises exceptions directly
        print_check("Successfully segmented video")
            
        # Step 2: Encode segments
        print_check("Encoding segments in parallel...")
        encode_segments(crop_filter, is_hdr, dv_flag)  # Pass HDR status through
            
        # Step 3: Concatenate segments
        print_check("Concatenating segments...")
        concatenate_segments(output_file)  # Now raises exceptions directly
        print_check("Segments concatenated successfully")
            
        return output_file
        
    except Exception as e:
        raise EncodingError(f"Standard encoding failed: {str(e)}", module="standard_encoder") from e
    finally:
        print_check("Cleaning up temporary files...")
        for temp_dir in ["segments", "encoded_segments"]:
            temp_path = WORKING_DIR / temp_dir
            if temp_path.exists():
                shutil.rmtree(temp_path)
