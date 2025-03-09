"""Handles muxing of video and audio streams"""

import json
import logging
from pathlib import Path
from typing import List, Optional

from .utils import run_cmd

logger = logging.getLogger(__name__)

def mux_tracks(
    video_track: Path,
    audio_tracks: List[Path],
    output_file: Path
) -> bool:
    """
    Mux video and audio tracks into final output file
    
    Args:
        video_track: Path to encoded video track
        audio_tracks: List of paths to encoded audio tracks
        output_file: Path for final muxed output
        
    Returns:
        bool: True if muxing successful
    """
    logger.info("Muxing tracks to: %s", output_file)
    
    from .video.command_builders import build_mux_command
    from .command_jobs import MuxJob
    cmd = build_mux_command(video_track, audio_tracks, output_file)
        
    try:
        job = MuxJob(cmd)
        job.execute()
        
        # Validate AV sync in muxed output
        sync_threshold = 0.1  # allowed difference in seconds

        vid_result = run_cmd([
            "ffprobe", "-v", "error",
            "-select_streams", "v:0",
            "-show_entries", "stream=start_time,duration",
            "-of", "json",
            str(output_file)
        ])
        vid_data = json.loads(vid_result.stdout)
        if not vid_data.get("streams"):
            logger.error("Muxed output: No video stream found")
            return False
        video_start = float(vid_data["streams"][0].get("start_time") or 0)

        aud_result = run_cmd([
            "ffprobe", "-v", "error",
            "-select_streams", "a:0",
            "-show_entries", "stream=start_time,duration",
            "-of", "json",
            str(output_file)
        ])
        aud_data = json.loads(aud_result.stdout)
        if not aud_data.get("streams"):
            logger.error("Muxed output: No audio stream found")
            return False
        audio_start = float(aud_data["streams"][0].get("start_time") or 0)

        if abs(video_start - audio_start) > sync_threshold:
            logger.error("Muxed output AV sync error: video_start=%.2fs, audio_start=%.2fs", 
                     video_start, audio_start)
            return False

        return True
    except Exception as e:
        logger.error("Muxing failed: %s", e)
        return False
