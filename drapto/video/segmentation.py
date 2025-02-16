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
from ..formatting import print_info, print_check

log = logging.getLogger(__name__)

def validate_segments(input_file: Path, segment_length: int, variable_segmentation: bool = False) -> bool:
    """
    Validate video segments after segmentation.
    
    Args:
        input_file: Original input video file for duration comparison.
        segment_length: Expected segment length in seconds (for fixed segmentation).
        variable_segmentation: If True, variable (scene-based) segmentation was used.
        
    Returns:
        bool: True if all segments are valid.
    """
    from .scene_detection import detect_scenes, validate_segment_boundaries
    segments_dir = WORKING_DIR / "segments"
    segments = sorted(segments_dir.glob("*.mkv"))
    
    if not segments:
        log.error("No segments created")
        return False
    log.info("Found %d segments", len(segments))
        
    if not variable_segmentation:
        # Fixed segmentation: perform expected segments check.
        try:
            result = run_cmd([
                "ffprobe", "-v", "error",
                "-show_entries", "format=duration",
                "-of", "default=noprint_wrappers=1:nokey=1",
                str(input_file)
            ])
            total_duration = float(result.stdout.strip())
            expected_segments = (total_duration + segment_length - 1) // segment_length
            
            if len(segments) < expected_segments * 0.9:  # Allow 10% tolerance
                log.error(
                    "Found fewer segments than expected: %d vs %d expected",
                    len(segments), expected_segments
                )
                return False
        except Exception as e:
            log.error("Failed to get input duration: %s", e)
            return False
    else:
        log.info("Variable segmentation in use; skipping fixed expected segments check")
        try:
            result = run_cmd([
                "ffprobe", "-v", "error",
                "-show_entries", "format=duration",
                "-of", "default=noprint_wrappers=1:nokey=1",
                str(input_file)
            ])
            total_duration = float(result.stdout.strip())
        except Exception as e:
            log.error("Failed to get input duration: %s", e)
            return False
        
    # Validate each segment and build a list of valid segments
    total_segment_duration = 0.0
    min_size = 1024  # 1KB minimum segment size
    valid_segments = []
    
    for segment in segments:
        # Check file size
        if segment.stat().st_size < min_size:
            log.error("Segment too small: %s", segment.name)
            return False
    
        try:
            result = run_cmd([
                "ffprobe", "-v", "error",
                "-show_entries", "format=duration:stream=codec_name",
                "-of", "default=noprint_wrappers=1:nokey=1",
                str(segment)
            ])
            lines = result.stdout.strip().split('\n')
            duration = None
            codec = None
            for line in lines:
                line = line.strip()
                if not line:
                    continue
                try:
                    duration = float(line)
                except ValueError:
                    codec = line
            if duration is None or codec is None:
                log.error("Invalid segment %s: missing duration or codec", segment.name)
                return False
                
            log.info("Segment %s: duration=%.2fs, codec=%s", segment.name, duration, codec)
    
            # Check if segment duration is short
            if duration < 1.0:
                # For any short segment, check scene alignment
                from .scene_detection import detect_scenes
                scenes = detect_scenes(input_file)
                cumulative_duration = sum(d for _, d in valid_segments)
                segment_end = cumulative_duration + duration
                tolerance = 0.5  # seconds
                aligned = any(abs(scene - segment_end) <= tolerance for scene in scenes)
                
                if aligned:
                    log.info("Short segment %.2fs in %s aligns with scene change", duration, segment.name)
                    valid_segments.append((segment, duration))
                    total_segment_duration += duration
                else:
                    # For non-scene-aligned short segments:
                    if segment == segments[-1]:
                        # Last segment can be shorter
                        if duration < 0.1:
                            log.error("Last segment too short: %.2fs in %s", duration, segment.name)
                            return False
                        valid_segments.append((segment, duration))
                        total_segment_duration += duration
                    else:
                        # Non-last segments must meet minimum requirements
                        if variable_segmentation:
                            log.warning("Skipping short non-scene-aligned segment: %.2fs in %s", duration, segment.name)
                            continue
                        else:
                            if duration < segment_length * 0.75:
                                log.error("Invalid segment duration: %.2fs in %s", duration, segment.name)
                                return False
            else:
                # Normal duration segment
                valid_segments.append((segment, duration))
                total_segment_duration += duration
    
        except Exception as e:
            log.error("Failed to validate segment %s: %s", segment.name, e)
            return False
    
    # After processing, validate total duration
    valid_count = len(valid_segments)
    try:
        result = run_cmd([
            "ffprobe", "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            str(input_file)
        ])
        total_duration = float(result.stdout.strip())
    except Exception as e:
        log.error("Failed to get input duration: %s", e)
        return False

    # For fixed segmentation, check both duration and expected count
    if not variable_segmentation:
        expected_segments = (total_duration + segment_length - 1) // segment_length
        if valid_count < expected_segments * 0.9:
            log.error("Found fewer valid segments than expected: %d vs %d expected", valid_count, expected_segments)
            return False
        if abs(total_segment_duration - total_duration) > segment_length:
            log.error("Total valid segment duration (%.2fs) differs significantly from input (%.2fs)",
                      total_segment_duration, total_duration)
            return False
    else:
        # For variable segmentation, only check that total duration matches within tolerance
        duration_tolerance = max(1.0, total_duration * 0.02)  # 2% tolerance or minimum 1 second
        if abs(total_segment_duration - total_duration) > duration_tolerance:
            log.error("Total valid segment duration (%.2fs) differs significantly from input (%.2fs)",
                      total_segment_duration, total_duration)
            return False

    # Detect scenes and validate segment boundaries against scene changes
    scenes = detect_scenes(input_file)
    short_segments = validate_segment_boundaries(segments_dir, scenes)
    
    # Don't fail validation for short segments that align with scene changes
    problematic_segments = [s for s, is_scene in short_segments if not is_scene]
    if problematic_segments:
        log.warning(
            "Found %d problematic short segments not aligned with scene changes",
            len(problematic_segments)
        )
    
    print_check(f"Successfully validated {valid_count} segments")
    return True

