"""Segment validation utilities

Responsibilities:
- Validate individual encoded segments
- Check segment durations and properties
- Compare segments against original input
"""

import logging
from pathlib import Path
from typing import Tuple, Optional

from ...ffprobe.media import get_duration, get_video_info
from ...ffprobe.session import probe_session
from ...ffprobe.exec import MetadataError
from ...exceptions import ValidationError

logger = logging.getLogger(__name__)

def validate_single_segment(segment: Path, tolerance: float = 0.2) -> Tuple[bool, Optional[str]]:
    """
    Validate a single encoded segment's properties.
    
    Args:
        segment: Path to the encoded segment
        tolerance: Duration comparison tolerance in seconds
        
    Returns:
        Tuple of (is_valid, error_message)
    """
    try:
        if not segment.exists() or segment.stat().st_size == 0:
            return False, f"Missing or empty segment: {segment.name}"
            
        with probe_session(segment) as probe:
            # Do not enforce codec check here since segmentation uses -c:v copy
            duration = float(probe.get("duration", "format"))
            if duration <= 0:
                return False, f"Invalid duration in segment: {segment.name}"
                
        return True, None
        
    except Exception as e:
        return False, f"Failed to validate segment {segment.name}: {str(e)}"

def validate_encoded_segments(segments_dir: Path) -> bool:
    """Validate encoded segments after parallel encoding."""
    from ...config import WORKING_DIR
    encoded_dir = WORKING_DIR / "encoded_segments"
    original_segments = sorted(segments_dir.glob("*.mkv"))
    encoded_segments = sorted(encoded_dir.glob("*.mkv"))
    
    if len(encoded_segments) != len(original_segments):
        logger.error(
            "Encoded segment count (%d) doesn't match original (%d)",
            len(encoded_segments), len(original_segments)
        )
        return False
        
    for orig, encoded in zip(original_segments, encoded_segments):
        is_valid, error_msg = validate_single_segment(encoded)
        if not is_valid:
            logger.error(error_msg)
            return False
            
        # Compare durations with original
        try:
            with probe_session(orig) as probe:
                orig_duration = float(probe.get("duration", "format"))
            with probe_session(encoded) as probe:
                enc_duration = float(probe.get("duration", "format"))
                
            # Allow a relative tolerance of 5% (or at least 0.2 sec)
            tolerance = max(0.2, orig_duration * 0.05)
            if abs(orig_duration - enc_duration) > tolerance:
                logger.error(
                    "Duration mismatch in %s: %.2f vs %.2f (tolerance: %.2f)",
                    encoded.name, orig_duration, enc_duration, tolerance
                )
                return False
        except Exception as e:
            logger.error("Failed to compare segment durations: %s", e)
            return False
            
    logger.info("Successfully validated %d encoded segments", len(encoded_segments))
    return True
