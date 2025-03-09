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
from ..exceptions import (
    SegmentationError, ValidationError,
    SegmentMergeError
)

from ..utils import run_cmd, check_dependencies
from ..formatting import print_info, print_check

logger = logging.getLogger(__name__)

def merge_segments(segments: List[Path], output: Path) -> None:
    """
    Merge two segments using ffmpeg's concat demuxer
    
    Args:
        segment1: First segment
        segment2: Second segment to append
        output: Output path for merged segment
        
    Raises:
        SegmentMergeError: If merging fails
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
            logger.error("Failed to create merged segment")
            raise SegmentMergeError("Failed to create merged segment", module="segmentation")
            
    except Exception as e:
        logger.error("Failed to merge segments: %s", e)
        raise SegmentMergeError(f"Failed to merge segments: {str(e)}", module="segmentation") from e
    finally:
        if concat_file.exists():
            concat_file.unlink()

def validate_segments(input_file: Path, variable_segmentation: bool = True) -> None:
    """
    Validate video segments after segmentation.
    
    Args:
        input_file: Original input video file for duration comparison.
        variable_segmentation: Always True, as only scene-based segmentation is supported.
        
    Raises:
        ValidationError: If segments are invalid
        SegmentationError: If segment validation fails
    """
    from .scene_detection import detect_scenes, validate_segment_boundaries
    segments_dir = WORKING_DIR / "segments"
    segments = sorted(segments_dir.glob("*.mkv"))
    
    if not segments:
        raise SegmentationError("No segments found in segments directory", module="segmentation")
    logger.info("Found %d segments", len(segments))
        
    logger.info("Variable segmentation in use")
    try:
        from ..ffprobe_utils import get_format_info
        format_info = get_format_info(input_file)
        total_duration = float(format_info.get("duration", 0))
    except Exception as e:
        raise SegmentationError(f"Failed to get input duration: {str(e)}", module="segmentation") from e
        
    # Validate each segment and build a list of valid segments
    total_segment_duration = 0.0
    min_size = 1024  # 1KB minimum segment size
    valid_segments = []
    
    for segment in segments:
        # Check file size
        if segment.stat().st_size < min_size:
            msg = f"Segment too small: {segment.name}"
            logger.error(msg)
            raise ValidationError(msg, module="segmentation")
    
        try:
            from ..ffprobe_utils import get_format_info, get_video_info
            format_info = get_format_info(segment)
            video_info = get_video_info(segment)
            
            duration = float(format_info.get("duration", 0))
            codec = video_info.get("codec_name")
            
            if not duration or not codec:
                msg = f"Invalid segment {segment.name}: missing duration or codec"
                logger.error(msg)
                raise ValidationError(msg, module="segmentation")
                
            logger.info("Segment %s: duration=%.2fs, codec=%s", segment.name, duration, codec)

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
                raise ValidationError(
                    f"Segment {segment.name} timestamp issue: video_start={video_start:.2f}s is not near 0",
                    module="segmentation"
                )
    
            # Check if segment duration is short
            if duration < 1.0 and valid_segments:
                # Try to merge with previous segment
                prev_segment, prev_duration = valid_segments[-1]
                merged_name = f"merged_{prev_segment.stem}_{segment.stem}.mkv"
                merged_path = segment.parent / merged_name
                
                if merge_segments([prev_segment, segment], merged_path):
                    logger.info("Merged short segment %.2fs with previous segment", duration)
                    # Update the previous segment entry with merged segment
                    merged_duration = prev_duration + duration
                    valid_segments[-1] = (merged_path, merged_duration)
                    total_segment_duration += duration
                    # Clean up original segments
                    prev_segment.unlink()
                    segment.unlink()
                else:
                    msg = f"Failed to merge short segment: {segment.name}"
                    logger.error(msg)
                    raise SegmentMergeError(msg, module="segmentation")
            else:
                # Normal duration segment or first segment
                valid_segments.append((segment, duration))
                total_segment_duration += duration
    
        except Exception as e:
            logger.error("Failed to validate segment %s: %s", segment.name, e)
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
        msg = f"Failed to get input duration: {str(e)}"
        logger.error(msg)
        raise SegmentationError(msg, module="segmentation") from e

    # Check that total duration matches within tolerance
    duration_tolerance = max(1.0, total_duration * 0.02)  # 2% tolerance or minimum 1 second
    if abs(total_segment_duration - total_duration) > duration_tolerance:
        raise ValidationError(
            f"Total valid segment duration ({total_segment_duration:.2f}s) differs significantly from input ({total_duration:.2f}s)",
            module="segmentation"
        )

    # Detect scenes and validate segment boundaries against scene changes
    scenes = detect_scenes(input_file)
    short_segments = validate_segment_boundaries(segments_dir, scenes)
    
    # Don't fail validation for short segments that align with scene changes
    problematic_segments = [s for s, is_scene in short_segments if not is_scene]
    if problematic_segments:
        logger.warning(
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
        if not scenes:
            raise SegmentationError("Scene detection failed; no scenes detected", module="segmentation")
            
        from ..command_jobs import SegmentationJob
        cmd = build_segment_command(input_file, segments_dir, scenes, hw_opt)
        job = SegmentationJob(cmd)
        job.execute()
        
        # Validate segments
        validate_segments(input_file, variable_segmentation=True)
        
        return True
        
    except Exception as e:
        if isinstance(e, (SegmentationError, ValidationError)):
            raise
        raise SegmentationError(f"Segmentation failed: {str(e)}", module="segmentation") from e

