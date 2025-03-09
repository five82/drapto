"""
ffprobe_utils.py

Helper functions to query ffprobe and return parsed data.
"""

import subprocess
import json
import logging
from pathlib import Path
from typing import Dict, Any, Union, Literal, Generator, Tuple
from functools import lru_cache
from contextlib import contextmanager
from .utils import run_cmd

logger = logging.getLogger(__name__)

class FFProbeSession:
    """Manages a session of ffprobe queries for a single file"""
    def __init__(self, path: Path):
        self.path = path
        self._cache = {}

    def get(self, property_name: str, stream_type: str = "video", stream_index: int = 0) -> Any:
        """Get a property, caching the result"""
        if stream_type == "format":
            cache_key = (property_name, "format", 0)
        else:
            cache_key = (property_name, stream_type, stream_index)
        if cache_key not in self._cache:
            self._cache[cache_key] = get_media_property(
                self.path, stream_type, property_name, stream_index
            )
        return self._cache[cache_key]

@contextmanager
def probe_session(path: Path) -> Generator[FFProbeSession, None, None]:
    """Context manager for probe sessions"""
    try:
        session = FFProbeSession(path)
        yield session
    except MetadataError as e:
        logger.error("Probe session failed: %s", e)
        raise

class MetadataError(Exception):
    """Raised when metadata cannot be retrieved or parsed"""
    def __init__(self, message: str, property_name: str = None):
        self.property_name = property_name
        super().__init__(f"Metadata error: {message}")

@lru_cache(maxsize=100)
def ffprobe_query(path: Path, args: tuple) -> Dict[str, Any]:  # Enforce tuple type
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
    cmd = ["ffprobe", "-v", "error"] + list(args) + [str(path)]
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, check=True)
        return json.loads(result.stdout)
    except (subprocess.CalledProcessError, json.JSONDecodeError) as e:
        raise MetadataError(f"Failed to query ffprobe: {str(e)}") from e

def get_media_property(
    path: Path,
    stream_type: Literal["video", "audio", "subtitle"],
    property_name: str,
    stream_index: int = 0
) -> Union[float, int, str]:
    """
    Unified metadata fetcher with error handling and type casting.
    
    Args:
        path: Path to media file
        stream_type: Type of stream ("video", "audio", or "subtitle")
        property_name: Name of the property to fetch
        stream_index: Stream index (default 0)
        
    Returns:
        Property value with appropriate type casting
        
    Raises:
        MetadataError: If property cannot be retrieved or parsed
    """
    if stream_type == "format":
        args = (
            "-show_entries", f"format={property_name}",
            "-of", "default=noprint_wrappers=1:nokey=1"
        )
    else:
        type_prefix = stream_type[0]  # v for video, a for audio, s for subtitle
        args = (
            "-select_streams", f"{type_prefix}:{stream_index}",
            "-show_entries", f"stream={property_name}",
            "-of", "default=noprint_wrappers=1:nokey=1"
        )
    
    try:
        result = run_cmd(["ffprobe", "-v", "error"] + list(args) + [str(path)])
        value = result.stdout.strip()
        
        # Handle empty results
        if not value or value.lower() in ["n/a", "nan"]:
            raise MetadataError(f"No valid value found for {property_name}", property_name)
            
        # Type casting based on property
        try:
            if property_name in ["duration", "start_time"]:
                return float(value)
            elif property_name in ["width", "height", "channels"]:
                return int(value)
            return value
        except ValueError as e:
            raise MetadataError(f"Could not convert {value} to required type", property_name) from e
        
    except Exception as e:
        raise MetadataError(
            f"Could not get {property_name}: {str(e)}", 
            property_name
        ) from e

def get_video_info(path: Path) -> Dict[str, Any]:
    """
    Retrieve key video stream information from the file.
    
    Args:
        path: Path to video file.
    
    Returns:
        Dictionary with video stream info (codec, width, height, duration, etc.)
    """
    properties = [
        "codec_name", "width", "height", "start_time", "duration",
        "pix_fmt", "r_frame_rate", "color_transfer", "color_primaries",
        "color_space"
    ]
    
    info = {}
    try:
        # Use a single probe session to batch all property queries
        with probe_session(path) as probe:
            for prop in properties:
                try:
                    info[prop] = probe.get(prop, "video")
                except MetadataError:
                    info[prop] = None
                    logger.debug("Property %s not found in video stream", prop)
        return info
    except MetadataError as e:
        logger.warning("Failed to get video info: %s", e)
        return {prop: None for prop in properties}

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
    args = (
        "-show_entries", "format=duration,size",
        "-of", "json"
    )
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
    args = (
        "-select_streams", "s",
        "-show_entries", "stream=index",
        "-of", "json"
    )
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
    args = (
        "-select_streams", "a",
        "-show_entries", "stream=codec_name,channels,bit_rate,start_time,duration",
        "-of", "json"
    )
    data = ffprobe_query(path, args)
    return data.get("streams", [])

def get_first_stream_timing(path: Path, stream_type: str) -> tuple[float, float]:
    """Get start time and duration for first stream of type"""
    return (
        get_media_property(path, stream_type, "start_time"),
        get_media_property(path, stream_type, "duration")
    )

def get_duration(
    path: Path, 
    stream_type: str = "video",
    stream_index: int = 0
) -> float:
    """Get duration from stream with fallback to format duration."""
    try:
        duration = get_media_property(path, stream_type, "duration", stream_index)
        if duration <= 0:
            raise MetadataError("Invalid duration value")
        return duration
    except MetadataError:
        try:
            format_duration = get_media_property(path, "format", "duration")
            if format_duration <= 0:
                raise MetadataError("Invalid format duration")
            return format_duration
        except MetadataError as e:
            raise MetadataError(f"No valid duration found: {str(e)}") from e

def get_resolution(path: Path) -> Tuple[int, int]:
    """Get video resolution (width, height)."""
    try:
        return (
            get_media_property(path, "video", "width"),
            get_media_property(path, "video", "height")
        )
    except MetadataError as e:
        raise MetadataError(f"Failed to get resolution: {str(e)}") from e

def get_audio_channels(path: Path, track_index: int = 0) -> int:
    """Get number of audio channels for specified track."""
    try:
        return get_media_property(path, "audio", "channels", track_index)
    except MetadataError as e:
        raise MetadataError(f"Failed to get audio channels: {str(e)}") from e
