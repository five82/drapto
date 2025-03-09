"""Handles muxing of video and audio streams"""

import json
import logging
from pathlib import Path
from typing import List, Optional

from .utils import run_cmd
from .exceptions import MuxingError
from .ffprobe_utils import get_media_property, probe_session, MetadataError

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
                # Get video timing with format fallback
                try:
                    video_start = probe.get("start_time", "video")
                    video_duration = probe.get("duration", "video")
                except MetadataError:
                    video_start = 0.0
                    video_duration = probe.get("duration", "format")
                    logger.warning("Using container duration for video validation")

                # Get audio timing with format fallback
                try:
                    audio_start = probe.get("start_time", "audio")
                    audio_duration = probe.get("duration", "audio")
                except MetadataError:
                    audio_start = 0.0
                    audio_duration = probe.get("duration", "format")
                    logger.warning("Using container duration for audio validation")

            start_diff = abs(video_start - audio_start)
            duration_diff = abs(video_duration - audio_duration)
            
            # Increase sync thresholds for container durations
            max_sync_threshold = 0.5 if any([isinstance(video_duration, float), isinstance(audio_duration, float)]) else 0.2
            if start_diff > max_sync_threshold or duration_diff > max_sync_threshold:
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
