"""Custom exceptions for drapto.

This module defines all custom exceptions used throughout the drapto package.
"""

class DraptoError(Exception):
    """Base exception class for all drapto errors."""
    pass

class ConfigError(DraptoError):
    """Raised when there is a configuration error."""
    pass

class ValidationError(DraptoError):
    """Raised when input validation fails."""
    pass

class EncodingError(DraptoError):
    """Raised when there is an error during encoding."""
    def __init__(self, message: str, ffmpeg_output: str = "") -> None:
        """Initialize encoding error.
        
        Args:
            message: Error message
            ffmpeg_output: FFmpeg output for debugging
        """
        super().__init__(message)
        self.ffmpeg_output = ffmpeg_output

class MediaError(DraptoError):
    """Raised when there is an error processing media files."""
    pass

class SystemError(DraptoError):
    """Raised when there is an error with system operations."""
    pass

class ResourceError(DraptoError):
    """Raised when there is an error with system resources."""
    pass

class StateError(DraptoError):
    """Raised when there is an error with state management."""
    pass

class ProcessError(DraptoError):
    """Raised when there is an error with process management."""
    def __init__(self, message: str, exit_code: int = 0, output: str = "") -> None:
        """Initialize process error.
        
        Args:
            message: Error message
            exit_code: Process exit code
            output: Process output for debugging
        """
        super().__init__(message)
        self.exit_code = exit_code
        self.output = output

class TemporaryFileError(DraptoError):
    """Raised when there is an error with temporary file operations."""
    pass

class UnsupportedFormatError(DraptoError):
    """Raised when an unsupported format is encountered."""
    pass

class HardwareAccelError(DraptoError):
    """Raised when there is an error with hardware acceleration."""
    pass 