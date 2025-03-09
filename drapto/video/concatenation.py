"""Handles concatenation of encoded video segments into the final output."""

import json
import logging
from pathlib import Path
from ..utils import run_cmd
from ..config import WORKING_DIR
from ..ffprobe_utils import (
    get_format_info, get_video_info, get_media_property,
    probe_session, MetadataError
)
from ..exceptions import ConcatenationError

logger = logging.getLogger(__name__)

def concatenate_segments(output_file: Path) -> None:
    """
    Concatenate encoded segments into final video.
    
    Raises:
        ConcatenationError: If concatenation fails
    """
    concat_file = WORKING_DIR / "concat.txt"
    try:
        total_segment_duration = 0
        segments = sorted((WORKING_DIR / "encoded_segments").glob("*.mkv"))
        
        for segment in segments:
            try:
                with probe_session(segment) as probe:
                    duration = float(probe.get("duration", "format"))
                    total_segment_duration += duration
            except MetadataError as e:
                logger.error("Failed to get segment duration: %s", e)
                raise ConcatenationError(f"Failed to get segment duration: {str(e)}", module="concatenation") from e
        
        with open(concat_file, 'w') as f:
            for segment in segments:
                f.write(f"file '{segment.absolute()}'\n")
            
        from .command_builders import build_concat_command
        from ..command_jobs import ConcatJob
        cmd = build_concat_command(segments, output_file, concat_file)
        job = ConcatJob(cmd)
        job.execute()

        if not output_file.exists() or output_file.stat().st_size == 0:
            raise ConcatenationError(
                "Concatenated output is missing or empty",
                module="concatenation"
            )

        try:
            with probe_session(output_file) as probe:
                output_duration = float(probe.get("duration", "format"))
                codec = probe.get("codec_name", "video")
                
                if abs(output_duration - total_segment_duration) > 1.0:
                    raise ConcatenationError(
                        f"Duration mismatch in concatenated output: {output_duration:.2f}s vs {total_segment_duration:.2f}s",
                        module="concatenation"
                    )
                    
                if codec != "av1":
                    raise ConcatenationError(
                        "Concatenated output has wrong codec - expected av1",
                        module="concatenation"
                    )
        except MetadataError as e:
            raise ConcatenationError(
                f"Failed to validate output codec: {str(e)}",
                module="concatenation"
            ) from e

        # Validate concatenated output video start time
        sync_threshold = 0.1  # allowed difference in seconds

        try:
            with probe_session(output_file) as probe:
                video_start = probe.get("start_time", "video")
                video_duration = probe.get("duration", "video")

            if abs(video_start) > sync_threshold:
                raise ConcatenationError(
                    f"Concatenated output video start time is {video_start:.2f}s (expected near 0)",
                    module="concatenation"
                )
        except MetadataError as e:
            raise ConcatenationError(f"Failed to validate video timing: {str(e)}", module="concatenation") from e

        logger.info("Successfully validated concatenated output")

    except Exception as e:
        raise ConcatenationError(f"Concatenation failed: {str(e)}", module="concatenation") from e
    finally:
        if concat_file.exists():
            concat_file.unlink()
