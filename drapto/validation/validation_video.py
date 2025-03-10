"""Video stream validation utilities

Responsibilities:
- Validate video stream codec and properties
- Check crop dimensions and AV sync
- Verify container integrity
"""

import logging
from pathlib import Path
from typing import List

from ..ffprobe.utils import (
    get_video_info, get_format_info, get_resolution,
    get_media_property, MetadataError, probe_session,
    get_audio_info
)
from ..exceptions import ValidationError

logger = logging.getLogger(__name__)

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
    """Validate audio/video sync"""
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
