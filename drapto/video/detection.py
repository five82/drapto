"""
Video detection utilities for drapto
"""
import logging
import subprocess
from pathlib import Path
from typing import Optional, Tuple

log = logging.getLogger(__name__)

def detect_dolby_vision(input_file: Path) -> bool:
    """
    Detect if input file contains Dolby Vision
    
    Args:
        input_file: Path to input video file
        
    Returns:
        bool: True if Dolby Vision is detected
    """
    try:
        result = subprocess.run(
            ["mediainfo", str(input_file)],
            capture_output=True,
            text=True,
            check=True
        )
        return "Dolby Vision" in result.stdout
    except subprocess.CalledProcessError:
        log.warning("Failed to run mediainfo on %s", input_file)
        return False

def detect_crop(input_file: Path, disable_crop: bool = False) -> Optional[str]:
    """
    Detect black bars and return crop filter string
    
    Args:
        input_file: Path to input video file
        disable_crop: Skip crop detection if True
        
    Returns:
        Optional[str]: ffmpeg crop filter string or None if no crop needed
    """
    if disable_crop:
        log.info("Crop detection disabled")
        return None
        
    log.info("Crop detection not implemented, skipping crop filter")
    return None
