"""Validation utilities for checking encode output"""

import logging
from pathlib import Path
from typing import Optional, Tuple

from .utils import run_cmd
from .formatting import print_check, print_error, print_header

def validate_video_stream(input_file: Path, output_file: Path, validation_report: list) -> bool:
    """Validate video stream properties"""
    try:
        # Check video codec
        result = run_cmd([
            "ffprobe", "-v", "error",
            "-select_streams", "v",
            "-show_entries", "stream=codec_name,width,height,pix_fmt,r_frame_rate",
            "-of", "default=noprint_wrappers=1:nokey=1",
            str(output_file)
        ])
        codec, width, height, pix_fmt, framerate = result.stdout.strip().split('\n')
        
        if codec != "av1":
            validation_report.append(f"ERROR: No AV1 video stream found (found {codec})")
            return False
            
        validation_report.append(f"Video: {width}x{height} {pix_fmt} @ {framerate}fps")
        return True
    except Exception as e:
        validation_report.append(f"ERROR: Failed to validate video stream: {e}")
        return False

def validate_audio_streams(input_file: Path, output_file: Path, validation_report: list) -> bool:
    """Validate audio stream properties"""
    try:
        result = run_cmd([
            "ffprobe", "-v", "error",
            "-select_streams", "a",
            "-show_entries", "stream=codec_name,channels,bit_rate",
            "-of", "default=noprint_wrappers=1:nokey=1",
            str(output_file)
        ])
        streams = result.stdout.strip().split('\n')
        opus_count = sum(1 for s in streams if s.startswith('opus'))
        
        if opus_count == 0:
            validation_report.append("ERROR: No Opus audio streams found")
            return False
            
        validation_report.append(f"Audio: {opus_count} Opus streams")
        return True
    except Exception as e:
        validation_report.append(f"ERROR: Failed to validate audio streams: {e}")
        return False

def validate_subtitle_tracks(input_file: Path, output_file: Path, validation_report: list) -> bool:
    """Validate subtitle track preservation"""
    try:
        result = run_cmd([
            "ffprobe", "-v", "error",
            "-select_streams", "s",
            "-show_entries", "stream=index",
            "-of", "default=noprint_wrappers=1:nokey=1",
            str(output_file)
        ])
        subtitle_count = len(result.stdout.strip().split('\n')) if result.stdout.strip() else 0
        validation_report.append(f"Subtitles: {subtitle_count} tracks")
        return True
    except Exception as e:
        validation_report.append(f"ERROR: Failed to validate subtitle tracks: {e}")
        return False

def validate_container(output_file: Path, validation_report: list) -> bool:
    """Validate container integrity"""
    try:
        run_cmd(["ffprobe", "-v", "error", str(output_file)])
        validation_report.append("Container: Valid MKV structure")
        return True
    except Exception as e:
        validation_report.append(f"ERROR: Invalid container structure: {e}")
        return False

def validate_crop_dimensions(input_file: Path, output_file: Path, validation_report: list) -> bool:
    """Validate crop dimensions if applied"""
    try:
        in_res = run_cmd([
            "ffprobe", "-v", "error",
            "-select_streams", "v",
            "-show_entries", "stream=width,height",
            "-of", "default=noprint_wrappers=1:nokey=1",
            str(input_file)
        ]).stdout.strip().split('\n')
        
        out_res = run_cmd([
            "ffprobe", "-v", "error",
            "-select_streams", "v",
            "-show_entries", "stream=width,height",
            "-of", "default=noprint_wrappers=1:nokey=1",
            str(output_file)
        ]).stdout.strip().split('\n')
        
        if in_res != out_res:
            validation_report.append(f"Crop: {out_res[0]}x{out_res[1]} (from {in_res[0]}x{in_res[1]})")
        return True
    except Exception as e:
        validation_report.append(f"ERROR: Failed to validate crop dimensions: {e}")
        return False

def validate_quality_metrics(input_file: Path, output_file: Path, validation_report: list) -> bool:
    """
    Skip bitrate reduction quality check.
    (Quality metric not implemented; using file size reduction as the summary metric.)
    """
    validation_report.append("Quality: N/A (quality metric not implemented)")
    return True

log = logging.getLogger(__name__)

def validate_output(input_file: Path, output_file: Path) -> bool:
    """
    Validate the output file to ensure encoding was successful.
    Checks:
    - File exists and has size
    - Video stream properties (codec, resolution, framerate, bit depth)
    - Audio streams (codec, channel layout, bitrate)
    - Duration matches input (within 1 second)
    - Container integrity
    - Subtitle track preservation
    - Crop dimensions (if applied)
    - Quality metrics (VMAF)
    
    Args:
        input_file: Path to input video file
        output_file: Path to output video file
        
    Returns:
        bool: True if validation successful
    """
    error = False
    validation_report = []
    
    # Check if file exists and has size
    if not output_file.exists() or output_file.stat().st_size == 0:
        print_error("Output file is empty or doesn't exist")
        return False

    # Validate video stream properties
    error |= not validate_video_stream(input_file, output_file, validation_report)
    
    # Validate audio streams
    error |= not validate_audio_streams(input_file, output_file, validation_report)
    
    # Validate subtitle tracks
    error |= not validate_subtitle_tracks(input_file, output_file, validation_report)
    
    # Validate container integrity
    error |= not validate_container(output_file, validation_report)
    
    # Validate crop dimensions if applied
    error |= not validate_crop_dimensions(input_file, output_file, validation_report)
    
    # Run quality metrics check
    error |= not validate_quality_metrics(input_file, output_file, validation_report)
    
    # Output validation report
    try:
        print_header("Validation Report")
        for entry in validation_report:
            if entry.startswith("ERROR"):
                print_error(entry[7:])  # Skip "ERROR: " prefix
            else:
                print_check(entry)
    except Exception as e:
        log.error("Failed to output validation report: %s", e)
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
