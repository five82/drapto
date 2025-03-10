"""
Video detection utilities for drapto

This module provides low-level video detection utilities including:
  - Identifying Dolby Vision content
  - Analyzing color properties and running blackdetect via ffmpeg to compute crop filters
  - Adjusting detection thresholds based on HDR content
  - Orchestrating frame sampling and analysis for black bar detection

It abstracts the orchestration of these tasks into helper functions.
"""
import logging
logger = logging.getLogger(__name__)
import subprocess
import re
from pathlib import Path
from typing import Optional, Tuple

from ..utils import run_cmd
from ..ffprobe.utils import (
    get_video_info, get_media_property, MetadataError,
    probe_session
)

def _determine_crop_threshold(ct: str, cp: str, cs: str) -> Tuple[int, bool]:
    """
    Determine the crop detection threshold based on color properties.
    Returns a tuple (crop_threshold, is_hdr).
    """
    crop_threshold = 16
    is_hdr = False
    if (re.match(r"^(smpte2084|arib-std-b67|smpte428|bt2020-10|bt2020-12)$", ct)
            or cp == "bt2020"
            or re.match(r"^(bt2020nc|bt2020c)$", cs)):
        is_hdr = True
        crop_threshold = 128
        logger.info("HDR content detected, adjusting detection sensitivity")
    return crop_threshold, is_hdr

def _run_hdr_blackdetect(input_file: Path, crop_threshold: int) -> int:
    """
    Run a set of ffmpeg commands to sample black levels for HDR content.
    Returns an updated crop threshold based on black level analysis.
    """
    try:
        ffmpeg_cmd = [
            "ffmpeg", "-hide_banner", "-i", str(input_file),
            "-vf", "select='eq(n,0)+eq(n,100)+eq(n,200)',blackdetect=d=0:pic_th=0.1",
            "-f", "null", "-"
        ]
        result = run_cmd(ffmpeg_cmd, capture_output=True)
        output = result.stderr
        matches = re.findall(r"black_level:\s*([0-9.]+)", output)
        if matches:
            avg_black_level = sum(float(x) for x in matches) / len(matches)
            black_level = int(avg_black_level)
            return int(black_level * 3 / 2)
        return crop_threshold
    except Exception as e:
        logger.error("Error during HDR black level analysis: %s", e)
        return crop_threshold


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
        detected = "Dolby Vision" in result.stdout
        if detected:
            logger.info("Dolby Vision detected")
        else:
            logger.info("Dolby Vision not detected")
        return detected
    except subprocess.CalledProcessError:
        logger.warning("Failed to run mediainfo on %s", input_file)
        return False

def _get_video_properties(input_file: Path) -> tuple[dict, tuple[int, int], float]:
    """Get video dimensions, color properties and duration"""
    try:
        with probe_session(input_file) as probe:
            # Get color properties
            color_props = {
                'transfer': probe.get("color_transfer", "video"),
                'primaries': probe.get("color_primaries", "video"),
                'space': probe.get("color_space", "video")
            }
            
            # Get dimensions
            width = int(probe.get("width", "video"))
            height = int(probe.get("height", "video"))
            
            # Get duration
            duration = float(probe.get("duration", "format"))
            
            return color_props, (width, height), duration
    except MetadataError as e:
        logger.error("Failed to get video properties: %s", e)
        return {}, (0, 0), 0.0

def _calculate_credits_skip(duration: float) -> float:
    """Calculate how much to skip at the end for credits"""
    if duration > 3600:
        return 180  # Skip 3 minutes for movies > 1 hour
    elif duration > 1200:
        return 60   # Skip 1 minute for content > 20 minutes
    elif duration > 300:
        return 30   # Skip 30 seconds for content > 5 minutes
    return 0

def _run_cropdetect(input_file: Path, crop_threshold: int,
                    dimensions: tuple[int, int], duration: float) -> Optional[str]:
    """Run ffmpeg cropdetect and analyze results"""
    orig_width, orig_height = dimensions
    
    # Calculate sampling parameters
    interval = 5  # Check every 5 seconds
    total_samples = int(duration) // interval
    if total_samples < 20:
        interval = duration // 20
        if interval < 1:
            interval = 1
        total_samples = 20

    try:
        # Run cropdetect
        cropdetect_filter = f"select='not(mod(n,30))',cropdetect=limit={crop_threshold}:round=2:reset=1"
        frames = total_samples * 2
        ffmpeg_cmd = [
            "ffmpeg", "-hide_banner", "-i", str(input_file),
            "-vf", cropdetect_filter,
            "-frames:v", str(frames),
            "-f", "null", "-"
        ]
        result = run_cmd(ffmpeg_cmd, capture_output=True)
        
        # Parse crop values
        matches = re.findall(r"crop=(\d+):(\d+):(\d+):(\d+)", result.stderr)
        valid_crops = [(int(w), int(h), int(x), int(y))
                      for (w, h, x, y) in matches if int(w) == orig_width]
                      
        if not valid_crops:
            logger.info("No crop values detected, using full dimensions")
            return f"crop={orig_width}:{orig_height}:0:0"
            
        # Analyze crop heights
        from collections import Counter
        crop_heights = [h for (_, h, _, _) in valid_crops if h >= 100]
        if not crop_heights:
            most_common_height = orig_height
        else:
            counter = Counter(crop_heights)
            most_common_height, _ = counter.most_common(1)[0]
            
        # Calculate black bars
        black_bar_size = (orig_height - most_common_height) // 2
        black_bar_percent = (black_bar_size * 100) // orig_height
        
        if black_bar_size > 0:
            logger.info("Found black bars: %d pixels (%d%% of height)",
                       black_bar_size, black_bar_percent)
        else:
            logger.info("No significant black bars detected")
            
        if black_bar_percent > 1:
            return f"crop={orig_width}:{most_common_height}:0:{black_bar_size}"
        return f"crop={orig_width}:{orig_height}:0:0"
        
    except Exception as e:
        logger.error("Error during crop detection: %s", e)
        return None

def detect_crop(input_file: Path, disable_crop: bool = None) -> Tuple[Optional[str], bool]:
    """
    Detect black bars and return an ffmpeg crop filter string.
    
    Args:
        input_file: Path to input video file.
        disable_crop: If True, skip crop detection.
    
    Returns:
        Tuple of (crop filter string, is_hdr flag)
        The crop string will be like "crop=1920:800:0:140" or full dimensions if no cropping is needed.
    """
    # Use config value if not explicitly set
    from ..config import DISABLE_CROP
    if disable_crop is None:
        disable_crop = DISABLE_CROP
        
    if disable_crop:
        logger.info("Crop detection disabled")
        return None, False

    logger.info("Analyzing video for black bars...")

    # Get video properties
    color_props, dimensions, duration = _get_video_properties(input_file)
    if not all(dimensions) or duration <= 0:
        return None, False

    # Determine crop threshold and HDR status
    crop_threshold, is_hdr = _determine_crop_threshold(
        color_props.get('transfer', ''),
        color_props.get('primaries', ''),
        color_props.get('space', '')
    )

    # For HDR content, analyze black levels
    if is_hdr:
        crop_threshold = _run_hdr_blackdetect(input_file, crop_threshold)
        crop_threshold = max(16, min(256, crop_threshold))

    # Adjust duration for credits
    credits_skip = _calculate_credits_skip(duration)
    if credits_skip > 0:
        duration -= credits_skip

    # Run crop detection
    crop_filter = _run_cropdetect(input_file, crop_threshold, dimensions, duration)
    return crop_filter, is_hdr
