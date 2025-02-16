"""Video segmentation and parallel encoding functions"""

import logging
import os
import shutil
import tempfile
from pathlib import Path
from typing import Optional

from ..config import (
    SEGMENT_LENGTH, TARGET_VMAF, VMAF_SAMPLE_COUNT,
    VMAF_SAMPLE_LENGTH, PRESET, SVT_PARAMS,
    WORKING_DIR
)
from ..utils import run_cmd, check_dependencies

log = logging.getLogger(__name__)

def segment_video(input_file: Path) -> bool:
    """
    Segment video into chunks for parallel encoding
    
    Args:
        input_file: Path to input video file
        
    Returns:
        bool: True if segmentation successful
    """
    segments_dir = WORKING_DIR / "segments"
    segments_dir.mkdir(parents=True, exist_ok=True)
    
    try:
        cmd = [
            "ffmpeg", "-hide_banner", "-loglevel", "error",
            "-i", str(input_file),
            "-c:v", "copy",
            "-an",
            "-f", "segment",
            "-segment_time", str(SEGMENT_LENGTH),
            "-reset_timestamps", "1",
            str(segments_dir / "%04d.mkv")
        ]
        run_cmd(cmd)
        
        # Validate segments
        segments = list(segments_dir.glob("*.mkv"))
        if not segments:
            log.error("No segments created")
            return False
            
        log.info("Created %d segments", len(segments))
        return True
        
    except Exception as e:
        log.error("Segmentation failed: %s", e)
        return False

def encode_segments(crop_filter: Optional[str] = None) -> bool:
    """
    Encode video segments in parallel using ab-av1
    
    Args:
        crop_filter: Optional ffmpeg crop filter string
        
    Returns:
        bool: True if all segments encoded successfully
    """
    if not check_dependencies():
        return False
        
    segments_dir = WORKING_DIR / "segments"
    encoded_dir = WORKING_DIR / "encoded_segments"
    encoded_dir.mkdir(parents=True, exist_ok=True)
    
    # Create temporary script for GNU Parallel
    script_file = tempfile.NamedTemporaryFile(mode='w', delete=False)
    try:
        for segment in segments_dir.glob("*.mkv"):
            output_segment = encoded_dir / segment.name
            
            # Build ab-av1 command
            cmd = [
                "ab-av1", "auto-encode",
                "--input", str(segment),
                "--output", str(output_segment),
                "--encoder", "libsvtav1",
                "--min-vmaf", str(TARGET_VMAF),
                "--preset", str(PRESET),
                "--svt", SVT_PARAMS,
                "--keyint", "10s",
                "--samples", str(VMAF_SAMPLE_COUNT),
                "--sample-duration", f"{VMAF_SAMPLE_LENGTH}s",
                "--vmaf", "n_subsample=8:pool=harmonic_mean",
                "--quiet"
            ]
            if crop_filter:
                cmd.extend(["--vfilter", crop_filter])
                
            script_file.write(" ".join(cmd) + "\n")
            
        script_file.close()
        os.chmod(script_file.name, 0o755)
        
        # Run encoding jobs in parallel
        cmd = [
            "parallel", "--no-notice",
            "--line-buffer",
            "--halt", "soon,fail=1",
            "--jobs", "0",
            ":::", script_file.name
        ]
        run_cmd(cmd)
        
        return True
        
    except Exception as e:
        log.error("Parallel encoding failed: %s", e)
        return False
    finally:
        Path(script_file.name).unlink()

def concatenate_segments(output_file: Path) -> bool:
    """
    Concatenate encoded segments into final video
    
    Args:
        output_file: Path for concatenated output
        
    Returns:
        bool: True if concatenation successful
    """
    encoded_dir = WORKING_DIR / "encoded_segments"
    concat_file = WORKING_DIR / "concat.txt"
    
    try:
        # Create concat file
        with open(concat_file, 'w') as f:
            for segment in sorted(encoded_dir.glob("*.mkv")):
                f.write(f"file '{segment.absolute()}'\n")
                
        # Concatenate segments
        cmd = [
            "ffmpeg", "-hide_banner", "-loglevel", "error",
            "-f", "concat",
            "-safe", "0",
            "-i", str(concat_file),
            "-c", "copy",
            "-y", str(output_file)
        ]
        run_cmd(cmd)
        
        return True
        
    except Exception as e:
        log.error("Concatenation failed: %s", e)
        return False
    finally:
        if concat_file.exists():
            concat_file.unlink()
