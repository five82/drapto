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
    Detect scene changes in video using PySceneDetect's content-aware detection.
    Optimizes scene boundaries to target roughly 10-second segments while preserving
    natural scene transitions.
    
    Args:
        input_file: Path to input video file
        
    Returns:
        List of timestamps (in seconds) where scene changes occur
    """
    # Determine if the video is HDR by checking the color_transfer property via ffprobe
    try:
        result = run_cmd([
            "ffprobe", "-v", "error",
            "-select_streams", "v:0",
            "-show_entries", "stream=color_transfer",
            "-of", "default=noprint_wrappers=1:nokey=1",
            str(input_file)
        ])
        color_transfer = result.stdout.strip().lower()
    except Exception as e:
        log.warning("Failed to obtain color_transfer info: %s", e)
        color_transfer = ""

    from ..config import HDR_SCENE_THRESHOLD  # already importing SCENE_THRESHOLD elsewhere
    if color_transfer in ["smpte2084", "arib-std-b67", "smpte428", "bt2020-10", "bt2020-12"]:
        used_threshold = HDR_SCENE_THRESHOLD
        log.info("HDR content detected, using HDR_SCENE_THRESHOLD: %s", used_threshold)
    else:
        used_threshold = SCENE_THRESHOLD

    try:
        try:
            log.debug("Starting scene detection on %s", input_file)
            scenes = detect(str(input_file),
                          ContentDetector(threshold=float(used_threshold),
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
                """Cluster timestamps with weighted medians and outlier rejection."""
                if not times:
                    return []
        
                from statistics import stdev, mean
    
                # Sort timestamps and extract scores if available
                sorted_times = []
                weights = []
                for t in sorted(times):
                    if hasattr(t, 'change_score'):
                        score = t.change_score / SCENE_THRESHOLD
                        # Smooth extreme scores
                        weight = min(max(score, 0.5), 2.0)
                    else:
                        weight = 1.0
                    sorted_times.append(float(t))
                    weights.append(weight)
    
                # Compute gaps between timestamps
                gaps = [sorted_times[i] - sorted_times[i-1] for i in range(1, len(sorted_times))]
                if gaps:
                    gap_mean = mean(gaps)
                    gap_std = stdev(gaps) if len(gaps) > 1 else gap_mean * 0.5
        
                    # Filter outliers (gaps that deviate too much from mean)
                    outlier_threshold = gap_mean + (2 * gap_std)
                    filtered_times = [sorted_times[0]]  # Keep first timestamp
                    filtered_weights = [weights[0]]
        
                    for i in range(1, len(sorted_times)):
                        gap = sorted_times[i] - filtered_times[-1]
                        if gap < outlier_threshold:
                            filtered_times.append(sorted_times[i])
                            filtered_weights.append(weights[i])
                else:
                    filtered_times = sorted_times
                    filtered_weights = weights
    
                # Cluster with weighted medians
                clusters = []
                current_cluster = [(filtered_times[0], filtered_weights[0])]
    
                for t, w in zip(filtered_times[1:], filtered_weights[1:]):
                    # Compute adaptive window based on local contrast
                    adaptive_window = window
                    if len(current_cluster) > 1:
                        # Adjust window based on weight strength
                        avg_weight = sum(w for _, w in current_cluster) / len(current_cluster)
                        adaptive_window *= (1.0 + avg_weight) / 2
        
                    if t - current_cluster[-1][0] <= adaptive_window:
                        current_cluster.append((t, w))
                    else:
                        # Compute weighted median for cluster
                        total_weight = sum(w for _, w in current_cluster)
                        cumsum = 0
                        for time, weight in sorted(current_cluster):
                            cumsum += weight
                            if cumsum >= total_weight / 2:
                                clusters.append(time)
                                break
                        current_cluster = [(t, w)]
    
                if current_cluster:
                    # Handle last cluster
                    total_weight = sum(w for _, w in current_cluster)
                    cumsum = 0
                    for time, weight in sorted(current_cluster):
                        cumsum += weight
                        if cumsum >= total_weight / 2:
                            clusters.append(time)
                            break
    
                # Post-process: merge very close scenes
                if len(clusters) > 1:
                    merged = [clusters[0]]
                    for c in clusters[1:]:
                        if c - merged[-1] >= 1.5:  # Minimum 1.5s gap
                            merged.append(c)
                    clusters = merged
    
                return clusters

            # Cluster nearby scene changes
            timestamps = cluster_timestamps(raw_timestamps, CLUSTER_WINDOW)
            
            # Calculate dynamic target segment length with enhanced HDR handling
            from statistics import median, mean, stdev
            
            # For HDR content, analyze contrast variance
            contrast_factor = 1.0
            if color_transfer in ["smpte2084", "arib-std-b67", "smpte428", "bt2020-10", "bt2020-12"]:
                try:
                    # Sample frames for contrast analysis
                    result = run_cmd([
                        "ffmpeg", "-i", str(input_file),
                        "-vf", "select='eq(pict_type,I)',blackdetect=d=0:pic_th=0.1",
                        "-frames:v", "30",
                        "-f", "null", "-"
                    ], capture_output=True)
                    
                    # Parse black levels to estimate contrast variance
                    black_levels = re.findall(r"black_level:\s*([0-9.]+)", result.stderr)
                    if black_levels:
                        levels = [float(x) for x in black_levels]
                        level_std = stdev(levels) if len(levels) > 1 else 0
                        # Adjust contrast factor based on variance
                        contrast_factor = max(0.8, min(1.2, 1.0 + (level_std / 128)))
                        log.debug("HDR contrast factor: %.2f", contrast_factor)
                except Exception as e:
                    log.warning("HDR contrast analysis failed: %s", e)
            
            # Calculate weighted dynamic target
            gaps = []  # Ensure gaps is always defined
            if len(timestamps) > 1:
                gaps = [timestamps[i] - timestamps[i-1] for i in range(1, len(timestamps))]
                if len(gaps) > 2:
                    # Remove extreme outliers before calculating target
                    gap_mean = mean(gaps)
                    gap_std = stdev(gaps)
                    filtered_gaps = [g for g in gaps if abs(g - gap_mean) <= 2 * gap_std]
                    if filtered_gaps:
                        dynamic_target = median(filtered_gaps) * contrast_factor
                    else:
                        dynamic_target = gap_mean * contrast_factor
                else:
                    dynamic_target = median(gaps) * contrast_factor
            else:
                dynamic_target = DEFAULT_TARGET_SEGMENT_LENGTH
                
            log.debug("Scene analysis:")
            log.debug("  Raw scene count: %d", len(raw_timestamps))
            log.debug("  Filtered scene count: %d", len(timestamps))
            log.debug("  Dynamic target: %.2fs", dynamic_target)
            if gaps and len(gaps) > 1:
                log.debug("  Gap statistics - Mean: %.2fs, StdDev: %.2fs", 
                         mean(gaps), stdev(gaps))
            
            # Process gaps between scenes
            final_timestamps = []
            last_time = 0.0
            
            for time in timestamps:
                gap = time - last_time
                # Only split if gap is significantly larger than dynamic target length
                if gap > MAX_SEGMENT_LENGTH or gap > dynamic_target * 1.25:
                    # Add intermediate points for very long gaps
                    # Use dynamic spacing based on gap size
                    num_splits = max(1, int(gap / dynamic_target))
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
                max_gap = dynamic_target * 1.5  # Allow 50% overrun based on dynamic target
                while duration - last_time > max_gap:
                    last_time += dynamic_target
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