def segment_video(input_file: Path) -> bool:
    """
    Segment video into chunks for parallel encoding
    
    Args:
        input_file: Path to input video file
        
    Returns:
        bool: True if segmentation successful
    """
    from .hardware import check_hardware_acceleration, get_hwaccel_options
    
    segments_dir = WORKING_DIR / "segments"
    segments_dir.mkdir(parents=True, exist_ok=True)
    
    try:
        # Check for hardware decoding support
        hw_type = check_hardware_acceleration()
        hw_opt = get_hwaccel_options(hw_type)
        
        cmd = [
            "ffmpeg", "-hide_banner", "-loglevel", "warning",
        ]
        
        if hw_opt:
            # Add hardware acceleration for decoding only
            cmd.extend(hw_opt.split())
            
        from .scene_detection import detect_scenes
        scenes = detect_scenes(input_file)
        if scenes:
            # Create a comma-separated list of scene-change timestamps (in seconds)
            # Optionally, filter out any scene times below a minimum value (e.g. 1.0s) if needed.
            segment_times = ",".join(f"{t:.2f}" for t in scenes if t > 1.0)
            cmd.extend([
                "-i", str(input_file),
                "-c:v", "copy",
                "-an",
                "-f", "segment",
                "-segment_times", segment_times,
                "-reset_timestamps", "1",
                str(segments_dir / "%04d.mkv")
            ])
            variable_seg = True
        else:
            log.error("Scene detection failed; no scenes detected. Failing segmentation.")
            return False
            
        run_cmd(cmd)
        
        # Validate segments with the appropriate variable_segmentation flag
        if not validate_segments(input_file, SEGMENT_LENGTH, variable_segmentation=variable_seg):
            return False
            
        return True
        
    except Exception as e:
        log.error("Segmentation failed: %s", e)
        return False
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
    from ..validation import validate_ab_av1
    
    if not check_dependencies() or not validate_ab_av1():
        return False
        
    segments_dir = WORKING_DIR / "segments"
    encoded_dir = WORKING_DIR / "encoded_segments"
    encoded_dir.mkdir(parents=True, exist_ok=True)
    
    # Track failed segments for reporting
    failed_segments = []
    
    # Create temporary script for GNU Parallel
    script_file = tempfile.NamedTemporaryFile(mode='w', delete=False)
    
    command_logged = False
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
                "--pix-format", "yuv420p10le",
                "--quiet"
            ]
            if crop_filter:
                cmd.extend(["--vfilter", crop_filter])

            # Log the command only once
            if not command_logged:
                formatted_command = " \\\n    ".join(cmd)
                print_info("Encoding command parameters (common for all segments):")
                log.info("\n%s", formatted_command)
                command_logged = True

            # Write the command to the temporary script file for GNU Parallel
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
        
        # Validate encoded segments
        if not validate_encoded_segments(segments_dir):
            return False
            
        return True
        
    except Exception as e:
        log.error("Parallel encoding failed: %s", e)
        return False
    finally:
        Path(script_file.name).unlink()

