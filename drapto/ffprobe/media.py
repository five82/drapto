"""High-level media property extraction

Responsibilities:
- Extract and validate video/audio/subtitle stream properties
- Handle duration calculations with fallbacks
- Provide stream-specific metadata retrieval
"""

import logging
from pathlib import Path
from typing import Dict, Any, List, Tuple

from .exec import ffprobe_query, get_media_property, MetadataError
from .session import probe_session

logger = logging.getLogger(__name__)

def get_video_info(path: Path) -> Dict[str, Any]:
    """Get key video stream information"""
    properties = [
        "codec_name", "width", "height", "start_time", "duration",
        "pix_fmt", "r_frame_rate", "color_transfer", "color_primaries",
        "color_space"
    ]
    
    info = {}
    try:
        with probe_session(path) as probe:
            for prop in properties:
                try:
                    info[prop] = probe.get(prop, "video")
                except MetadataError:
                    info[prop] = None
        return info
    except MetadataError as e:
        logger.warning("Failed to get video info: %s", e)
        return {prop: None for prop in properties}

def get_audio_info(path: Path, stream_index: int = 0) -> Dict[str, Any]:
    """Get key audio stream information"""
    props = ["codec_name", "channels", "bit_rate", "start_time", "duration"]
    info = {}
    try:
        with probe_session(path) as probe:
            for prop in props:
                try:
                    info[prop] = probe.get(prop, "audio", stream_index)
                except MetadataError:
                    info[prop] = None
        return info
    except MetadataError as e:
        logger.warning("Failed to get audio info: %s", e)
        return {prop: None for prop in props}

def get_format_info(path: Path) -> Dict[str, Any]:
    """Get format-wide information"""
    args = (
        "-show_entries", "format=duration,size",
        "-of", "json"
    )
    data = ffprobe_query(path, args)
    return data.get("format", {})

def get_subtitle_info(path: Path) -> Dict[str, Any]:
    """Get subtitle stream information"""
    args = (
        "-select_streams", "s",
        "-show_entries", "stream=index",
        "-of", "json"
    )
    data = ffprobe_query(path, args)
    return data

def get_all_audio_info(path: Path) -> list:
    """Get information for all audio streams"""
    args = (
        "-select_streams", "a",
        "-show_entries", "stream=codec_name,channels,bit_rate,start_time,duration",
        "-of", "json"
    )
    data = ffprobe_query(path, args)
    return data.get("streams", [])

def get_duration(path: Path, stream_type: str = "video", stream_index: int = 0) -> float:
    """Get duration with multiple fallback methods"""
    try:
        duration = get_media_property(path, stream_type, "duration", stream_index)
        if duration > 0:
            return duration
        raise MetadataError("Invalid duration value")
    except MetadataError:
        format_duration = get_media_property(path, "format", "duration")
        if format_duration > 0:
            return format_duration
            
        # Try fallback methods...
        try:
            nb_frames = get_media_property(path, stream_type, "nb_frames", stream_index)
            time_base = get_media_property(path, stream_type, "time_base", stream_index)
            
            if nb_frames and time_base:
                numerator, denominator = map(float, time_base.split('/'))
                return nb_frames * numerator / denominator
        except (MetadataError, ValueError):
            pass

        try:
            bit_rate = float(get_media_property(path, stream_type, "bit_rate", stream_index))
            stream_size = get_media_property(path, stream_type, "size", stream_index)
            
            if bit_rate > 0 and stream_size > 0:
                return (stream_size * 8) / bit_rate
        except (MetadataError, ValueError):
            pass

        try:
            args = (
                "-select_streams", f"{stream_type[0]}:{stream_index}",
                "-show_entries", "packet=duration_time",
                "-of", "json"
            )
            data = ffprobe_query(path, args)
            total = sum(float(p["duration_time"]) for p in data.get("packets", []))
            return round(total, 3)
        except Exception as e:
            raise MetadataError(f"All duration methods failed: {str(e)}") from e

def get_resolution(path: Path) -> Tuple[int, int]:
    """Get video resolution"""
    try:
        return (
            get_media_property(path, "video", "width"),
            get_media_property(path, "video", "height")
        )
    except MetadataError as e:
        raise MetadataError(f"Failed to get resolution: {str(e)}") from e

def get_audio_channels(path: Path, track_index: int = 0) -> int:
    """Get number of audio channels"""
    try:
        return get_media_property(path, "audio", "channels", track_index)
    except MetadataError as e:
        raise MetadataError(f"Failed to get audio channels: {str(e)}") from e
