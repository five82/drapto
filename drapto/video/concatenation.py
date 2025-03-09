"""Handles concatenation of encoded video segments into the final output."""

import json
import logging
from pathlib import Path
from ..utils import run_cmd
from ..config import WORKING_DIR
from ..ffprobe_utils import get_format_info, get_video_info

logger = logging.getLogger(__name__)

def concatenate_segments(output_file: Path) -> bool:
    """
    Concatenate encoded segments into final video.
    
    Args:
        output_file: Path for concatenated output.
    
    Returns:
        bool: True if concatenation successful.
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
            logger.error("Concatenated output is missing or empty")
            return False

        format_info = get_format_info(output_file)
        output_duration = float(format_info.get("duration", 0))
        
        if abs(output_duration - total_segment_duration) > 1.0:
            logger.error("Duration mismatch in concatenated output: %.2fs vs %.2fs", output_duration, total_segment_duration)
            return False

        video_info = get_video_info(output_file)
        if video_info.get("codec_name") != "av1":
            logger.error("Concatenated output has wrong codec: %s", result.stdout.strip())
            return False

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
            logger.error("Concatenated output video start time is %.2fs (expected near 0)", video_start)
            return False

        logger.info("Successfully validated concatenated output")
        return True

    except Exception as e:
        logger.error("Concatenation failed: %s", e)
        return False
    finally:
        if concat_file.exists():
            concat_file.unlink()
