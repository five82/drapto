"""Handles concatenation of encoded video segments into the final output."""

import json
import logging
from pathlib import Path
from ..utils import run_cmd
from ..config import WORKING_DIR

log = logging.getLogger(__name__)

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
            
        cmd = [
            "ffmpeg", "-hide_banner", "-loglevel", "error",
            "-fflags", "+genpts",
            "-f", "concat",
            "-safe", "0",
            "-i", str(concat_file),
            "-c", "copy",
            "-y", str(output_file)
        ]
        run_cmd(cmd)

        if not output_file.exists() or output_file.stat().st_size == 0:
            log.error("Concatenated output is missing or empty")
            return False

        result = run_cmd([
            "ffprobe", "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            str(output_file)
        ])
        output_duration = float(result.stdout.strip())
        
        if abs(output_duration - total_segment_duration) > 1.0:
            log.error("Duration mismatch in concatenated output: %.2fs vs %.2fs", output_duration, total_segment_duration)
            return False

        result = run_cmd([
            "ffprobe", "-v", "error",
            "-select_streams", "v",
            "-show_entries", "stream=codec_name",
            "-of", "default=noprint_wrappers=1:nokey=1",
            str(output_file)
        ])
        if result.stdout.strip() != "av1":
            log.error("Concatenated output has wrong codec: %s", result.stdout.strip())
            return False

        # Check AV sync in concatenated output
        sync_threshold = 0.1  # allowed difference in seconds

        # Check video stream start time
        vid_result = run_cmd([
            "ffprobe", "-v", "error",
            "-select_streams", "v:0",
            "-show_entries", "stream=start_time",
            "-of", "json",
            str(output_file)
        ])
        vid_data = json.loads(vid_result.stdout)
        video_start = float(vid_data["streams"][0].get("start_time") or 0)

        # Check audio stream start time
        aud_result = run_cmd([
            "ffprobe", "-v", "error",
            "-select_streams", "a:0",
            "-show_entries", "stream=start_time",
            "-of", "json",
            str(output_file)
        ])
        aud_data = json.loads(aud_result.stdout)
        audio_start = float(aud_data["streams"][0].get("start_time") or 0)

        if abs(video_start - audio_start) > sync_threshold:
            log.error("Concatenated output AV sync error: video_start=%.2fs, audio_start=%.2fs", 
                     video_start, audio_start)
            return False

        log.info("Successfully validated concatenated output")
        return True

    except Exception as e:
        log.error("Concatenation failed: %s", e)
        return False
    finally:
        if concat_file.exists():
            concat_file.unlink()
