"""Validation utilities for checking encode output"""

import logging
from pathlib import Path
from typing import Optional, Tuple

from .utils import run_cmd
from .formatting import print_check, print_error

log = logging.getLogger(__name__)

def validate_output(input_file: Path, output_file: Path) -> bool:
    """
    Validate the output file to ensure encoding was successful.
    Checks:
    - File exists and has size
    - Video stream is AV1
    - Audio streams are Opus
    - Duration matches input (within 1 second)
    
    Args:
        input_file: Path to input video file
        output_file: Path to output video file
        
    Returns:
        bool: True if validation successful
    """
    error = False
    
    # Check if file exists and has size
    if not output_file.exists() or output_file.stat().st_size == 0:
        print_error("Output file is empty or doesn't exist")
        return False
        
    # Check video stream
    try:
        result = run_cmd([
            "ffprobe", "-v", "error",
            "-select_streams", "v",
            "-show_entries", "stream=codec_name",
            "-of", "default=noprint_wrappers=1:nokey=1",
            str(output_file)
        ])
        video_codec = result.stdout.strip()
        if video_codec != "av1":
            print_error(f"No AV1 video stream found in output (found {video_codec})")
            error = True
        else:
            # Get duration if codec check passes
            result = run_cmd([
                "ffprobe", "-v", "error",
                "-show_entries", "format=duration",
                "-of", "default=noprint_wrappers=1:nokey=1",
                str(output_file)
            ])
            duration = float(result.stdout.strip())
            print_check(f"Video stream: AV1, Duration: {duration:.2f}s")
    except Exception as e:
        log.error("Failed to check video stream: %s", e)
        error = True
        
    # Check audio streams
    try:
        result = run_cmd([
            "ffprobe", "-v", "error",
            "-select_streams", "a",
            "-show_entries", "stream=codec_name",
            "-of", "default=noprint_wrappers=1:nokey=1",
            str(output_file)
        ])
        audio_codecs = result.stdout.strip().split('\n')
        opus_count = sum(1 for codec in audio_codecs if codec == "opus")
        
        if opus_count == 0:
            print_error("No Opus audio streams found in output")
            error = True
        else:
            print_check(f"Audio streams: {opus_count} Opus stream(s)")
    except Exception as e:
        log.error("Failed to check audio streams: %s", e)
        error = True
        
    # Compare input and output duration
    try:
        result = run_cmd([
            "ffprobe", "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            str(input_file)
        ])
        input_duration = float(result.stdout.strip())
        
        result = run_cmd([
            "ffprobe", "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            str(output_file)
        ])
        output_duration = float(result.stdout.strip())
        
        duration_diff = abs(input_duration - output_duration)
        if duration_diff > 1.0:  # Allow 1 second difference
            print_error(
                f"Output duration ({output_duration:.2f}s) differs significantly "
                f"from input ({input_duration:.2f}s)"
            )
            error = True
    except Exception as e:
        log.error("Failed to compare durations: %s", e)
        error = True
        
    if error:
        print_error("Output validation failed")
        return False
        
    print_check("Output validation successful")
    return True

def validate_ab_av1() -> bool:
    """
    Check if ab-av1 is available when chunked encoding is enabled
    
    Returns:
        bool: True if ab-av1 is available or not needed
    """
    from .config import ENABLE_CHUNKED_ENCODING
    
    if ENABLE_CHUNKED_ENCODING:
        print_check("Checking for ab-av1...")
        try:
            run_cmd(["which", "ab-av1"])
            print_check("ab-av1 found")
            return True
        except Exception:
            print_error(
                "ab-av1 is required for chunked encoding but not found. "
                "Install with: cargo install ab-av1"
            )
            return False
    return True
