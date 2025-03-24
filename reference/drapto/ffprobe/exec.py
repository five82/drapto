"""Low-level ffprobe command execution utilities

Responsibilities:
- Execute ffprobe commands and parse JSON output
- Handle command failures and JSON parsing errors
- Provide caching for repeated queries
- Define base exception types
"""

import subprocess
import json
import logging
from pathlib import Path
from typing import Dict, Any, Union, Tuple
from functools import lru_cache

logger = logging.getLogger(__name__)

class MetadataError(Exception):
    """Raised when metadata cannot be retrieved or parsed"""
    def __init__(self, message: str, property_name: str = None):
        self.property_name = property_name
        super().__init__(f"Metadata error: {message}")

@lru_cache(maxsize=100)
def ffprobe_query(path: Path, args: tuple) -> Dict[str, Any]:
    """
    Run ffprobe with the specified arguments and return parsed JSON output.
    
    Args:
        path: Path to media file.
        args: List of additional ffprobe arguments.
    
    Returns:
        Parsed ffprobe JSON as a dictionary.
    
    Raises:
        MetadataError if the command fails or output cannot be parsed.
    """
    cmd = ["ffprobe", "-v", "error"] + list(args) + [str(path)]
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, check=True)
        return json.loads(result.stdout)
    except (subprocess.CalledProcessError, json.JSONDecodeError) as e:
        raise MetadataError(f"Failed to query ffprobe: {str(e)}") from e

def get_media_property(
    path: Path,
    stream_type: str,
    property_name: str,
    stream_index: int = 0
) -> Union[float, int, str]:
    """
    Get a single media property with type conversion.
    
    Args:
        path: Path to media file
        stream_type: Type of stream ("video", "audio", "subtitle" or "format")
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
        result = subprocess.run(
            ["ffprobe", "-v", "error"] + list(args) + [str(path)],
            capture_output=True,
            text=True,
            check=True
        )
        value = result.stdout.strip()
        
        if not value or value.lower() in ["n/a", "nan"]:
            raise MetadataError(f"No valid value found for {property_name}", property_name)
            
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
