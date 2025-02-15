"""Handles muxing of video and audio streams"""

import logging
from pathlib import Path
from typing import List, Optional

from .utils import run_cmd

log = logging.getLogger(__name__)

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
    log.info("Muxing tracks to: %s", output_file)
    
    # Build ffmpeg command
    cmd = ["ffmpeg", "-hide_banner", "-loglevel", "warning"]
    
    # Add video input
    cmd.extend(["-i", str(video_track)])
    
    # Add audio inputs
    for audio_track in audio_tracks:
        cmd.extend(["-i", str(audio_track)])
    
    # Add mapping
    cmd.extend(["-map", "0:v:0"])  # Video track
    for i in range(len(audio_tracks)):
        cmd.extend(["-map", f"{i+1}:a:0"])  # Audio tracks
    
    # Add output file
    cmd.extend(["-c", "copy", "-y", str(output_file)])
    
    try:
        run_cmd(cmd)
        return True
    except Exception as e:
        log.error("Muxing failed: %s", e)
        return False
