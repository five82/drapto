"""Scene detection utilities for video processing"""

import logging
import re
from pathlib import Path
from typing import List, Optional, Tuple

from scenedetect import detect, ContentDetector, AdaptiveDetector
from scenedetect.scene_manager import save_images

from ..utils import run_cmd
from ..formatting import print_check, print_warning
from ..config import (
    SCENE_THRESHOLD, MIN_SCENE_INTERVAL, DEFAULT_TARGET_SEGMENT_LENGTH,
    CLUSTER_WINDOW, MAX_SEGMENT_LENGTH
)

log = logging.getLogger(__name__)

def detect_scenes(input_file: Path) -> List[float]:
    """
    Improved scene detection for target VMAF encoding.
    This function computes ideal segment boundaries based on the video's total duration
    and TARGET_SEGMENT_LENGTH, then refines these boundaries using candidate scene changes
    detected via PySceneDetect. For each ideal boundary, if a candidate scene is found within
    a tolerance window, its timestamp is used; otherwise the ideal boundary is used.
    
    Args:
        input_file: Path to input video file
        
    Returns:
        List of scene-change timestamps (in seconds) for segmentation.
    """
    from .utils import run_cmd
    from ..config import TARGET_SEGMENT_LENGTH, SCENE_THRESHOLD, HDR_SCENE_THRESHOLD
    import math

    # 1. Get total duration of the video via ffprobe.
    try:
        result = run_cmd([
            "ffprobe", "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            str(input_file)
        ])
        total_duration = float(result.stdout.strip())
    except Exception as e:
        log.error("Failed to get video duration: %s", e)
        return []

    # 2. Calculate ideal segmentation boundaries.
    # If TARGET_SEGMENT_LENGTH is e.g. 15 sec, determine number of segments.
    num_boundaries = max(1, int(math.floor(total_duration / TARGET_SEGMENT_LENGTH)))
    ideal_boundaries = [i * (total_duration / (num_boundaries + 1)) for i in range(1, num_boundaries + 1)]

    # 3. Determine scene detection threshold based on HDR or SDR.
    try:
        result = run_cmd([
            "ffprobe", "-v", "error",
            "-select_streams", "v:0",
            "-show_entries", "stream=color_transfer",
            "-of", "default=noprint_wrappers=1:nokey=1",
            str(input_file)
        ])
        ct = result.stdout.strip().lower()
        if ct in ["smpte2084", "arib-std-b67", "smpte428", "bt2020-10", "bt2020-12"]:
            threshold_val = HDR_SCENE_THRESHOLD
        else:
            threshold_val = SCENE_THRESHOLD
    except Exception:
        threshold_val = SCENE_THRESHOLD

    # 4. Run candidate scene detection using PySceneDetect.
    from scenedetect import detect, ContentDetector
    try:
        candidates = detect(str(input_file), ContentDetector(threshold=threshold_val, min_scene_len=5))
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
        log.error("Candidate scene detection failed: %s", e)
        candidate_timestamps = []

    # 5. For each ideal boundary, select the candidate scene if it lies within tolerance.
    tolerance = 2.0  # seconds tolerance for matching ideal boundary with detected candidate
    final_boundaries = []
    for ideal in ideal_boundaries:
        # Find candidate scenes within tolerance of the ideal boundary.
        nearby = [ts for ts in candidate_timestamps if abs(ts - ideal) <= tolerance]
        if nearby:
            # Choose the candidate closest to the ideal boundary.
            best_candidate = min(nearby, key=lambda ts: abs(ts - ideal))
            final_boundaries.append(best_candidate)
        else:
            final_boundaries.append(ideal)
            
    # Ensure the boundaries are sorted and unique.
    final_boundaries = sorted(set(final_boundaries))
    log.info("Detected %d candidate scenes; final boundaries: %r", len(candidate_timestamps), final_boundaries)
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
            # Get segment duration
            result = run_cmd([
                "ffprobe", "-v", "error",
                "-show_entries", "format=duration",
                "-of", "default=noprint_wrappers=1:nokey=1",
                str(segment)
            ])
            duration = float(result.stdout.strip())
            
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
            
        return short_segments
        
    except Exception as e:
        log.error("Failed to validate segment boundaries: %s", e)
        return []
