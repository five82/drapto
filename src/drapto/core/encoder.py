"""Core encoder interface and base classes for drapto.

This module defines the base encoder interface and common encoding options
that all encoder implementations must support.
"""

from abc import ABC, abstractmethod
from dataclasses import dataclass
from pathlib import Path
from typing import Optional, Dict, Any

@dataclass
class EncodingOptions:
    """Encoding options that apply to all encoders.
    
    Attributes:
        preset: SVT-AV1 encoding preset (0-13, lower is higher quality but slower)
        pix_fmt: Output pixel format
        crop_filter: Optional FFmpeg crop filter string
        hardware_accel: Whether to use hardware acceleration
        crf: Constant Rate Factor value for quality control
        max_muxing_queue_size: FFmpeg muxing queue size
    """
    preset: int = 6
    pix_fmt: str = "yuv420p10le"
    crop_filter: Optional[str] = None
    hardware_accel: bool = True
    crf: int = 30
    max_muxing_queue_size: int = 1024

    def __post_init__(self) -> None:
        """Validate encoding options after initialization."""
        if not 0 <= self.preset <= 13:
            raise ValueError("Preset must be between 0 and 13")
        if not 0 <= self.crf <= 63:
            raise ValueError("CRF must be between 0 and 63")

class Encoder(ABC):
    """Base encoder interface that all encoders must implement."""
    
    @abstractmethod
    def encode(
        self, 
        input_file: Path,
        output_file: Path,
        options: EncodingOptions
    ) -> bool:
        """Execute encoding process.
        
        Args:
            input_file: Path to input video file
            output_file: Path where encoded video should be written
            options: Encoding options to use
            
        Returns:
            bool: True if encoding was successful, False otherwise
            
        Raises:
            FileNotFoundError: If input file doesn't exist
            PermissionError: If output location isn't writable
            ValueError: If options are invalid
        """
        pass

    @abstractmethod
    def can_handle(self, input_file: Path) -> bool:
        """Check if encoder can handle the given input file.
        
        Args:
            input_file: Path to input video file to check
            
        Returns:
            bool: True if this encoder can handle the input, False otherwise
        """
        pass

    @abstractmethod
    def get_version_info(self) -> Dict[str, str]:
        """Get version information for the encoder and its dependencies.
        
        Returns:
            Dict[str, str]: Version information for encoder components
        """
        pass 