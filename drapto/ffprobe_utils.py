"""FFProbe utilities for media file analysis (DEPRECATED)

This module is deprecated and will be removed in a future release.
All functionality has been moved to the ffprobe submodules:
- drapto.ffprobe.exec
- drapto.ffprobe.session
- drapto.ffprobe.media

Please update your imports to use the new modules directly.
"""

from .ffprobe.exec import MetadataError, ffprobe_query, get_media_property
from .ffprobe.session import probe_session, FFProbeSession
from .ffprobe.media import (
    get_video_info, get_audio_info, get_format_info,
    get_subtitle_info, get_all_audio_info, get_duration,
    get_resolution, get_audio_channels
)

__all__ = [
    "MetadataError", "ffprobe_query", "get_media_property",
    "probe_session", "FFProbeSession", "get_video_info",
    "get_audio_info", "get_format_info", "get_subtitle_info",
    "get_all_audio_info", "get_duration", "get_resolution",
    "get_audio_channels"
]
