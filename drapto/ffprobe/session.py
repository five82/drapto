"""FFProbe session management

Responsibilities:
- Provide a session context for batched ffprobe queries
- Cache query results within a session
- Handle session cleanup and resource management
"""

import logging
from pathlib import Path
from typing import Any, Generator
from contextlib import contextmanager

from .exec import get_media_property, MetadataError

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
