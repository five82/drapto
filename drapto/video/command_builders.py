"""Helper functions for building ffmpeg commands"""

import logging
from pathlib import Path
from typing import List, Optional

log = logging.getLogger(__name__)

def build_segment_command(
    input_file: Path,
    segments_dir: Path,
    scene_times: List[float],
    hw_opt: Optional[str] = None
) -> List[str]:
    """Build ffmpeg command for video segmentation"""
    cmd = ["ffmpeg", "-hide_banner", "-loglevel", "warning"]
    
    if hw_opt:
        cmd.extend(hw_opt.split())
        
    segment_times = ",".join(f"{t:.2f}" for t in scene_times if t > 1.0)
    cmd.extend([
        "-i", str(input_file),
        "-c:v", "copy",
        "-an",
        "-f", "segment",
        "-segment_times", segment_times,
        "-reset_timestamps", "1",
        str(segments_dir / "%04d.mkv")
    ])
    
    return cmd

def build_audio_encode_command(
    input_file: Path,
    output_file: Path,
    track_index: int,
    bitrate: str,
) -> List[str]:
    """Build ffmpeg command for audio encoding"""
    return [
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

def build_mux_command(
    video_track: Path,
    audio_tracks: List[Path],
    output_file: Path
) -> List[str]:
    """Build ffmpeg command for muxing tracks"""
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
    
    return cmd

def build_concat_command(
    segments: List[Path],
    output_file: Path,
    concat_file: Path
) -> List[str]:
    """Build ffmpeg command for concatenating segments"""
    return [
        "ffmpeg", "-hide_banner", "-loglevel", "error",
        "-fflags", "+genpts",
        "-f", "concat",
        "-safe", "0",
        "-i", str(concat_file),
        "-c", "copy",
        "-y", str(output_file)
    ]
