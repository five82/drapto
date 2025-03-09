"""Audio encoding functions for drapto"""

import logging
from pathlib import Path
from typing import List, Optional

logger = logging.getLogger(__name__)

from ..config import WORKING_DIR
from ..utils import run_cmd, run_cmd_with_progress
from ..formatting import print_info
from ..exceptions import AudioEncodingError
from ..ffprobe_utils import get_all_audio_info, get_media_property, probe_session, MetadataError

def encode_audio_tracks(input_file: Path) -> List[Path]:
    """
    Encode all audio tracks from input file using libopus
    
    Returns:
        List[Path]: List of encoded audio track paths
        
    Raises:
        AudioEncodingError: If encoding fails
    """
    try:
        # Get number of audio tracks from ffprobe_utils
        audio_info = get_all_audio_info(input_file)
        num_tracks = len(audio_info)
        
        if num_tracks == 0:
            logger.warning("No audio tracks found")
            return []
            
        encoded_tracks = []
        for track_idx in range(num_tracks):
            try:
                output_track = encode_audio_track(input_file, track_idx)
                encoded_tracks.append(output_track)
            except Exception as e:
                raise AudioEncodingError(
                    f"Failed to encode audio track {track_idx}: {str(e)}",
                    module="audio_encoding"
                ) from e
                
        return encoded_tracks
        
    except Exception as e:
        raise AudioEncodingError(
            f"Audio processing failed: {str(e)}", 
            module="audio_encoding"
        ) from e

def encode_audio_track(input_file: Path, track_index: int) -> Path:
    """
    Encode a single audio track using libopus
    
    Returns:
        Path: Path to encoded audio file
        
    Raises:
        AudioEncodingError: If encoding fails
    """
    output_file = WORKING_DIR / f"audio-{track_index}.mkv"
    
    try:
        # Get number of channels
        num_channels = int(get_media_property(input_file, "audio", "channels", track_index))
        
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
            
        print_info(
            f"Configuring audio track {track_index}:\n"
            f"Channels: {num_channels}\n"
            f"Layout:   {layout}\n"
            f"Bitrate:  {bitrate}"
        )
        
        from ..video.command_builders import build_audio_encode_command
        from ..command_jobs import AudioEncodeJob
        cmd = build_audio_encode_command(input_file, output_file, track_index, bitrate)
        formatted_cmd = " \\\n    ".join(cmd)
        logger.info("Audio encoding command for track %d:\n%s", track_index, formatted_cmd)
        # Get audio duration for progress reporting
        try:
            with probe_session(input_file) as probe:
                try:
                    audio_duration = float(probe.get("duration", "audio", track_index))
                except MetadataError:
                    audio_duration = float(probe.get("duration", "format"))
                    logger.warning("Using container duration for audio progress reporting")
        except MetadataError as e:
            logger.error("Could not get audio duration: %s", e)
            audio_duration = None

        job = AudioEncodeJob(cmd)
        job.execute(total_duration=audio_duration, log_interval=5.0)
        
        return output_file
        
    except Exception as e:
        logger.error("Failed to encode audio track %d: %s", track_index, e)
        raise AudioEncodingError(
            f"Audio track {track_index} encoding failed: {str(e)}",
            module="audio_encoding"
        ) from e
