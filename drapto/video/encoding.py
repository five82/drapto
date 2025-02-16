"""Video encoding functions for drapto"""

import logging
import shutil
from pathlib import Path
from typing import Optional

from ..config import (
    PRESET, CRF_SD, CRF_HD, CRF_UHD, SVT_PARAMS,
    WORKING_DIR
)
from ..utils import run_cmd
from .hardware import get_hwaccel_options
from .detection import detect_crop
from .segmentation import (
    segment_video,
    encode_segments,
    concatenate_segments
)

log = logging.getLogger(__name__)

def encode_dolby_vision(input_file: Path) -> Optional[Path]:
    """
    Encode Dolby Vision content using ffmpeg with libsvtav1
    
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
    
    # Get video width for CRF selection
    try:
        result = run_cmd([
            "ffprobe", "-v", "error",
            "-select_streams", "v:0",
            "-show_entries", "stream=width",
            "-of", "default=noprint_wrappers=1:nokey=1",
            str(input_file)
        ])
        width = int(result.stdout.strip())
    except Exception as e:
        log.error("Failed to get video width: %s", e)
        return None
        
    # Select CRF based on resolution
    if width >= 3840:
        crf = CRF_UHD
    elif width >= 1920:
        crf = CRF_HD
    else:
        crf = CRF_SD

    # Crop detection for both Dolby Vision and standard encoding
    crop_filter = detect_crop(input_file)
        
    # For Dolby Vision content:
    # 1. Force software decoding
    # 2. Use simpler command structure
    cmd = [
        "ffmpeg", "-hide_banner",
        "-loglevel", "warning",
        "-hwaccel", "none",
        "-i", str(input_file)
    ]

    # If a crop filter was detected, add it to the command
    if crop_filter:
        cmd.extend(["-vf", crop_filter])
    
    # Add encoding options
    cmd.extend([
        "-map", "0:v:0",
        "-c:v", "libsvtav1",
        "-preset", str(PRESET),
        "-crf", str(crf),
        "-svtav1-params", SVT_PARAMS,
        "-pix_fmt", "yuv420p10le",
        "-dolbyvision", "true",
        "-y", str(output_file)
    ])
    
    try:
        run_cmd(cmd)
        return output_file
    except Exception as e:
        log.error("Failed to encode Dolby Vision content: %s", e)
        log.info("Retrying without hardware acceleration")
        try:
            # Retry is not really needed now because we already removed hwaccel options.
            run_cmd(cmd)
            return output_file
        except Exception as e:
            log.error("Software fallback failed: %s", e)
        return None

def encode_standard(input_file: Path) -> Optional[Path]:
    """
    Encode standard content using chunked encoding with ab-av1
    
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
        if not segment_video(input_file):
            return None
            
        # Step 2: Encode segments
        if not encode_segments(crop_filter):
            return None
            
        # Step 3: Concatenate segments
        if not concatenate_segments(output_file):
            return None
            
        return output_file
        
    except Exception as e:
        log.error("Failed to encode standard content: %s", e)
        return None
    finally:
        # Cleanup temporary segment files
        for temp_dir in ["segments", "encoded_segments"]:
            temp_path = WORKING_DIR / temp_dir
            if temp_path.exists():
                shutil.rmtree(temp_path)
