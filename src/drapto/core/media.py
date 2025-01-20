"""Media file handling interfaces for drapto.

This module defines the core interfaces for handling media files,
including video, audio, and subtitle stream processing.
"""

from abc import ABC, abstractmethod
from dataclasses import dataclass
from enum import Enum, auto
from pathlib import Path
from typing import Dict, List, Optional, Any

class StreamType(Enum):
    """Types of media streams."""
    VIDEO = auto()
    AUDIO = auto()
    SUBTITLE = auto()

@dataclass
class StreamInfo:
    """Information about a media stream.
    
    Attributes:
        index: Stream index in the container
        type: Type of stream (video/audio/subtitle)
        codec: Codec name
        language: Stream language code
        title: Stream title
        default: Whether this is the default stream
        forced: Whether this is a forced stream
        metadata: Additional stream metadata
    """
    index: int
    type: StreamType
    codec: str
    language: Optional[str] = None
    title: Optional[str] = None
    default: bool = False
    forced: bool = False
    metadata: Dict[str, str] = None
    
    def __post_init__(self) -> None:
        """Initialize default values."""
        if self.metadata is None:
            self.metadata = {}

@dataclass
class VideoStreamInfo(StreamInfo):
    """Video stream specific information.
    
    Attributes:
        width: Video width in pixels
        height: Video height in pixels
        fps: Frames per second
        bitrate: Bitrate in bits per second
        pixel_format: Pixel format
        color_space: Color space information
        hdr: Whether stream contains HDR metadata
    """
    width: int = 0
    height: int = 0
    fps: float = 0.0
    bitrate: Optional[int] = None
    pixel_format: Optional[str] = None
    color_space: Optional[str] = None
    hdr: bool = False

@dataclass
class AudioStreamInfo(StreamInfo):
    """Audio stream specific information.
    
    Attributes:
        channels: Number of audio channels
        sample_rate: Sample rate in Hz
        bitrate: Bitrate in bits per second
    """
    channels: int = 0
    sample_rate: int = 0
    bitrate: Optional[int] = None

class MediaFile(ABC):
    """Interface for media file operations."""
    
    @abstractmethod
    def open(self, path: Path) -> None:
        """Open and analyze a media file.
        
        Args:
            path: Path to media file
            
        Raises:
            FileNotFoundError: If file doesn't exist
            MediaError: If file analysis fails
        """
        pass
    
    @abstractmethod
    def get_streams(self, type_filter: Optional[StreamType] = None) -> List[StreamInfo]:
        """Get information about media streams.
        
        Args:
            type_filter: Optional filter for stream type
            
        Returns:
            List of stream information objects
        """
        pass
    
    @abstractmethod
    def get_duration(self) -> float:
        """Get media duration in seconds.
        
        Returns:
            Duration in seconds
            
        Raises:
            MediaError: If duration cannot be determined
        """
        pass
    
    @abstractmethod
    def get_format_info(self) -> Dict[str, Any]:
        """Get container format information.
        
        Returns:
            Dictionary of format information
        """
        pass
    
    @abstractmethod
    def close(self) -> None:
        """Close the media file and free resources."""
        pass
    
    def __enter__(self) -> 'MediaFile':
        """Context manager entry."""
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb) -> None:
        """Context manager exit."""
        self.close() 