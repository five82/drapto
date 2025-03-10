"""
Video Segmentation Package

Responsibilities:
- Coordinate scene detection and segmentation
- Manage segment validation and merging
- Handle segment boundary analysis
"""

from .segmentation_main import segment_video
from .validation import validate_single_segment, validate_encoded_segments

__all__ = [
    "segment_video",
    "validate_single_segment",
    "validate_encoded_segments"
]
