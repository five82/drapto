"""Dolby Vision encoding functions for drapto.

This module handles only Dolby Vision content encoding.
"""

import logging
import shutil
from pathlib import Path
from typing import Optional

logger = logging.getLogger(__name__)

from ..config import (
    PRESET, CRF_SD, CRF_HD, CRF_UHD, SVT_PARAMS,
    WORKING_DIR
)
from ..utils import run_cmd, run_cmd_interactive, run_cmd_with_progress
from ..formatting import print_check
from .hardware import get_hwaccel_options
from .detection import detect_crop
from .segmentation import segment_video
from .segment_encoding import encode_segments
from .concatenation import concatenate_segments

log = logging.getLogger(__name__)

def encode_dolby_vision(input_file: Path, disable_crop: bool = False) -> Optional[Path]:
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

    # Get total duration in seconds for progress reporting
    try:
        duration_result = run_cmd([
            "ffprobe", "-v", "error",
            "-select_streams", "v:0",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            str(input_file)
        ])
        total_duration = float(duration_result.stdout.strip())
    except Exception as e:
        logger.error("Could not get total duration for progress reporting: %s", e)
        total_duration = None  # We'll still run without progress percentages

    # Crop detection for Dolby Vision; disable if requested
    crop_filter = detect_crop(input_file, disable_crop)
        
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

    # Format and log the command for better readability
    formatted_cmd = " \\\n    ".join(cmd)
    log.info("Dolby Vision encoding command:\n%s", formatted_cmd)
    
    try:
        if run_cmd_with_progress(cmd, total_duration=total_duration, log_interval=5.0) == 0:
            return output_file
        
        log.error("Failed to encode Dolby Vision content")
        log.info("Retrying without hardware acceleration")
        try:
            # Retry is not really needed now because we already removed hwaccel options.
            if run_cmd_interactive(cmd) == 0:
                return output_file
        except Exception as e:
            log.error("Software fallback failed: %s", e)
        return None
    except Exception as e:
        log.error("Dolby Vision encoding failed: %s", e)
        return None

