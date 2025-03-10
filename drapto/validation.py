"""Validation utilities for checking encode output

Responsibilities:
  - Coordinate validation of video, audio, subtitle and quality metrics
  - Aggregate validation results into a unified report
  - Present validation status and errors in a consistent format
"""

import logging
from pathlib import Path

from .formatting import print_check, print_error, print_header
from .exceptions import ValidationError, DependencyError
from .utils import run_cmd

from .validation.validation_video import (
    validate_video_stream,
    validate_crop_dimensions,
    validate_av_sync,
    validate_container
)
from .validation.validation_audio import (
    validate_input_audio,
    validate_audio_streams
)
from .validation.validation_subtitles import validate_subtitle_tracks
from .validation.validation_quality import validate_quality_metrics

logger = logging.getLogger(__name__)

def validate_output(input_file: Path, output_file: Path) -> None:
    """Validate the output file to ensure encoding was successful."""
    validation_report = []
    has_errors = False
    
    # Check if file exists and has size
    if not output_file.exists():
        raise ValidationError("Output file does not exist", module="validation")
    if output_file.stat().st_size == 0:
        raise ValidationError("Output file is empty", module="validation")

    try:
        # Add input audio validation
        validate_input_audio(input_file)
        
        # Validate individual components using specialized modules
        validate_video_stream(input_file, output_file, validation_report)
        validate_audio_streams(input_file, output_file, validation_report)
        validate_subtitle_tracks(input_file, output_file, validation_report)
        validate_container(output_file, validation_report)
        validate_crop_dimensions(input_file, output_file, validation_report)
        validate_quality_metrics(input_file, output_file, validation_report)
        validate_av_sync(output_file, validation_report)
        
    except ValidationError as e:
        has_errors = True
        validation_report.append(f"ERROR: {e.message}")
    
    # Print validation report
    if validation_report:
        print_header("Validation Report")
        for entry in validation_report:
            if entry.startswith("ERROR"):
                print_error(entry[7:])
            else:
                print_check(entry)

    # Raise validation exception if errors found
    if any(entry.startswith("ERROR") for entry in validation_report) or has_errors:
        raise ValidationError(
            "Output validation failed with the above issues", 
            module="validation"
        )
    
    print_check("Output validation successful")

def validate_ab_av1() -> None:
    """Check if ab-av1 is available."""
    print_check("Checking for ab-av1...")
    try:
        run_cmd(["which", "ab-av1"])
        print_check("ab-av1 found")
        return
    except Exception:
        raise DependencyError(
            "ab-av1 is required for encoding but not found. Install with: cargo install ab-av1",
            module="validation"
        )
