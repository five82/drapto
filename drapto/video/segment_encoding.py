"""Functions for encoding video segments in parallel"""

import logging
import shutil
from pathlib import Path
from typing import List, Optional
from concurrent.futures import ProcessPoolExecutor, as_completed
import multiprocessing

from ..config import (
    PRESET, TARGET_VMAF, SVT_PARAMS, 
    VMAF_SAMPLE_COUNT, VMAF_SAMPLE_LENGTH,
    WORKING_DIR
)
from ..utils import run_cmd, check_dependencies
from ..formatting import print_check
from ..validation import validate_ab_av1

log = logging.getLogger(__name__)

def encode_segment(segment: Path, output_segment: Path, crop_filter: Optional[str] = None) -> dict:
    """
    Encode a single video segment using ab-av1.
    
    Returns:
        dict: Encoding statistics and metrics
    """
    import time
    start_time = time.time()
    
    # Get input segment details
    input_info = run_cmd([
        "ffprobe", "-v", "error",
        "-show_entries", "stream=codec_name,width,height,r_frame_rate:format=duration",
        "-of", "default=noprint_wrappers=1:nokey=1",
        str(segment)
    ]).stdout.strip().split('\n')
    input_duration = float(input_info[-1])  # Duration is last item
    
    if input_duration < 10:
        sample_count = 2
    else:
        sample_count = 3
    sample_duration_value = 1
    
    # Run encoding
    cmd = [
        "ab-av1", "auto-encode",
        "--input", str(segment),
        "--output", str(output_segment),
        "--encoder", "libsvtav1",
        "--min-vmaf", str(TARGET_VMAF),
        "--preset", str(PRESET),
        "--svt", SVT_PARAMS,
        "--keyint", "10s",
        "--samples", str(sample_count),
        "--sample-duration", f"{sample_duration_value}s",
        "--vmaf", "n_subsample=8:pool=perc5_min",
        "--pix-format", "yuv420p10le",
    ]
    if crop_filter:
        cmd.extend(["--vfilter", crop_filter])
    
    result = run_cmd(cmd)
    end_time = time.time()
    encoding_time = end_time - start_time
    
    # Get output details
    output_info = run_cmd([
        "ffprobe", "-v", "error",
        "-show_entries", "stream=codec_name,width,height,r_frame_rate:format=duration,size",
        "-of", "default=noprint_wrappers=1:nokey=1",
        str(output_segment)
    ]).stdout.strip().split('\n')
    
    output_duration = float(output_info[-2])  # Duration is second to last
    output_size = int(output_info[-1])  # Size is last item
    
    # Calculate bitrate and speed metrics
    bitrate_kbps = (output_size * 8) / (output_duration * 1000)
    speed_factor = input_duration / encoding_time
    
    # Parse VMAF scores from ab-av1 output if available
    vmaf_score = None
    vmaf_min = None
    vmaf_max = None
    try:
        # Look for VMAF scores in stderr output
        for line in result.stderr.split('\n'):
            if "VMAF score:" in line:
                vmaf_parts = line.split(":")
                if len(vmaf_parts) > 1:
                    scores = [float(s) for s in vmaf_parts[1].strip().split()]
                    if scores:
                        vmaf_score = sum(scores) / len(scores)
                        vmaf_min = min(scores)
                        vmaf_max = max(scores)
                        log.info("  VMAF scores - Avg: %.2f, Min: %.2f, Max: %.2f",
                                vmaf_score, vmaf_min, vmaf_max)
                        break
    except Exception as e:
        log.debug("Could not parse VMAF scores: %s", e)
    
    # Compile segment statistics
    stats = {
        'segment': segment.name,
        'duration': output_duration,
        'size_mb': output_size / (1024 * 1024),
        'bitrate_kbps': bitrate_kbps,
        'encoding_time': encoding_time,
        'speed_factor': speed_factor,
        'resolution': f"{output_info[1]}x{output_info[2]}",
        'framerate': output_info[3],
        'crop_filter': crop_filter or "none",
        'vmaf_score': vmaf_score,
        'vmaf_min': vmaf_min,
        'vmaf_max': vmaf_max
    }
    
    # Log detailed segment info
    log.info("Segment encoding complete: %s", segment.name)
    log.info("  Duration: %.2fs", stats['duration'])
    log.info("  Size: %.2f MB", stats['size_mb'])
    log.info("  Bitrate: %.2f kbps", stats['bitrate_kbps'])
    log.info("  Encoding time: %.2fs (%.2fx realtime)", 
             stats['encoding_time'], stats['speed_factor'])
    log.info("  Resolution: %s @ %s", stats['resolution'], stats['framerate'])
    
    return stats

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
    
    try:
        # Get list of segments to encode
        segments = list(segments_dir.glob("*.mkv"))
        if not segments:
            log.error("No segments found to encode")
            return False

        # Log common ab-av1 encoding parameters once (using placeholders for input/output)
        sample_cmd = [
            "ab-av1", "auto-encode",
            "--input", "<input_segment>",
            "--output", "<output_segment>",
            "--encoder", "libsvtav1",
            "--min-vmaf", str(TARGET_VMAF),
            "--preset", str(PRESET),
            "--svt", SVT_PARAMS,
            "--keyint", "10s",
            "--samples", "<dynamic>",
            "--sample-duration", "<dynamic: sec>",
            "--vmaf", "n_subsample=8:pool=perc5_min",
            "--pix-format", "yuv420p10le",
        ]
        if crop_filter:
            sample_cmd.extend(["--vfilter", crop_filter])
        formatted_sample = " \\\n    ".join(sample_cmd)
        log.info("Common ab-av1 encoding parameters:\n%s", formatted_sample)

        # Use ProcessPoolExecutor for parallel encoding
        max_workers = max(1, multiprocessing.cpu_count())
        segment_stats = []
        
        with ProcessPoolExecutor(max_workers=max_workers) as executor:
            # Submit all encoding jobs
            futures = {}
            for segment in segments:
                output_segment = encoded_dir / segment.name
                future = executor.submit(encode_segment, segment, output_segment, crop_filter)
                futures[future] = segment

            # Monitor jobs as they complete
            failed_segments = []
            for future in as_completed(futures):
                segment = futures[future]
                try:
                    stats = future.result()
                    segment_stats.append(stats)
                    log.info("Successfully encoded segment: %s", segment.name)
                except Exception as e:
                    log.error("Failed to encode segment %s: %s", segment.name, e)
                    failed_segments.append(segment)

            if failed_segments:
                log.error("Failed to encode %d segments", len(failed_segments))
                return False
                
        # Print summary statistics
        if segment_stats:
            total_duration = sum(s['duration'] for s in segment_stats)
            total_size = sum(s['size_mb'] for s in segment_stats)
            avg_bitrate = sum(s['bitrate_kbps'] for s in segment_stats) / len(segment_stats)
            avg_speed = sum(s['speed_factor'] for s in segment_stats) / len(segment_stats)
            
            # Handle VMAF statistics safely
            vmaf_stats = [s for s in segment_stats if s.get('vmaf_score') is not None]
            if vmaf_stats:
                avg_vmaf = sum(s['vmaf_score'] or 0 for s in vmaf_stats) / len(vmaf_stats)
                min_vmaf = min(s['vmaf_min'] or float('inf') for s in vmaf_stats)
                max_vmaf = max(s['vmaf_max'] or 0 for s in vmaf_stats)
                
            log.info("Encoding Summary:")
            log.info("  Total Duration: %.2f seconds", total_duration)
            log.info("  Total Size: %.2f MB", total_size)
            log.info("  Average Bitrate: %.2f kbps", avg_bitrate)
            log.info("  Average Speed: %.2fx realtime", avg_speed)
            if 'avg_vmaf' in locals():
                log.info("  VMAF Scores - Avg: %.2f, Min: %.2f, Max: %.2f",
                         avg_vmaf, min_vmaf, max_vmaf)
        
        # Validate encoded segments
        if not validate_encoded_segments(segments_dir):
            return False
            
        return True
        
    except Exception as e:
        log.error("Parallel encoding failed: %s", e)
        return False
    finally:
        # Cleanup handled by the calling function
        pass

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


