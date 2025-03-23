"""Audio validation utilities

Responsibilities:
- Validate input audio streams
- Verify encoded audio properties
- Check audio track preservation
"""

import logging
from pathlib import Path
from typing import List

from ..ffprobe.media import get_all_audio_info, get_audio_info, get_duration
from ..ffprobe.exec import MetadataError, get_media_property
from ..exceptions import ValidationError

logger = logging.getLogger(__name__)

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
    """Validate audio stream properties"""
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
                
            # Duration validation with calculated fallbacks
            try:
                in_dur = get_duration(input_file, "audio", idx)
                out_dur = get_duration(output_file, "audio", idx)
                
                duration_diff = abs(in_dur - out_dur)
                if duration_diff > 0.5:  # Allow 500ms difference
                    raise ValidationError(
                        f"Track {idx} duration mismatch: input {in_dur:.1f}s vs output {out_dur:.1f}s",
                        module="audio_validation"
                    )
                    
            except MetadataError as e:
                logger.warning("Could not validate duration for track %d: %s", idx, e)
        
        validation_report.append(f"Audio: {len(output_audio)} validated Opus streams")
        
    except Exception as e:
        validation_report.append(f"ERROR: {str(e)}")
        raise
