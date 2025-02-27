"""Video segmentation and parallel encoding functions"""

import json
import logging
import os
import shutil
import tempfile
from pathlib import Path
from typing import List, Optional

from ..config import (
    TARGET_VMAF, VMAF_SAMPLE_COUNT,
    VMAF_SAMPLE_LENGTH, PRESET, SVT_PARAMS,
    WORKING_DIR
)

from ..utils import run_cmd, check_dependencies
from ..formatting import print_info, print_check

log = logging.getLogger(__name__)

def merge_segments(segments: List[Path], output: Path) -> bool:
    """
    Merge two segments using ffmpeg's concat demuxer
    
    Args:
        segment1: First segment
        segment2: Second segment to append
        output: Output path for merged segment
        
    Returns:
        bool: True if merge successful
    """
    # Create temporary concat file
    concat_file = output.parent / "concat.txt"
    try:
        with open(concat_file, 'w') as f:
            for segment in segments:
                f.write(f"file '{segment.absolute()}'\n")
            
        cmd = [
            "ffmpeg", "-hide_banner", "-loglevel", "warning",
            "-f", "concat",
            "-safe", "0",
            "-i", str(concat_file),
            "-c", "copy",
            "-y", str(output)
        ]
        run_cmd(cmd)
        
        # Verify merged output
        if not output.exists() or output.stat().st_size == 0:
            log.error("Failed to create merged segment")
            return False
            
        return True
    except Exception as e:
        log.error("Failed to merge segments: %s", e)
        return False
    finally:
        if concat_file.exists():
            concat_file.unlink()

def validate_segments(input_file: Path, variable_segmentation: bool = True) -> bool:
    """
    Validate video segments after segmentation.
    
    Args:
        input_file: Original input video file for duration comparison.
        variable_segmentation: Always True, as only scene-based segmentation is supported.
        
    Returns:
        bool: True if all segments are valid.
    """
    from .scene_detection import detect_scenes, validate_segment_boundaries
    segments_dir = WORKING_DIR / "segments"
    segments = sorted(segments_dir.glob("*.mkv"))
    
    if not segments:
        log.error("No segments created")
        return False
    log.info("Found %d segments", len(segments))
        
    log.info("Variable segmentation in use")
    try:
        from ..ffprobe_utils import get_format_info
        format_info = get_format_info(input_file)
        total_duration = float(format_info.get("duration", 0))
    except Exception as e:
        log.error("Failed to get input duration: %s", e)
        return False
        
    # Validate each segment and build a list of valid segments
    total_segment_duration = 0.0
    min_size = 1024  # 1KB minimum segment size
    valid_segments = []
    
    for segment in segments:
        # Check file size
        if segment.stat().st_size < min_size:
            log.error("Segment too small: %s", segment.name)
            return False
    
        try:
            from ..ffprobe_utils import get_format_info, get_video_info
            format_info = get_format_info(segment)
            video_info = get_video_info(segment)
            
            duration = float(format_info.get("duration", 0))
            codec = video_info.get("codec_name")
            
            if not duration or not codec:
                log.error("Invalid segment %s: missing duration or codec", segment.name)
                return False
                
            log.info("Segment %s: duration=%.2fs, codec=%s", segment.name, duration, codec)

            # Validate the segment's video timestamps.
            # Since segments are created without audio (-an), we only check that the video start time is near zero.
            sync_threshold = 0.2  # increased allowed difference in seconds

            vid_result = run_cmd([
                "ffprobe", "-v", "error",
                "-select_streams", "v:0",
                "-show_entries", "stream=start_time",
                "-of", "json",
                str(segment)
            ])
            vid_data = json.loads(vid_result.stdout)
            video_start = float(vid_data["streams"][0].get("start_time") or 0)

            if abs(video_start) > sync_threshold:
                log.error("Segment %s timestamp issue: video_start=%.2fs is not near 0", segment.name, video_start)
                return False
    
            # Check if segment duration is short
            if duration < 1.0 and valid_segments:
                # Try to merge with previous segment
                prev_segment, prev_duration = valid_segments[-1]
                merged_name = f"merged_{prev_segment.stem}_{segment.stem}.mkv"
                merged_path = segment.parent / merged_name
                
                if merge_segments([prev_segment, segment], merged_path):
                    log.info("Merged short segment %.2fs with previous segment", duration)
                    # Update the previous segment entry with merged segment
                    merged_duration = prev_duration + duration
                    valid_segments[-1] = (merged_path, merged_duration)
                    total_segment_duration += duration
                    # Clean up original segments
                    prev_segment.unlink()
                    segment.unlink()
                else:
                    log.error("Failed to merge short segment: %s", segment.name)
                    return False
            else:
                # Normal duration segment or first segment
                valid_segments.append((segment, duration))
                total_segment_duration += duration
    
        except Exception as e:
            log.error("Failed to validate segment %s: %s", segment.name, e)
            return False
    
    # After processing, validate total duration
    valid_count = len(valid_segments)
    try:
        result = run_cmd([
            "ffprobe", "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            str(input_file)
        ])
        total_duration = float(result.stdout.strip())
    except Exception as e:
        log.error("Failed to get input duration: %s", e)
        return False

    # Check that total duration matches within tolerance
    duration_tolerance = max(1.0, total_duration * 0.02)  # 2% tolerance or minimum 1 second
    if abs(total_segment_duration - total_duration) > duration_tolerance:
        log.error("Total valid segment duration (%.2fs) differs significantly from input (%.2fs)",
                  total_segment_duration, total_duration)
        return False

    # Detect scenes and validate segment boundaries against scene changes
    scenes = detect_scenes(input_file)
    short_segments = validate_segment_boundaries(segments_dir, scenes)
    
    # Don't fail validation for short segments that align with scene changes
    problematic_segments = [s for s, is_scene in short_segments if not is_scene]
    if problematic_segments:
        log.warning(
            "Found %d problematic short segments not aligned with scene changes",
            len(problematic_segments)
        )
    
    print_check(f"Successfully validated {valid_count} segments")
    return True

def segment_video(input_file: Path) -> bool:
    """
    Segment video into chunks for parallel encoding
    
    Args:
        input_file: Path to input video file
        
    Returns:
        bool: True if segmentation successful
    """
    from .hardware import check_hardware_acceleration, get_hwaccel_options
    
    segments_dir = WORKING_DIR / "segments"
    segments_dir.mkdir(parents=True, exist_ok=True)
    
    try:
        # Check for hardware decoding support
        hw_type = check_hardware_acceleration()
        hw_opt = get_hwaccel_options(hw_type)
        
        from .scene_detection import detect_scenes
        from .command_builders import build_segment_command
        
        scenes = detect_scenes(input_file)
        if scenes:
            from ..command_jobs import SegmentationJob
            from ..command_jobs import SegmentationJob
            cmd = build_segment_command(input_file, segments_dir, scenes, hw_opt)
            job = SegmentationJob(cmd)
            job.execute()
            variable_seg = True
        else:
            log.error("Scene detection failed; no scenes detected. Failing segmentation.")
            return False
            
        job = SegmentationJob(cmd)
        job.execute()
        
        # Validate segments with the appropriate variable_segmentation flag
        if not validate_segments(input_file, variable_segmentation=True):
            return False
            
        return True
        
    except Exception as e:
        log.error("Segmentation failed: %s", e)
        return False

