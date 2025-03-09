"""Handles concatenation of encoded video segments into the final output."""

import json
import logging
from pathlib import Path
from ..utils import run_cmd
from ..config import WORKING_DIR
from ..ffprobe_utils import get_format_info, get_video_info, get_media_property
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
            result = run_cmd([
                "ffprobe", "-v", "error",
                "-show_entries", "format=duration",
                "-of", "default=noprint_wrappers=1:nokey=1",
                str(segment)
            ])
            duration = float(result.stdout.strip())
            total_segment_duration += duration
        
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

        format_info = get_format_info(output_file)
        output_duration = float(format_info.get("duration", 0))
        
        if abs(output_duration - total_segment_duration) > 1.0:
            raise ConcatenationError(
                f"Duration mismatch in concatenated output: {output_duration:.2f}s vs {total_segment_duration:.2f}s",
                module="concatenation"
            )

        if get_media_property(output_file, "video", "codec_name") != "av1":
            raise ConcatenationError(
                "Concatenated output has wrong codec - expected av1",
                module="concatenation"
            )

        # Validate concatenated output video start time
        sync_threshold = 0.1  # allowed difference in seconds

        vid_result = run_cmd([
            "ffprobe", "-v", "error",
            "-select_streams", "v:0",
            "-show_entries", "stream=start_time",
            "-of", "json",
            str(output_file)
        ])
        vid_data = json.loads(vid_result.stdout)
        video_start = float(vid_data["streams"][0].get("start_time") or 0)

        if abs(video_start) > sync_threshold:
            raise ConcatenationError(
                f"Concatenated output video start time is {video_start:.2f}s (expected near 0)",
                module="concatenation"
            )

        logger.info("Successfully validated concatenated output")

    except Exception as e:
        raise ConcatenationError(f"Concatenation failed: {str(e)}", module="concatenation") from e
    finally:
        if concat_file.exists():
            concat_file.unlink()
