"""Custom exceptions for drapto encoding pipeline"""

class DraptoError(Exception):
    """
    Base exception for all drapto errors.

    Attributes:
        message (str): A description of the error.
        module (str): The module where the error originated.

    Args:
        message (str): Explanation of the error.
        module (str, optional): Name of the module in which the error occurred.

    Usage:
        raise DraptoError("An error occurred", module="segmentation")
    """
    def __init__(self, message: str, module: str = None):
        self.message = message
        self.module = module
        super().__init__(f"[{module or 'unknown'}] {message}")

class DependencyError(DraptoError):
    """
    Exception raised when one or more required dependencies for the drapto pipeline are missing.

    This error indicates that the system is missing external tools (e.g., ffmpeg, ffprobe, mediainfo)
    required for proper operation.
    """
    pass

class EncodingError(DraptoError):
    """
    Exception raised when video encoding fails.

    This error is thrown if any stage in the video encoding process encounters a failure.
    """
    pass

class ValidationError(DraptoError):
    """
    Exception raised when output validation fails.

    Use this exception to indicate that the final encoded output did not meet the required quality
    or structural expectations.
    """
    pass

class ConcatenationError(DraptoError):
    """
    Exception raised when segment concatenation fails.

    This error is raised if the process of merging encoded segments fails or produces an invalid output.
    """
    pass

class SegmentEncodingError(DraptoError):
    """
    Exception raised when individual segment encoding fails.

    This error is specifically thrown when a video segment cannot be encoded successfully, even after retrying.
    """
    pass

class CommandExecutionError(DraptoError):
    """
    Exception raised when a subprocess command fails during execution.

    Indicates an error in executing an external command (e.g., ffmpeg or ffprobe) through subprocess.
    """
    pass

class SegmentMergeError(EncodingError):
    """
    Exception raised when merging of short video segments fails.

    This error signals that the output from the segment merge process is missing or invalid.
    """
    pass

class HardwareAccelError(DraptoError):
    """
    Exception raised when hardware acceleration configuration fails.

    Use this error to indicate issues with detecting or configuring hardware decoding/acceleration.
    """
    pass

class AudioEncodingError(DraptoError):
    """
    Exception raised when audio encoding fails.

    This error is thrown if the process of encoding an audio track encounters any issue.
    """
    pass

class MuxingError(DraptoError):
    """
    Exception raised when muxing of audio and video tracks fails.

    This error indicates that combining multiple media streams into a final output file did not succeed.
    """
    pass

class SegmentationError(DraptoError):
    """
    Exception raised when video segmentation fails.

    This error is raised if the process of splitting the video into segments based on scene detection or other
    criteria fails.
    """
    pass
