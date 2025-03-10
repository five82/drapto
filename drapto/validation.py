"""Validation utilities for checking encode output"""

import json
import logging
from pathlib import Path
from typing import Optional, Tuple

from .utils import run_cmd
from .formatting import print_check, print_error, print_header
from .ffprobe_utils import (
    get_video_info, get_format_info, get_subtitle_info,
    get_all_audio_info, probe_session, get_resolution,
    get_audio_info, get_media_property, MetadataError
)
from .exceptions import ValidationError, DependencyError

def validate_video_stream(input_file: Path, output_file: Path, validation_report: list) -> None:
    """Validate video stream properties"""
    try:
        video_info = get_video_info(output_file)
        codec = video_info.get("codec_name")
        width = video_info.get("width")
        height = video_info.get("height")
        pix_fmt = video_info.get("pix_fmt")
        framerate = video_info.get("r_frame_rate")
            
        if codec != "av1":
            raise ValidationError(
                f"No AV1 video stream found (found {codec})", 
                module="validation"
            )
            
        validation_report.append(f"Video: {width}x{height} {pix_fmt} @ {framerate}fps")
    except Exception as e:
        validation_report.append(f"ERROR: {str(e)}")
        raise

def validate_input_audio(input_file: Path) -> None:
    """Validate input audio streams before processing"""
    try:
        audio_info = get_all_audio_info(input_file)
        if not audio_info:
            raise ValidationError("Input file contains no audio streams", module="audio_validation")
            
        logger.info("Found %d audio streams in input", len(audio_info))
        for idx, stream in enumerate(audio_info):
            if not stream.get('codec_name'):
                raise ValidationError(f"Audio stream {idx} has invalid codec", module="audio_validation")
                
    except Exception as e:
        raise ValidationError(f"Input audio validation failed: {str(e)}", module="audio_validation") from e

def validate_encoded_audio(audio_file: Path, original_index: int) -> None:
    """Validate an encoded audio track"""
    try:
        # Basic file validation
        if not audio_file.exists():
            raise ValidationError(f"Encoded audio track {original_index} missing", module="audio_validation")
        if audio_file.stat().st_size < 1024:
            raise ValidationError(f"Encoded audio track {original_index} too small", module="audio_validation")
            
        # Codec validation
        audio_info = get_audio_info(audio_file, 0)
        if audio_info.get('codec_name') != 'opus':
            raise ValidationError(f"Encoded track {original_index} has wrong codec", module="audio_validation")
            
        # Channel count validation
        original_channels = get_media_property(audio_file, "audio", "channels", 0)
        if original_channels < 1:
            raise ValidationError(f"Encoded track {original_index} has invalid channel count", module="audio_validation")
            
    except MetadataError as e:
        raise ValidationError(f"Encoded audio validation failed: {str(e)}", module="audio_validation") from e

def validate_audio_streams(input_file: Path, output_file: Path, validation_report: list) -> None:
    """Validate audio stream properties with enhanced checks"""
    try:
        # Get original input audio info
        input_audio = get_all_audio_info(input_file)
        output_audio = get_all_audio_info(output_file)
        
        # Track count validation
        if len(input_audio) != len(output_audio):
            raise ValidationError(
                f"Audio track count mismatch: input {len(input_audio)} vs output {len(output_audio)}",
                module="audio_validation"
            )
            
        # Track-by-track validation
        for idx, (in_stream, out_stream) in enumerate(zip(input_audio, output_audio)):
            # Codec check
            if out_stream.get("codec_name") != "opus":
                raise ValidationError(
                    f"Track {idx} has wrong codec: {out_stream.get('codec_name')}",
                    module="audio_validation"
                )
                
            # Channel count preservation
            in_channels = in_stream.get("channels")
            out_channels = out_stream.get("channels")
            if in_channels != out_channels:
                raise ValidationError(
                    f"Track {idx} channel mismatch: input {in_channels} vs output {out_channels}",
                    module="audio_validation"
                )
                
            # Duration validation with null checks
            in_dur = in_stream.get("duration")
            out_dur = out_stream.get("duration")
            
            if in_dur is None:
                logger.warning("Input audio track %d has no duration metadata", idx)
            if out_dur is None:
                logger.warning("Output audio track %d has no duration metadata", idx)
                
            if None not in (in_dur, out_dur):
                try:
                    duration_diff = abs(float(in_dur) - float(out_dur))
                    if duration_diff > 0.5:  # Allow 500ms difference
                        raise ValidationError(
                            f"Track {idx} duration mismatch: input {float(in_dur):.1f}s vs output {float(out_dur):.1f}s",
                            module="audio_validation"
                        )
                except (TypeError, ValueError) as e:
                    logger.warning("Skipping duration validation for track %d: %s", idx, str(e))
        
        validation_report.append(f"Audio: {len(output_audio)} validated Opus streams")
        
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
        in_width, in_height = get_resolution(input_file)
        out_width, out_height = get_resolution(output_file)
        
        in_res = [str(in_width), str(in_height)]
        out_res = [str(out_width), str(out_height)]
            
        if not all(in_res) or not all(out_res):
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
        vid_start = video_info.get("start_time", 0.0)
        vid_duration = video_info.get("duration") or 0.0
        
        audio_info = get_audio_info(output_file, 0)
        if not audio_info:
            raise ValidationError("No audio stream info found", module="validation")
        aud_start = audio_info.get("start_time", 0.0)
        aud_duration = audio_info.get("duration") or 0.0
        
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
        
        # Validate individual components
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
    
    # Final check for any accumulated errors
    if any(entry.startswith("ERROR") for entry in validation_report) or has_errors:
        print_header("Validation Report")
        for entry in validation_report:
            if entry.startswith("ERROR"):
                print_error(entry[7:])
            else:
                print_check(entry)
        raise ValidationError(
            "Output validation failed with the above issues", 
            module="validation"
        )
    
    print_check("Output validation successful")

def validate_ab_av1() -> None:
    """
    Check if ab-av1 is available.
    
    Returns:
        bool: True if ab-av1 is available.
    """
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
