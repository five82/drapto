"""Scene detection utilities for video processing"""

import functools
import logging
logger = logging.getLogger(__name__)
import logging
import re
from pathlib import Path
from typing import List, Optional, Tuple

from scenedetect import detect, ContentDetector, AdaptiveDetector
from scenedetect.scene_manager import save_images

from ..utils import run_cmd
from ..formatting import print_check, print_warning
from ..ffprobe_utils import (
    probe_session, MetadataError, get_format_info,
    get_duration, get_video_info
)
from ..config import (
    SCENE_THRESHOLD, HDR_SCENE_THRESHOLD, TARGET_MIN_SEGMENT_LENGTH, MAX_SEGMENT_LENGTH
)

logger = logging.getLogger(__name__)

@functools.lru_cache(maxsize=None)
def detect_scenes(input_file: Path) -> List[float]:
    """
    Improved scene detection for dynamic segmentation.
    
    This function uses candidate scene detection via PySceneDetect, filters out
    scenes that are too close together (less than MIN_SCENE_INTERVAL apart), and
    inserts artificial boundaries if a gap exceeds MAX_SEGMENT_LENGTH.
    
    Args:
        input_file: Path to input video file.
    
    Returns:
        List[float]: Sorted list of scene-change timestamps (in seconds) for segmentation.
    """
    from ..utils import run_cmd
    from ..config import SCENE_THRESHOLD, HDR_SCENE_THRESHOLD, TARGET_MIN_SEGMENT_LENGTH, MAX_SEGMENT_LENGTH
    import math

    # 1. Get total duration of the video via ffprobe.
    try:
        try:
            total_duration = get_duration(input_file)
            if total_duration <= 0:
                logger.warning("Invalid duration %.2f, using fallback detection", total_duration)
                return []
        except MetadataError as e:
            logger.error("Could not get video duration: %s", e)
            return []
                
            if total_duration < 2.0:  # Minimum duration for scene detection
                logger.info("Skipping scene detection for ultra-short video")
                return [total_duration]  # Single segment
                
    except Exception as e:
        logger.error("Failed to get video duration: %s", e)
        return []

    # 2. Determine scene detection threshold based on HDR or SDR.
    try:
        try:
            video_info = get_video_info(input_file)
            ct = (video_info.get("color_transfer") or "").lower()
            cp = video_info.get("color_primaries") or ""
            cs = video_info.get("color_space") or ""
                
            if ct in ["smpte2084", "arib-std-b67", "smpte428", "bt2020-10", "bt2020-12"]:
                threshold_val = HDR_SCENE_THRESHOLD
            else:
                threshold_val = SCENE_THRESHOLD
        except MetadataError as e:
            logger.warning("Could not determine color properties: %s", e)
            threshold_val = SCENE_THRESHOLD
    except Exception:
        threshold_val = SCENE_THRESHOLD

    # 4. Run candidate scene detection using PySceneDetect.
    try:
        candidates = detect(str(input_file), ContentDetector(threshold=threshold_val, min_scene_len=int(TARGET_MIN_SEGMENT_LENGTH)))
        candidate_timestamps = []
        for scene in candidates:
            if hasattr(scene, "start_time"):
                candidate_timestamps.append(scene.start_time.get_seconds())
            elif isinstance(scene, (tuple, list)) and scene:
                try:
                    candidate_timestamps.append(float(scene[0]))
                except Exception:
                    continue
        candidate_timestamps.sort()
    except Exception as e:
        logger.error("Candidate scene detection failed: %s", e)
        candidate_timestamps = []

    # Filter out scenes that are too close together
    filtered_scenes = []
    last_ts = 0.0
    for ts in candidate_timestamps:
        if ts - last_ts >= TARGET_MIN_SEGMENT_LENGTH:
            filtered_scenes.append(ts)
            last_ts = ts

    # Insert artificial boundaries for gaps exceeding MAX_SEGMENT_LENGTH
    final_boundaries = []
    prev_boundary = 0.0
    for ts in filtered_scenes:
        gap = ts - prev_boundary
        if gap > MAX_SEGMENT_LENGTH:
            # Insert additional boundaries every MAX_SEGMENT_LENGTH seconds
            num_inserts = int(gap // MAX_SEGMENT_LENGTH)
            for i in range(1, num_inserts + 1):
                final_boundaries.append(prev_boundary + i * MAX_SEGMENT_LENGTH)
        final_boundaries.append(ts)
        prev_boundary = ts
    
    # Check for potential gap after last scene to end of video
    if total_duration - prev_boundary > MAX_SEGMENT_LENGTH:
        remaining_gap = total_duration - prev_boundary
        num_inserts = int(remaining_gap // MAX_SEGMENT_LENGTH)
        for i in range(1, num_inserts + 1):
            final_boundaries.append(prev_boundary + i * MAX_SEGMENT_LENGTH)

    # Ensure boundaries are sorted and unique
    final_boundaries = sorted(set(final_boundaries))
    logger.info("Detected %d scenes, final boundaries: %r", len(candidate_timestamps), final_boundaries)
    return final_boundaries

def validate_segment_boundaries(
    segments_dir: Path,
    scene_timestamps: List[float],
    min_duration: float = 1.0,
    scene_tolerance: float = 0.5
) -> List[Tuple[Path, bool]]:
    """
    Validate segment durations against scene change points
    
    Args:
        segments_dir: Directory containing video segments
        scene_timestamps: List of scene change timestamps
        min_duration: Minimum acceptable segment duration
        scene_tolerance: Maximum distance (in seconds) to consider a segment boundary
                        aligned with a scene change
        
    Returns:
        List of tuples (segment_path, is_scene_boundary) for segments shorter
        than min_duration
    """
    short_segments = []
    
    try:
        segments = sorted(segments_dir.glob("*.mkv"))
        cumulative_duration = 0.0
        
        for segment in segments:
            try:
                # Get segment duration via utility
                duration = get_duration(segment)
                
                if duration < min_duration:
                    # Check if this segment boundary aligns with a scene change
                    segment_end = cumulative_duration + duration
                    is_scene = any(
                        abs(scene_time - segment_end) <= scene_tolerance
                        for scene_time in scene_timestamps
                    )
                    
                    if is_scene:
                        print_check(
                            f"Short segment {segment.name} ({duration:.2f}s) "
                            "aligns with scene change"
                        )
                    else:
                        print_warning(
                            f"Short segment {segment.name} ({duration:.2f}s) "
                            "does not align with scene changes"
                        )
                        
                    short_segments.append((segment, is_scene))
                
                cumulative_duration += duration
            except Exception as e:
                logger.error("Failed to validate segment %s: %s", segment.name, e)
            
        return short_segments
        
    except Exception as e:
        logger.error("Failed to validate segment boundaries: %s", e)
        return []
