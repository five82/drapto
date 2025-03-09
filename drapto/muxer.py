"""Handles muxing of video and audio streams"""

import json
import logging
from pathlib import Path
from typing import List, Optional

from .utils import run_cmd
from .exceptions import MuxingError
from .ffprobe_utils import get_media_property

logger = logging.getLogger(__name__)

def mux_tracks(
    video_track: Path,
    audio_tracks: List[Path],
    output_file: Path
) -> None:
    """Mux video and audio tracks into final output file"""
    logger.info("Muxing tracks to: %s", output_file)
    
    from .video.command_builders import build_mux_command
    from .command_jobs import MuxJob
    
    try:
        cmd = build_mux_command(video_track, audio_tracks, output_file)
        job = MuxJob(cmd)
        job.execute()
        
        # Validate AV sync in muxed output using probe session
        sync_threshold = 0.1  # allowed difference in seconds

        try:
            with probe_session(output_file) as probe:
                video_start = probe.get("start_time", "video")
                video_duration = probe.get("duration", "video") 
                audio_start = probe.get("start_time", "audio")
                audio_duration = probe.get("duration", "audio")

            start_diff = abs(video_start - audio_start)
            duration_diff = abs(video_duration - audio_duration)
            
            if start_diff > sync_threshold or duration_diff > sync_threshold:
                raise MuxingError(
                    f"AV sync issue detected: video_start={video_start:.2f}s vs audio_start={audio_start:.2f}s; "
                    f"video_duration={video_duration:.2f}s vs audio_duration={audio_duration:.2f}s",
                    module="muxer"
                )

        except MetadataError as e:
            logger.error("Failed to validate AV sync: %s", e)
            raise MuxingError(f"AV sync validation failed: {str(e)}", module="muxer") from e
            
    except Exception as e:
        logger.error("Muxing failed: %s", e)
        raise MuxingError(f"Muxing failed: {str(e)}", module="muxer") from e
