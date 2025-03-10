"""
Video Segmentation Module

Responsibilities:
  - Perform scene detection and candidate selection
  - Filter and refine scene candidate timestamps
  - Insert artificial segmentation boundaries for long gaps
  - Orchestrate segmentation and subsequent segment validation
  - Validate segment boundaries against detected scenes
  - Handle merging of short segments when needed

High-level orchestration logic is separated from low-level scene filtering and boundary insertion.
Scene detection uses PySceneDetect with custom filtering and refinement logic.
"""

import logging
from pathlib import Path
from typing import List, Optional

from .scene_detection import detect_scenes, validate_segment_boundaries
from .hardware import check_hardware_acceleration, get_hwaccel_options
from ..exceptions import ValidationError

def _validate_single_segment(segment: Path, min_size: int = 1024) -> float:
    """
    Validate a single video segment.

    Args:
        segment: Path to the segment file.
        min_size: Minimum acceptable size for a segment in bytes.

    Returns:
        The segment duration as a float.

    Raises:
        ValidationError: If the segment is too small or has invalid properties.
    """
    if segment.stat().st_size < min_size:
        raise ValidationError(f"Segment too small: {segment.name}", module="segmentation")
    try:
        duration = get_duration(segment)
    except Exception as e:
        raise ValidationError(f"Could not determine duration for {segment.name}: {str(e)}", module="segmentation")
    return duration

def _prepare_segmentation(input_file: Path) -> tuple[str, list[float]]:
    """
    Prepare parameters for segmentation by checking for hardware acceleration and performing scene detection.

    Args:
        input_file: Path to the input video file.

    Returns:
        A tuple with the hardware option string and the list of scene change timestamps.
    """
    hw_type = check_hardware_acceleration()
    hw_opt = get_hwaccel_options(hw_type)
    scenes = detect_scenes(input_file)
    return hw_opt, scenes
from .command_builders import build_segment_command
from ..command_jobs import SegmentationJob
from ..config import WORKING_DIR
from ..exceptions import (
    SegmentationError, ValidationError,
    SegmentMergeError, SegmentEncodingError
)
from ..utils import run_cmd
from ..formatting import print_check
from ..ffprobe_utils import get_duration, get_video_info, MetadataError

logger = logging.getLogger(__name__)


def validate_segments(input_file: Path) -> bool:
    """
    Validate video segments after segmentation.
    
    Args:
        input_file: Original input video file for duration comparison.
        
    Raises:
        ValidationError: If segments are invalid
        SegmentationError: If segment validation fails
    """
    segments_dir = WORKING_DIR / "segments"
    segments = sorted(segments_dir.glob("*.mkv"))
    
    if not segments:
        raise SegmentationError("No segments found in segments directory", module="segmentation")
    logger.info("Found %d segments", len(segments))
        
    logger.info("Scene-based segmentation in use")
    try:
        try:
            # First try to get video stream duration
            total_duration = get_duration(input_file, "video")
            logger.debug("Using video stream duration: %.2fs", total_duration)
        except MetadataError:
            # Fall back to format duration if video stream duration unavailable
            total_duration = get_duration(input_file, "format")
            logger.debug("Using container duration: %.2fs", total_duration)
                
        if not isinstance(total_duration, (int, float)) or total_duration <= 0:
            raise MetadataError(f"Invalid duration value: {total_duration}")
            
    except MetadataError as e:
        raise SegmentationError(f"Failed to get input duration: {str(e)}", module="segmentation") from e
        
    # Validate each segment
    total_segment_duration = 0.0
    min_size = 1024  # 1KB minimum segment size
    
    for segment in segments:
        try:
            duration = _validate_single_segment(segment)
            video_info = get_video_info(segment)
            codec = video_info.get("codec_name")
            video_start = video_info.get("start_time", 0.0)

            if not duration or not codec:
                msg = f"Invalid segment {segment.name}: missing duration or codec"
                logger.error(msg)
                raise ValidationError(msg, module="segmentation")

            # Validate video timestamps
            sync_threshold = 0.2  # increased allowed difference in seconds
            if abs(video_start) > sync_threshold:
                raise ValidationError(
                    f"Segment {segment.name} timestamp issue: video_start={video_start:.2f}s is not near 0",
                    module="segmentation"
                )
            
            total_segment_duration += duration
            logger.info("Segment %s: duration=%.2fs, codec=%s", segment.name, duration, codec)
        except (MetadataError, ValidationError) as e:
            logger.error("Failed to validate segment timing: %s", e)
            raise ValidationError(
                f"Failed to validate segment {segment.name} timing",
                module="segmentation"
            ) from e
    
    
    # Check that total duration matches within tolerance
    duration_tolerance = max(1.0, total_duration * 0.02)  # 2% tolerance or minimum 1 second
    if abs(total_segment_duration - total_duration) > duration_tolerance:
        raise ValidationError(
            f"Total valid segment duration ({total_segment_duration:.2f}s) "
            f"differs significantly from input ({total_duration:.2f}s)",
            module="segmentation"
        )

    # Detect scenes and validate segment boundaries against scene changes
    scenes = detect_scenes(input_file)
    short_segments = validate_segment_boundaries(segments_dir, scenes)
    
    # Don't fail validation for short segments that align with scene changes
    problematic_segments = [_s for _s, is_scene in short_segments if not is_scene]
    if problematic_segments:
        logger.warning(
            "Found %d problematic short segments not aligned with scene changes",
            len(problematic_segments)
        )
    
    print_check(f"Successfully validated {len(segments)} segments")
    return True

def segment_video(input_file: Path) -> bool:
    """
    Segment video into chunks for parallel encoding
    
    Args:
        input_file: Path to input video file
        
    Returns:
        bool: True if segmentation successful
    """
    segments_dir = WORKING_DIR / "segments"
    segments_dir.mkdir(parents=True, exist_ok=True)
    
    try:
        # Prepare segmentation parameters
        hw_opt, scenes = _prepare_segmentation(input_file)
        if not scenes:
            raise SegmentationError("Scene detection failed; no scenes detected", module="segmentation")
            
        cmd = build_segment_command(input_file, segments_dir, scenes, hw_opt)
        job = SegmentationJob(cmd)
        job.execute()
        
        # Validate segments
        validate_segments(input_file)
        
        return True
        
    except Exception as e:
        if isinstance(e, (SegmentationError, ValidationError)):
            raise
        raise SegmentationError(f"Segmentation failed: {str(e)}", module="segmentation") from e

