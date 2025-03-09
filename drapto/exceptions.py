"""Custom exceptions for drapto encoding pipeline"""

class DraptoError(Exception):
    """Base exception for all drapto errors"""
    def __init__(self, message: str, module: str = None):
        self.message = message
        self.module = module or "unknown"
        super().__init__(f"[{self.module}] {self.message}")

class EncodingError(DraptoError):
    """Base class for encoding-related errors"""
    def __init__(self, message: str, module: str = None):
        super().__init__(f"Encoding error: {message}", module)

class SegmentEncodingError(EncodingError):
    """Error during segment encoding"""

class ConcatenationError(EncodingError):
    """Error during segment concatenation"""

class ValidationError(DraptoError):
    """Base class for validation errors"""
    def __init__(self, message: str, module: str = None):
        super().__init__(f"Validation error: {message}", module)

class ConfigurationError(DraptoError):
    """Error in configuration/setup"""

class DependencyError(DraptoError):
    """Missing required dependencies"""

class MemoryError(DraptoError):
    """Insufficient memory for operation"""
