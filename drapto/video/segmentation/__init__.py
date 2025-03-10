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
"""Video segmentation package

This package provides scene-based video segmentation functionality including:
- Scene detection and analysis
- Segment boundary calculation
- Validation of segment boundaries
- Memory-aware parallel encoding
"""

from .segmentation_main import segment_video
from ..scene_detection_helpers import validate_segment_boundaries
from .validation import validate_encoded_segments

__all__ = [
    'segment_video',
    'validate_segment_boundaries',
    'validate_encoded_segments'
]