def validate_encoded_segments(segments_dir: Path) -> bool:
    """
    Validate encoded segments after parallel encoding
    
    Args:
        segments_dir: Directory containing original segments for comparison
        
    Returns:
        bool: True if all encoded segments are valid
    """
    encoded_dir = WORKING_DIR / "encoded_segments"
    original_segments = sorted(segments_dir.glob("*.mkv"))
    encoded_segments = sorted(encoded_dir.glob("*.mkv"))
    
    if len(encoded_segments) != len(original_segments):
        log.error(
            "Encoded segment count (%d) doesn't match original (%d)",
            len(encoded_segments), len(original_segments)
        )
        return False
        
    for orig, encoded in zip(original_segments, encoded_segments):
        try:
            # Check encoded segment exists and has size
            if not encoded.exists() or encoded.stat().st_size == 0:
                log.error("Missing or empty encoded segment: %s", encoded.name)
                return False
                
            # Verify AV1 codec and basic stream properties
            result = run_cmd([
                "ffprobe", "-v", "error",
                "-show_entries", "stream=codec_name,width,height:format=duration",
                "-of", "default=noprint_wrappers=1:nokey=1",
                str(encoded)
            ])
            
            lines = result.stdout.strip().split('\n')
            if len(lines) < 4:  # codec, width, height, duration
                log.error("Invalid encoded segment  %s", encoded.name)
                return False
                
            codec, width, height, duration = lines
            
            # Verify codec
            if codec != "av1":
                log.error(
                    "Wrong codec '%s' in encoded segment: %s",
                    codec, encoded.name
                )
                return False
                
            # Compare durations (allow 0.1s difference)
            orig_duration = float(run_cmd([
                "ffprobe", "-v", "error",
                "-show_entries", "format=duration",
                "-of", "default=noprint_wrappers=1:nokey=1",
                str(orig)
            ]).stdout.strip())
            
            enc_duration = float(duration)
            if abs(orig_duration - enc_duration) > 0.1:
                log.error(
                    "Duration mismatch in %s: %.2f vs %.2f",
                    encoded.name, orig_duration, enc_duration
                )
                return False
                
        except Exception as e:
            log.error("Failed to validate encoded segment %s: %s", encoded.name, e)
            return False
            
    log.info("Successfully validated %d encoded segments", len(encoded_segments))
    return True

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
        # Get total duration of encoded segments
        total_segment_duration = 0
        segments = sorted(encoded_dir.glob("*.mkv"))
        
        for segment in segments:
            try:
                result = run_cmd([
                    "ffprobe", "-v", "error",
                    "-show_entries", "format=duration",
                    "-of", "default=noprint_wrappers=1:nokey=1",
                    str(segment)
                ])
                duration = float(result.stdout.strip())
                total_segment_duration += duration
            except Exception as e:
                log.error("Failed to get duration for segment %s: %s", segment.name, e)
                return False
        
        # Create concat file
        with open(concat_file, 'w') as f:
            for segment in segments:
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
        
        # Validate concatenated output
        if not output_file.exists() or output_file.stat().st_size == 0:
            log.error("Concatenated output is missing or empty")
            return False
            
        # Verify output duration matches total segment duration
        try:
            result = run_cmd([
                "ffprobe", "-v", "error",
                "-show_entries", "format=duration",
                "-of", "default=noprint_wrappers=1:nokey=1",
                str(output_file)
            ])
            output_duration = float(result.stdout.strip())
            
            if abs(output_duration - total_segment_duration) > 1.0:
                log.error(
                    "Concatenated output duration (%.2fs) differs from "
                    "total segment duration (%.2fs)",
                    output_duration, total_segment_duration
                )
                return False
                
            # Verify AV1 codec in output
            result = run_cmd([
                "ffprobe", "-v", "error",
                "-select_streams", "v",
                "-show_entries", "stream=codec_name",
                "-of", "default=noprint_wrappers=1:nokey=1",
                str(output_file)
            ])
            if result.stdout.strip() != "av1":
                log.error("Concatenated output has wrong codec: %s", result.stdout.strip())
                return False
                
            log.info("Successfully validated concatenated output")
            return True
            
        except Exception as e:
            log.error("Failed to validate concatenated output: %s", e)
            return False
        
    except Exception as e:
        log.error("Concatenation failed: %s", e)
        return False
    finally:
        if concat_file.exists():
            concat_file.unlink()
