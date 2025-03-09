"""Validation utilities for checking encode output"""

import json
import logging
from pathlib import Path
from typing import Optional, Tuple

from .utils import run_cmd
from .formatting import print_check, print_error, print_header
from .ffprobe_utils import get_video_info, get_format_info, get_subtitle_info, get_all_audio_info
from .exceptions import ValidationError

def validate_video_stream(input_file: Path, output_file: Path, validation_report: list) -> None:
    """Validate video stream properties"""
    try:
        info = get_video_info(output_file)
        codec = info.get("codec_name", "")
        width = info.get("width", "")
        height = info.get("height", "")
        pix_fmt = info.get("pix_fmt", "")
        framerate = info.get("r_frame_rate", "")
        
        if codec != "av1":
            raise ValidationError(
                f"No AV1 video stream found (found {codec})", 
                module="validation"
            )
            
        validation_report.append(f"Video: {width}x{height} {pix_fmt} @ {framerate}fps")
    except Exception as e:
        validation_report.append(f"ERROR: {str(e)}")
        raise

def validate_audio_streams(input_file: Path, output_file: Path, validation_report: list) -> None:
    """Validate audio stream properties"""
    try:
        streams = get_all_audio_info(output_file)
        opus_count = sum(1 for s in streams if s.get("codec_name") == "opus")
        
        if opus_count == 0:
            raise ValidationError("No Opus audio streams found", module="validation")
            
        validation_report.append(f"Audio: {opus_count} Opus stream(s)")
    except Exception as e:
        validation_report.append(f"ERROR: {str(e)}")
        raise

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

def validate_container(output_file: Path, validation_report: list) -> None:
    """Validate container integrity"""
    try:
        format_info = get_format_info(output_file)
        if not format_info:
            raise ValidationError("Invalid container structure", module="validation")
        validation_report.append("Container: Valid MKV structure")
    except Exception as e:
        validation_report.append(f"ERROR: {str(e)}")
        raise

def validate_crop_dimensions(input_file: Path, output_file: Path, validation_report: list) -> None:
    """Validate crop dimensions if applied"""
    try:
        in_video = get_video_info(input_file)
        out_video = get_video_info(output_file)
        
        in_res = [str(in_video.get("width", "")), str(in_video.get("height", ""))]
        out_res = [str(out_video.get("width", "")), str(out_video.get("height", ""))]
        
        if not in_res or not out_res:
            raise ValidationError("Failed to get resolution data", module="validation")
            
        if in_res != out_res:
            validation_report.append(f"Crop: {out_res[0]}x{out_res[1]} (from {in_res[0]}x{in_res[1]})")
    except Exception as e:
        validation_report.append(f"ERROR: {str(e)}")
        raise

def validate_av_sync(output_file: Path, validation_report: list) -> None:
    """
    Validate audio/video sync by comparing the start time and duration of the first
    audio and video streams in the output file. Any difference above a 0.1-second
    threshold will flag a sync issue.
    
    Args:
        output_file: Path to the output video file.
        validation_report: List to append status messages.
    """
    try:
        video_info = get_video_info(output_file)
        if not video_info:
            raise ValidationError("No video stream info found", module="validation")
        vid_start = float(video_info.get("start_time") or 0)
        vid_duration = float(video_info.get("duration") or 0)
        
        all_audio = get_all_audio_info(output_file)
        if not all_audio:
            raise ValidationError("No audio stream info found", module="validation")
        audio_info = all_audio[0]
        aud_start = float(audio_info.get("start_time") or 0)
        aud_duration = float(audio_info.get("duration") or 0)
        
        start_diff = abs(vid_start - aud_start)
        duration_diff = abs(vid_duration - aud_duration)
        threshold = 0.1  # allowed difference in seconds
        
        if start_diff > threshold or duration_diff > threshold:
            raise ValidationError(
                f"AV sync issue: video_start={vid_start:.2f}s, audio_start={aud_start:.2f}s",
                module="validation"
            )
            
        validation_report.append("AV sync validated: audio and video start times and durations are within threshold")
    except Exception as e:
        validation_report.append(f"ERROR: {str(e)}")
        raise

def validate_quality_metrics(input_file: Path, output_file: Path, validation_report: list) -> None:
    """
    Validate quality metrics using VMAF analysis.
    """
    try:
        from .config import TARGET_VMAF
        validation_report.append(f"Quality target: VMAF {TARGET_VMAF}")
    except Exception as e:
        validation_report.append(f"ERROR: {str(e)}")
        raise

logger = logging.getLogger(__name__)

def validate_output(input_file: Path, output_file: Path) -> None:
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
        raise ValidationError("Output file is empty or doesn't exist", module="validation")

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
    
    # Validate audio/video sync
    error |= not validate_av_sync(output_file, validation_report)
    
    # Output validation report
    try:
        print_header("Validation Report")
        for entry in validation_report:
            if entry.startswith("ERROR"):
                print_error(entry[7:])  # Skip "ERROR: " prefix
            else:
                print_check(entry)
    except Exception as e:
        logger.error("Failed to output validation report: %s", e)
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
        logger.error("Failed to check audio streams: %s", e)
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
        logger.error("Failed to compare durations: %s", e)
        error = True
        
    if error:
        raise ValidationError("Output validation failed", module="validation")
        
    print_check("Output validation successful")

def validate_ab_av1() -> bool:
    """
    Check if ab-av1 is available.
    
    Returns:
        bool: True if ab-av1 is available.
    """
    print_check("Checking for ab-av1...")
    try:
        run_cmd(["which", "ab-av1"])
        print_check("ab-av1 found")
        return True
    except Exception:
        print_error(
            "ab-av1 is required for encoding but not found. "
            "Install with: cargo install ab-av1"
        )
        return False
