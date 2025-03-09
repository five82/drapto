"""Video segmentation and parallel encoding functions"""

import logging
from pathlib import Path
from typing import List, Optional

from .scene_detection import detect_scenes, validate_segment_boundaries
from ..config import (
    TARGET_VMAF, VMAF_SAMPLE_COUNT,
    VMAF_SAMPLE_LENGTH, PRESET, SVT_PARAMS,
    WORKING_DIR
)
from ..exceptions import (
    SegmentationError, ValidationError,
    SegmentMergeError, SegmentEncodingError
)
from ..utils import run_cmd
from ..formatting import print_check
from ..ffprobe_utils import (
    MetadataError, probe_session
)

logger = logging.getLogger(__name__)

def merge_segments(segments: List[Path], output: Path) -> None:
    """
    Merge video segments using ffmpeg's concat demuxer
    
    Args:
        segments: List of segment paths to merge
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

def validate_segments(input_file: Path) -> bool:
    """
    Validate video segments after segmentation.
    
    Args:
        input_file: Original input video file for duration comparison.
        variable_segmentation: Always True, as only scene-based segmentation is supported.
        
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
        with probe_session(input_file) as probe:
            total_duration = probe.get("duration", "video")
    except MetadataError as e:
        raise SegmentationError(f"Failed to get input duration: {str(e)}", module="segmentation") from e
        
    # Validate each segment
    total_segment_duration = 0.0
    min_size = 1024  # 1KB minimum segment size
    
    for segment in segments:
        # Check file size
        if segment.stat().st_size < min_size:
            msg = f"Segment too small: {segment.name}"
            logger.error(msg)
            raise ValidationError(msg, module="segmentation")
    
        try:
            with probe_session(segment) as probe:
                duration = float(probe.get("duration", "format"))
                codec = probe.get("codec_name", "video")
                video_start = probe.get("start_time", "video")

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
    problematic_segments = [s for s, is_scene in short_segments if not is_scene]
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

