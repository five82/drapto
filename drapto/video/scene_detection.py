"""
Scene Detection Module

Responsibilities:
  - Coordinate scene detection and boundary analysis
  - Handle HDR/SDR threshold determination
  - Manage scene detection workflow
"""

import functools
import logging
from pathlib import Path
from typing import List

from ..ffprobe.utils import (
    MetadataError, get_duration, get_video_info
)
from ..config import (
    SCENE_THRESHOLD, HDR_SCENE_THRESHOLD
)
from .scene_detection_helpers import (
    get_candidate_scenes,
    filter_scene_candidates,
    insert_artificial_boundaries,
    validate_segment_boundaries
)

logger = logging.getLogger(__name__)

@functools.lru_cache(maxsize=None)
def detect_scenes(input_file: Path) -> List[float]:
    """
    Improved scene detection for dynamic segmentation.
    
    This function uses candidate scene detection via PySceneDetect, filters out
    scenes that are too close together, and inserts artificial boundaries if a gap
    exceeds MAX_SEGMENT_LENGTH.
    
    Args:
        input_file: Path to input video file.
    
    Returns:
        List[float]: Sorted list of scene-change timestamps (in seconds) for segmentation.
    """
    # 1. Get total duration of the video via ffprobe.
    try:
        total_duration = get_duration(input_file)
        if total_duration <= 0:
            logger.warning("Invalid duration %.2f, using fallback detection", total_duration)
            return []
            
        if total_duration < 2.0:  # Minimum duration for scene detection
            logger.info("Skipping scene detection for ultra-short video")
            return [total_duration]  # Single segment
                
    except MetadataError as e:
        logger.error("Could not get video duration: %s", e)
        return []

    # 2. Determine scene detection threshold based on HDR or SDR.
    try:
        video_info = get_video_info(input_file)
        ct = (video_info.get("color_transfer") or "").lower()
        
        if ct in ["smpte2084", "arib-std-b67", "smpte428", "bt2020-10", "bt2020-12"]:
            threshold_val = HDR_SCENE_THRESHOLD
        else:
            threshold_val = SCENE_THRESHOLD
    except MetadataError as e:
        logger.warning("Could not determine color properties: %s", e)
        threshold_val = SCENE_THRESHOLD
    except Exception:
        threshold_val = SCENE_THRESHOLD

    # Run scene detection using helper functions
    try:
        candidate_ts = get_candidate_scenes(input_file, threshold_val)
        filtered_ts = filter_scene_candidates(candidate_ts)
        final_boundaries = insert_artificial_boundaries(filtered_ts, total_duration)
        logger.info("Detected %d scenes, final boundaries: %r", len(final_boundaries), final_boundaries)
        return final_boundaries
    except Exception as e:
        logger.error("Scene detection failed: %s", e)
        return []
