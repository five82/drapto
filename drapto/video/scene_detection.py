"""Scene detection utilities for video processing"""

import logging
from pathlib import Path
from typing import List, Optional, Tuple

from ..utils import run_cmd
from ..formatting import print_check, print_warning

log = logging.getLogger(__name__)

def detect_scenes(input_file: Path, threshold: float = 0.3) -> List[float]:
    """
    Detect scene changes in video using FFmpeg's scene detection filter
    
    Args:
        input_file: Path to input video file
        threshold: Scene change threshold (0.0-1.0, default 0.3)
        
    Returns:
        List of timestamps (in seconds) where scene changes occur
    """
    scenes = []
    try:
        # Run FFmpeg with scene detection filter
        cmd = [
            "ffmpeg", "-hide_banner",
            "-i", str(input_file),
            "-vf", f"select=gt(scene,{threshold}),showinfo",
            "-f", "null", "-"
        ]
        result = run_cmd(cmd, capture_output=True, check=True)
        
        # Parse scene change timestamps from stderr output
        # Example line: "[Parsed_showinfo_1 @ 0x7f8f5c] n:   1 pts:    2.002 pts_time:2.002
        for line in result.stderr.splitlines():
            if "pts_time:" in line:
                try:
                    pts_time = float(line.split("pts_time:")[1].split()[0])
                    scenes.append(pts_time)
                except (ValueError, IndexError):
                    continue
                    
        log.info("Detected %d scene changes", len(scenes))
        return scenes
        
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
