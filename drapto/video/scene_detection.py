"""Scene detection utilities for video processing"""

import logging
from pathlib import Path
from typing import List, Optional, Tuple

from scenedetect import detect, ContentDetector, AdaptiveDetector
from scenedetect.scene_manager import save_images

from ..utils import run_cmd
from ..formatting import print_check, print_warning
from ..config import SCENE_THRESHOLD, MIN_SCENE_INTERVAL, TARGET_SEGMENT_LENGTH

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
            timestamps = []
            last_time = 0.0
            for scene in scenes:
                log.debug("Processing scene object: %r (type: %s)", scene, type(scene))
                try:
                    if hasattr(scene, "start_time"):
                        start_time = scene.start_time.get_seconds()
                    elif isinstance(scene, (tuple, list)):
                        try:
                            start_time = float(scene[0])
                        except Exception as e_index:
                            log.warning("Error accessing index 0 of scene %r: %s", scene, e_index)
                            continue
                    elif isinstance(scene, (float, int)):
                        start_time = float(scene)
                    else:
                        log.warning("Unrecognized scene object: %r", scene)
                        continue
                    if start_time > 1.0 and (start_time - last_time) >= MIN_SCENE_INTERVAL:
                        timestamps.append(start_time)
                        last_time = start_time
                    else:
                        log.debug("Skipping scene at %.2fs (too close to previous)", start_time)
                except Exception as e:
                    log.warning("Error processing scene timestamp: %s", e)
                    continue

            max_gap = TARGET_SEGMENT_LENGTH * 1.5  # Allow 50% overrun
            final_timestamps = []
            last_time = 0.0

            for time in timestamps:
                if time - last_time > max_gap:
                    current = last_time + TARGET_SEGMENT_LENGTH
                    while current < time:
                        final_timestamps.append(current)
                        current += TARGET_SEGMENT_LENGTH
                final_timestamps.append(time)
                last_time = time

            try:
                duration = float(run_cmd([
                    "ffprobe", "-v", "error",
                    "-show_entries", "format=duration",
                    "-of", "default=noprint_wrappers=1:nokey=1",
                    str(input_file)
                ]).stdout.strip())
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
