"""
ffprobe_utils.py

Helper functions to query ffprobe and return parsed data.
"""

import subprocess
import json
from pathlib import Path
from typing import Dict, Any

def ffprobe_query(path: Path, args: list) -> Dict[str, Any]:
    """
    Run ffprobe with the specified arguments and return parsed JSON output.
    
    Args:
        path: Path to media file.
        args: List of additional ffprobe arguments.
    
    Returns:
        Parsed ffprobe JSON as a dictionary.
    
    Raises:
        subprocess.CalledProcessError if the command fails.
    """
    cmd = ["ffprobe", "-v", "error"] + args + [str(path)]
    result = subprocess.run(cmd, capture_output=True, text=True, check=True)
    return json.loads(result.stdout)

def get_video_info(path: Path) -> Dict[str, Any]:
    """
    Retrieve key video stream information from the file.
    
    Args:
        path: Path to video file.
    
    Returns:
        Dictionary with video stream info (codec, width, height, duration, etc.)
    """
    args = [
        "-select_streams", "v:0",
        "-show_entries", "stream=codec_name,width,height,start_time,duration,pix_fmt,r_frame_rate",
        "-of", "json"
    ]
    data = ffprobe_query(path, args)
    return data.get("streams", [{}])[0]

def get_audio_info(path: Path, stream_index: int = 0) -> Dict[str, Any]:
    """
    Retrieve key audio stream information.
    
    Args:
        path: Path to video file.
        stream_index: The audio stream index to query.
    
    Returns:
        Dictionary with audio stream info (codec, channels, bit_rate, start_time, duration, etc.)
    """
    args = [
        "-select_streams", f"a:{stream_index}",
        "-show_entries", "stream=codec_name,channels,bit_rate,start_time,duration",
        "-of", "json"
    ]
    data = ffprobe_query(path, args)
    return data.get("streams", [{}])[0]

def get_format_info(path: Path) -> Dict[str, Any]:
    """
    Retrieve format-wide information such as duration and file size.
    
    Args:
        path: Path to media file.
    
    Returns:
        Dictionary with format info.
    """
    args = [
        "-show_entries", "format=duration,size",
        "-of", "json"
    ]
    data = ffprobe_query(path, args)
    return data.get("format", {})

def get_subtitle_info(path: Path) -> Dict[str, Any]:
    """
    Retrieve subtitle stream information.
    
    Args:
        path: Path to media file.
    
    Returns:
        Dictionary with subtitle stream info.
    """
    args = [
        "-select_streams", "s",
        "-show_entries", "stream=index",
        "-of", "json"
    ]
    data = ffprobe_query(path, args)
    return data

def get_all_audio_info(path: Path) -> list:
    """
    Retrieve information for all audio streams.
    
    Args:
        path: Path to media file.
    
    Returns:
        List of dictionaries for each audio stream.
    """
    args = [
        "-select_streams", "a",
        "-show_entries", "stream=codec_name,channels,bit_rate,start_time,duration",
        "-of", "json"
    ]
    data = ffprobe_query(path, args)
    return data.get("streams", [])
