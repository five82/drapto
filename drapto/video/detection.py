"""
Video detection utilities for drapto
"""
import logging
logger = logging.getLogger(__name__)
import subprocess
from pathlib import Path
from typing import Optional, Tuple

from ..utils import run_cmd
from ..ffprobe_utils import get_video_info, get_media_property, MetadataError


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

def detect_crop(input_file: Path, disable_crop: bool = None) -> Tuple[Optional[str], bool]:
    """
    Detect black bars and return an ffmpeg crop filter string.
    Mirrors the bash implementation by checking for HDR content,
    adjusting the crop threshold based on black level analysis,
    sampling frames, and aggregating crop values.
    
    Args:
        input_file: Path to input video file.
        disable_crop: If True, skip crop detection.
    
    Returns:
        Tuple of (crop filter string, is_hdr flag)
        The crop string will be like "crop=1920:800:0:140" or full dimensions if no cropping is needed.
    """
    import re
    from collections import Counter

    # Use config value if not explicitly set
    from ..config import DISABLE_CROP
    if disable_crop is None:
        disable_crop = DISABLE_CROP
        
    if disable_crop:
        logger.info("Crop detection disabled")
        return None

    logger.info("Analyzing video for black bars...")

    # Get video stream info from ffprobe_utils
    try:
        from ..ffprobe_utils import probe_session
        with probe_session(input_file) as probe:
            ct = probe.get("color_transfer")
            cp = probe.get("color_primaries") 
            cs = probe.get("color_space")
    except MetadataError as e:
        logger.error("Unable to read video color properties: %s", e)
        ct = cp = cs = ""

    # Set initial crop threshold and adjust for HDR content
    crop_threshold = 16
    is_hdr = False
    if (re.match(r"^(smpte2084|arib-std-b67|smpte428|bt2020-10|bt2020-12)$", ct)
            or cp == "bt2020"
            or re.match(r"^(bt2020nc|bt2020c)$", cs)):
        is_hdr = True
        crop_threshold = 128
        logger.info("HDR content detected, adjusting detection sensitivity")

    # For HDR input, sample a few frames to find average black level and adjust threshold
    if is_hdr:
        try:
            ffmpeg_cmd = [
                "ffmpeg", "-hide_banner", "-i", str(input_file),
                "-vf", "select='eq(n,0)+eq(n,100)+eq(n,200)',blackdetect=d=0:pic_th=0.1",
                "-f", "null", "-"
            ]
            result = run_cmd(ffmpeg_cmd, capture_output=True)
            # ffmpeg outputs blackdetect data on stderr
            output = result.stderr
            matches = re.findall(r"black_level:\s*([0-9.]+)", output)
            if matches:
                avg_black_level = sum(float(x) for x in matches) / len(matches)
                black_level = int(avg_black_level)
            else:
                black_level = 128
            crop_threshold = int(black_level * 3 / 2)
        except Exception as e:
            logger.error("Error during HDR black level analysis: %s", e)

    # Clamp crop_threshold within reasonable bounds
    if crop_threshold < 16:
        crop_threshold = 16
    elif crop_threshold > 256:
        crop_threshold = 256

    # Determine video duration via ffprobe
    try:
        with probe_session(input_file) as probe:
            duration = float(probe.get("duration", "format"))
            duration = int(round(duration))
    except MetadataError as e:
        logger.error("Failed to get duration: %s", e)
        duration = 0

    # Skip "credits" for long videos
    credits_skip = 0
    if duration > 3600:
        credits_skip = 180  # Skip 3 minutes for movies > 1 hour
    elif duration > 1200:
        credits_skip = 60   # Skip 1 minute for content > 20 minutes
    elif duration > 300:
        credits_skip = 30   # Skip 30 seconds for content > 5 minutes
    if credits_skip > 0 and duration > credits_skip:
        duration -= credits_skip

    interval = 5  # Check every 5 seconds
    total_samples = duration // interval
    if total_samples < 20:
        interval = duration // 20
        if interval < 1:
            interval = 1
        total_samples = 20
    logger.info("Analyzing %d frames for black bars (threshold: %d)...", total_samples, crop_threshold)

    # Get video dimensions
    try:
        with probe_session(input_file) as probe:
            orig_width = int(probe.get("width"))
            orig_height = int(probe.get("height"))
    except MetadataError as e:
        logger.error("Failed to get video dimensions: %s", e)
        return None

    # Run ffmpeg cropdetect filter over a sample of frames
    try:
        cropdetect_filter = f"select='not(mod(n,30))',cropdetect=limit={crop_threshold}:round=2:reset=1"
        frames = total_samples * 2
        ffmpeg_cmd = [
            "ffmpeg", "-hide_banner", "-i", str(input_file),
            "-vf", cropdetect_filter,
            "-frames:v", str(frames),
            "-f", "null", "-"
        ]
        result = run_cmd(ffmpeg_cmd, capture_output=True)
        output = result.stderr  # cropdetect output is in stderr
        # Find all crop= values, e.g. "crop=1920:800:0:140"
        matches = re.findall(r"crop=(\d+):(\d+):(\d+):(\d+)", output)
        # Filter to only consider crops that preserve the original width
        valid_crops = [(int(w), int(h), int(x), int(y))
                       for (w, h, x, y) in matches if int(w) == orig_width]
    except Exception as e:
        logger.error("Error during crop detection: %s", e)
        return None

    if not valid_crops:
        logger.info("No crop values detected, using full dimensions")
        return f"crop={orig_width}:{orig_height}:0:0"

    # Analyze crop heights (the second value) from valid crops; ignore very small crop heights (<100)
    crop_heights = [h for (_, h, _, _) in valid_crops if h >= 100]
    if not crop_heights:
        most_common_height = orig_height
    else:
        counter = Counter(crop_heights)
        most_common_height, _ = counter.most_common(1)[0]

    black_bar_size = (orig_height - most_common_height) // 2
    black_bar_percent = (black_bar_size * 100) // orig_height

    if black_bar_size > 0:
        logger.info("Found black bars: %d pixels (%d%% of height)", black_bar_size, black_bar_percent)
    else:
        logger.info("No significant black bars detected")

    if black_bar_percent > 1:
        crop_value = f"crop={orig_width}:{most_common_height}:0:{black_bar_size}"
    else:
        crop_value = f"crop={orig_width}:{orig_height}:0:0"

    return crop_value, is_hdr
