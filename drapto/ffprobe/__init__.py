"""FFProbe utilities for media file analysis

This package provides utilities for:
- Executing ffprobe commands and parsing results
- Managing probe sessions for efficient querying
- Extracting high-level media properties
"""

from .exec import MetadataError, ffprobe_query, get_media_property
from .session import FFProbeSession, probe_session
from .media import (
    get_video_info, get_audio_info, get_format_info,
    get_subtitle_info, get_all_audio_info, get_duration,
    get_resolution, get_audio_channels
)

__all__ = [
    'MetadataError',
    'ffprobe_query',
    'get_media_property',
    'FFProbeSession',
    'probe_session',
    'get_video_info',
    'get_audio_info', 
    'get_format_info',
    'get_subtitle_info',
    'get_all_audio_info',
    'get_duration',
    'get_resolution',
    'get_audio_channels'
]
"""FFProbe utilities for media file analysis"""

from .utils import *
