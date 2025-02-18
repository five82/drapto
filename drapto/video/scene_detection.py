"""Scene detection utilities for video processing"""

import logging
from pathlib import Path
from typing import List, Optional, Tuple

from scenedetect import detect, ContentDetector, AdaptiveDetector
from scenedetect.scene_manager import save_images

from ..utils import run_cmd
from ..formatting import print_check, print_warning
from ..config import (
    SCENE_THRESHOLD, MIN_SCENE_INTERVAL, TARGET_SEGMENT_LENGTH,
    CLUSTER_WINDOW, MAX_SEGMENT_LENGTH
)

log = logging.getLogger(__name__)

def detect_scenes(input_file: Path) -> List[float]:
    """
    Detect scene changes in video using PySceneDetect's content-aware detection.
    Optimizes scene boundaries to target roughly 10-second segments while preserving
    natural scene transitions.
    
    Args:
        input_file: Path to input video file
        
    Returns:
        List of timestamps (in seconds) where scene changes occur
    """
    try:
        try:
            log.debug("Starting scene detection on %s", input_file)
            scenes = detect(str(input_file),
                          ContentDetector(threshold=float(SCENE_THRESHOLD),
                                        min_scene_len=int(MIN_SCENE_INTERVAL)))
            log.debug("Raw scenes detected: %r", scenes)
        except Exception as e:
            log.error("Scene detection failed during detect() call: %s", e)
            raise

        if not scenes:
            try:
                scenes = detect(str(input_file),
                                AdaptiveDetector(min_scene_len=int(MIN_SCENE_INTERVAL)))
                log.debug("Adaptive scenes detected: %r", scenes)
            except Exception as e:
                log.error("Adaptive scene detection failed: %s", e)
                raise

        try:
            # Extract all scene timestamps
            raw_timestamps = []
            for scene in scenes:
                log.debug("Processing scene object: %r (type: %s)", scene, type(scene))
                try:
                    if hasattr(scene, "start_time"):
                        start_time = scene.start_time.get_seconds()
                    elif isinstance(scene, (tuple, list)):
                        # If the first element is a string in "HH:MM:SS.mmm" format, parse it.
                        if isinstance(scene[0], str):
                            try:
                                parts = scene[0].split(":")
                                if len(parts) == 3:
                                    hours = int(parts[0])
                                    minutes = int(parts[1])
                                    seconds = float(parts[2])
                                    start_time = hours * 3600 + minutes * 60 + seconds
                                else:
                                    start_time = float(scene[0])
                            except Exception as e_index:
                                log.warning("Error parsing time string in scene %r: %s", scene, e_index)
                                continue
                        else:
                            try:
                                start_time = float(scene[0])
                            except Exception as e_index:
                                log.warning("Error converting scene element in scene %r: %s", scene, e_index)
                                continue
                    elif isinstance(scene, (float, int)):
                        start_time = float(scene)
                    else:
                        log.warning("Unrecognized scene object: %r", scene)
                        continue
                    # Filter based on scene change strength if available
                    if start_time > 1.0:  # Skip very early scenes
                        if hasattr(scene, "change_score"):
                            if scene.change_score >= (SCENE_THRESHOLD / 2):
                                raw_timestamps.append(start_time)
                        else:
                            raw_timestamps.append(start_time)
                except Exception as e:
                    log.warning("Error processing scene timestamp: %s", e)
                    continue

            # Cluster nearby scene changes
            from itertools import groupby
            from operator import itemgetter
            from statistics import median

            def cluster_timestamps(times, window):
                if not times:
                    return []
                # Sort timestamps
                sorted_times = sorted(times)
                clusters = []
                current_cluster = [sorted_times[0]]
                
                for t in sorted_times[1:]:
                    # Use adaptive window based on configuration
                    adaptive_window = window
                    if hasattr(t, 'change_score'):
                        # Adjust window based on scene change strength
                        score_factor = t.change_score / SCENE_THRESHOLD
                        adaptive_window = window * (1.0 + score_factor)
                    
                    if t - current_cluster[-1] <= adaptive_window:
                        current_cluster.append(t)
                    else:
                        # Use median of cluster as representative timestamp
                        clusters.append(median(current_cluster))
                        current_cluster = [t]
                        
                if current_cluster:
                    clusters.append(median(current_cluster))
                    
                return clusters

            # Cluster nearby scene changes
            timestamps = cluster_timestamps(raw_timestamps, CLUSTER_WINDOW)
            
            # Process gaps between scenes
            final_timestamps = []
            last_time = 0.0
            
            for time in timestamps:
                gap = time - last_time
                # Only split if gap is significantly larger than target length
                if gap > MAX_SEGMENT_LENGTH or gap > TARGET_SEGMENT_LENGTH * 1.25:
                    # Add intermediate points for very long gaps
                    # Use dynamic spacing based on gap size
                    num_splits = max(1, int(gap / TARGET_SEGMENT_LENGTH))
                    split_size = gap / (num_splits + 1)
                    current = last_time + split_size
                    while current < time:
                        final_timestamps.append(current)
                        current += split_size
                final_timestamps.append(time)
                last_time = time

            try:
                duration = float(run_cmd([
                    "ffprobe", "-v", "error",
                    "-show_entries", "format=duration",
                    "-of", "default=noprint_wrappers=1:nokey=1",
                    str(input_file)
                ]).stdout.strip())
                max_gap = TARGET_SEGMENT_LENGTH * 1.5  # Allow 50% overrun
                while duration - last_time > max_gap:
                    last_time += TARGET_SEGMENT_LENGTH
                    final_timestamps.append(last_time)
            except Exception as e:
                log.warning("Could not get video duration: %s", e)

            log.info("Detected %d scene changes, optimized to %d segments", len(timestamps), len(final_timestamps))
            return final_timestamps
        except Exception as e:
            log.error("Scene detection failed: %s", e)
            return []
    except Exception as e:
        log.error("Scene detection failed: %s", e)
        return []

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
