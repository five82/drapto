"""Core package for drapto.

This package provides the core interfaces and base classes for the drapto video encoder.
"""

from drapto.core.encoder import Encoder, EncodingOptions
from drapto.core.config import DraptoConfig, ConfigSchema
from drapto.core.exceptions import (
    DraptoError,
    ConfigError,
    ValidationError,
    EncodingError,
    MediaError,
    SystemError,
    ResourceError,
    StateError,
    ProcessError,
    TemporaryFileError,
    UnsupportedFormatError,
    HardwareAccelError,
)

__all__ = [
    # Encoder interfaces
    'Encoder',
    'EncodingOptions',
    
    # Configuration
    'DraptoConfig',
    'ConfigSchema',
    
    # Exceptions
    'DraptoError',
    'ConfigError',
    'ValidationError',
    'EncodingError',
    'MediaError',
    'SystemError',
    'ResourceError',
    'StateError',
    'ProcessError',
    'TemporaryFileError',
    'UnsupportedFormatError',
    'HardwareAccelError',
]
