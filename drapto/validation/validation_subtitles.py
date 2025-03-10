"""Subtitle validation utilities

Responsibilities:
- Verify subtitle track preservation
- Validate subtitle stream properties
"""

import logging
from pathlib import Path
from typing import List

from ..ffprobe.utils import get_subtitle_info
from ..exceptions import ValidationError

logger = logging.getLogger(__name__)

def validate_subtitle_tracks(input_file: Path, output_file: Path, validation_report: list) -> None:
    """Validate subtitle track preservation"""
    try:
        subtitle_info = get_subtitle_info(output_file)
        if not subtitle_info:
            raise ValidationError("No subtitle streams found", module="validation")
        subtitle_count = len(subtitle_info.get("streams", []))
        validation_report.append(f"Subtitles: {subtitle_count} tracks")
    except Exception as e:
        validation_report.append(f"ERROR: {str(e)}")
        raise
