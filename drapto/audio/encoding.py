"""Audio encoding functions for drapto"""

import logging
from pathlib import Path
from typing import List, Optional

from ..config import WORKING_DIR
from ..utils import run_cmd

log = logging.getLogger(__name__)

def encode_audio_tracks(input_file: Path) -> Optional[List[Path]]:
    """
    Encode all audio tracks from input file using libopus
    
    Args:
        input_file: Path to input video file
        
    Returns:
        Optional[List[Path]]: List of encoded audio track paths if successful
    """
    try:
        # Get number of audio tracks
        result = run_cmd([
            "ffprobe", "-v", "error",
            "-select_streams", "a",
            "-show_entries", "stream=index",
            "-of", "csv=p=0",
            str(input_file)
        ])
        num_tracks = len(result.stdout.strip().split('\n'))
        
        if num_tracks == 0:
            log.warning("No audio tracks found")
            return []
            
        encoded_tracks = []
        for track_idx in range(num_tracks):
            output_track = encode_audio_track(input_file, track_idx)
            if output_track:
                encoded_tracks.append(output_track)
            else:
                log.error("Failed to encode audio track %d", track_idx)
                return None
                
        return encoded_tracks
        
    except Exception as e:
        log.error("Failed to process audio tracks: %s", e)
        return None

def encode_audio_track(input_file: Path, track_index: int) -> Optional[Path]:
    """
    Encode a single audio track using libopus
    
    Args:
        input_file: Path to input video file
        track_index: Index of audio track to encode
        
    Returns:
        Optional[Path]: Path to encoded audio file if successful
    """
    output_file = WORKING_DIR / f"audio-{track_index}.mkv"
    
    try:
        # Get number of channels
        result = run_cmd([
            "ffprobe", "-v", "error",
            "-select_streams", f"a:{track_index}",
            "-show_entries", "stream=channels",
            "-of", "csv=p=0",
            str(input_file)
        ])
        num_channels = int(result.stdout.strip())
        
        # Determine bitrate based on channel count
        if num_channels == 1:
            bitrate = "64k"
            layout = "mono"
        elif num_channels == 2:
            bitrate = "128k"
            layout = "stereo"
        elif num_channels == 6:
            bitrate = "256k"
            layout = "5.1"
        elif num_channels == 8:
            bitrate = "384k"
            layout = "7.1"
        else:
            bitrate = f"{num_channels * 48}k"
            layout = "custom"
            
        log.info(
            "Configuring audio track %d: %d channels, %s layout, %s bitrate",
            track_index, num_channels, layout, bitrate
        )
        
        # Encode audio track
        cmd = [
            "ffmpeg", "-hide_banner", "-loglevel", "warning",
            "-i", str(input_file),
            "-map", f"0:a:{track_index}",
            "-c:a", "libopus",
            "-af", "aformat=channel_layouts=7.1|5.1|stereo|mono",
            "-application", "audio",
            "-vbr", "on",
            "-compression_level", "10",
            "-frame_duration", "20",
            "-b:a", bitrate,
            "-avoid_negative_ts", "make_zero",
            "-y", str(output_file)
        ]
        run_cmd(cmd)
        
        return output_file
        
    except Exception as e:
        log.error("Failed to encode audio track %d: %s", track_index, e)
        return None
