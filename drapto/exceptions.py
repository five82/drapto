"""Custom exceptions for drapto encoding pipeline"""

class DraptoError(Exception):
    """Base exception for all drapto errors"""
    def __init__(self, message: str, module: str = None):
        self.message = message
        self.module = module
        super().__init__(f"[{module or 'unknown'}] {message}")

class DependencyError(DraptoError):
    """Raised when required dependencies are missing"""
    pass

class EncodingError(DraptoError):
    """Raised when video encoding fails"""
    pass

class ValidationError(DraptoError):
    """Raised when output validation fails"""
    pass

class ConcatenationError(DraptoError):
    """Raised when segment concatenation fails"""
    pass

class SegmentEncodingError(DraptoError):
    """Raised when segment encoding fails"""
    pass

class CommandExecutionError(DraptoError):
    """Raised when a subprocess command fails"""
    pass

class SegmentMergeError(EncodingError):
    """Raised when merging short segments fails"""
    pass

class HardwareAccelError(DraptoError):
    """Raised when hardware acceleration setup fails"""
    pass

class AudioEncodingError(DraptoError):
    """Raised when audio encoding fails"""
    pass

class MuxingError(DraptoError):
    """Raised when muxing audio/video tracks fails"""
    pass

class SegmentationError(DraptoError):
    """Raised when video segmentation fails"""
    pass
