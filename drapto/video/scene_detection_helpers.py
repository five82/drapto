"""Low-level scene detection helper functions

Responsibilities:
- Extract and filter scene change candidates
- Insert artificial boundaries for long segments
- Validate segment boundaries against detected scenes
"""

import logging
from pathlib import Path
from typing import List, Tuple

from scenedetect import detect, ContentDetector
from ..ffprobe.media import get_duration
from ..ffprobe.exec import MetadataError
from ..config import TARGET_MIN_SEGMENT_LENGTH, MAX_SEGMENT_LENGTH
from ..formatting import print_check, print_warning

logger = logging.getLogger(__name__)

def get_candidate_scenes(input_file: Path, threshold: float) -> list[float]:
    """
    Run PySceneDetect to obtain candidate scene change timestamps.

    Args:
        input_file: Path to the video file.
        threshold: Detection threshold value (HDR vs. SDR).

    Returns:
        Sorted list of detected candidate scene timestamps.
    """
    candidates = detect(str(input_file), ContentDetector(threshold=threshold, min_scene_len=int(TARGET_MIN_SEGMENT_LENGTH)))
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
    return candidate_timestamps

def filter_scene_candidates(candidate_timestamps: list[float], min_gap: float = TARGET_MIN_SEGMENT_LENGTH) -> list[float]:
    """
    Filter out candidate scenes that are too close together.

    Args:
        candidate_timestamps: List of candidate timestamps.
        min_gap: Minimum allowed gap between scenes.

    Returns:
        Filtered list of scene timestamps.
    """
    filtered = []
    last_ts = 0.0
    for ts in candidate_timestamps:
        if ts - last_ts >= min_gap:
            filtered.append(ts)
            last_ts = ts
    return filtered

def insert_artificial_boundaries(filtered_scenes: list[float], total_duration: float) -> list[float]:
    """
    Insert additional boundaries when there are gaps exceeding MAX_SEGMENT_LENGTH.

    Args:
        filtered_scenes: Sorted list of scene timestamps after filtering.
        total_duration: Total duration of the video.

    Returns:
        Final sorted list of scene boundaries.
    """
    final_boundaries = []
    prev_boundary = 0.0
    for ts in filtered_scenes:
        gap = ts - prev_boundary
        if gap > MAX_SEGMENT_LENGTH:
            num_inserts = int(gap // MAX_SEGMENT_LENGTH)
            for i in range(1, num_inserts + 1):
                final_boundaries.append(prev_boundary + i * MAX_SEGMENT_LENGTH)
        final_boundaries.append(ts)
        prev_boundary = ts
    if total_duration - prev_boundary > MAX_SEGMENT_LENGTH:
        num_inserts = int((total_duration - prev_boundary) // MAX_SEGMENT_LENGTH)
        for i in range(1, num_inserts + 1):
            final_boundaries.append(prev_boundary + i * MAX_SEGMENT_LENGTH)
    return sorted(set(final_boundaries))

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
                duration = get_duration(segment)
                
                if duration < min_duration:
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
