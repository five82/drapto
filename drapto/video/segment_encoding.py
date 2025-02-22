"""Functions for encoding video segments in parallel"""

import logging
import re
import shutil
import time
import resource
import psutil
from pathlib import Path
from typing import List, Optional, Dict

# Maximum concurrent memory tokens (8 total):
# - Up to 2 concurrent 4K segments (4 tokens each)
# - Up to 4 concurrent 1080p segments (2 tokens each) 
# - Up to 8 concurrent SD segments (1 token each)
MAX_MEMORY_TOKENS = 8

def estimate_memory_weight(segment: Path) -> int:
    """
    Estimate memory weight based on segment resolution:
    - 4K (width ≥ 3840): 4 tokens
    - 1080p (width ≥ 1920): 2 tokens  
    - Lower resolution: 1 token
    """
    try:
        result = run_cmd([
            "ffprobe", "-v", "error",
            "-select_streams", "v:0",
            "-show_entries", "stream=width",
            "-of", "default=noprint_wrappers=1:nokey=1",
            str(segment)
        ])
        width = int(result.stdout.strip())
        if width >= 3840:  # 4K
            return 4
        elif width >= 1920:  # 1080p/2K
            return 2
        return 1  # SD/HD
    except Exception as e:
        log.warning("Failed to get segment width, assuming SD/HD weight: %s", e)
        return 1

from ..config import (
    PRESET, TARGET_VMAF, SVT_PARAMS, 
    VMAF_SAMPLE_COUNT, VMAF_SAMPLE_LENGTH,
    WORKING_DIR
)
from ..utils import run_cmd, check_dependencies
from ..formatting import print_check
from ..validation import validate_ab_av1

log = logging.getLogger(__name__)

def encode_segment(segment: Path, output_segment: Path, crop_filter: Optional[str] = None, retry_count: int = 0, dv_flag: bool = False) -> tuple[dict, list[str]]:
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
    
    if retry_count == 0:
        sample_count = 3
        sample_duration_value = 1
        min_vmaf_value = str(TARGET_VMAF)
    elif retry_count == 1:
        # Second attempt: increase sample count and duration
        sample_count = 4
        sample_duration_value = 2
        min_vmaf_value = str(TARGET_VMAF)
    elif retry_count == 2:
        # Third attempt: keep increased sample settings but raise min_vmaf
        sample_count = 4
        sample_duration_value = 2
        min_vmaf_value = "95"
    
    try:
        # Run encoding
        cmd = [
            "ab-av1", "auto-encode",
            "--input", str(segment),
            "--output", str(output_segment),
            "--encoder", "libsvtav1",
            "--min-vmaf", min_vmaf_value,
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
            
        if dv_flag:
            cmd.extend(["--enc", "dolbyvision=true"])
        
        result = run_cmd(cmd)
    except Exception as e:
        if retry_count < 2:  # Allow up to 2 retries (3 total attempts)
            log.warning("Segment encoding failed, retrying (%d): %s", retry_count + 1, e)
            # Remove failed output if it exists
            if output_segment.exists():
                output_segment.unlink()
            # Retry with incremented retry count
            return encode_segment(segment, output_segment, crop_filter, retry_count + 1, dv_flag)
        else:
            log.error("Segment encoding failed after %d retries", retry_count)
            raise
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
    vmaf_values = []
    try:
        for line in result.stderr.split('\n'):
            # Use a regular expression to capture the VMAF value in lines like "... VMAF 88.72 ..."
            match = re.search(r"VMAF\s+([0-9.]+)", line)
            if match:
                try:
                    value = float(match.group(1))
                    vmaf_values.append(value)
                except Exception:
                    continue
        if vmaf_values:
            vmaf_score = sum(vmaf_values) / len(vmaf_values)
            vmaf_min = min(vmaf_values)
            vmaf_max = max(vmaf_values)
            # Removed duplicate VMAF score log:
            # log.info("  VMAF scores - Avg: %.2f, Min: %.2f, Max: %.2f",
            #          vmaf_score, vmaf_min, vmaf_max)
            log.info("Segment analysis complete: %s – VMAF Avg: %.2f, Min: %.2f, Max: %.2f (CRF target determined)",
                     segment.name, vmaf_score, vmaf_min, vmaf_max)
        else:
            log.info("Segment analysis complete: %s – No VMAF scores parsed", segment.name)
    except Exception as e:
        log.debug("Could not parse VMAF scores: %s", e)
        log.info("Segment analysis complete: %s – No VMAF scores parsed", segment.name)
    
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
    
    # Get peak memory usage (in kilobytes)
    peak_memory_kb = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss

    # Determine resolution category from output width
    try:
        width_result = run_cmd([
            "ffprobe", "-v", "error",
            "-select_streams", "v:0",
            "-show_entries", "stream=width",
            "-of", "default=noprint_wrappers=1:nokey=1",
            str(output_segment)
        ])
        width = int(width_result.stdout.strip())
    except Exception:
        width = 1280  # Fallback

    if width >= 3840:
        resolution_category = "4k"
    elif width >= 1920:
        resolution_category = "1080p"
    else:
        resolution_category = "SDR"

    # Convert peak memory to bytes for later comparison
    peak_memory_bytes = peak_memory_kb * 1024

    # Record the dynamic values in the stats
    stats['peak_memory_kb'] = peak_memory_kb
    stats['peak_memory_bytes'] = peak_memory_bytes
    stats['resolution_category'] = resolution_category

    # Capture all log messages from this worker
    from distributed.worker import get_worker
    worker = get_worker()
    log_messages = getattr(worker, 'log_messages', [])

    return stats, log_messages


def encode_segments(crop_filter: Optional[str] = None, dv_flag: bool = False) -> bool:
    """
    Encode video segments in parallel using Ray for dynamic resource management
    
    Args:
        crop_filter: Optional ffmpeg crop filter string
        dv_flag: Whether this is Dolby Vision content
        
    Returns:
        bool: True if all segments encoded successfully
    """
    from ..validation import validate_ab_av1
    
    if not check_dependencies() or not validate_ab_av1():
        return False

    from dask.distributed import Client, worker_client, Worker
    import logging as python_logging

    # Configure Dask cluster with logging capture
    def worker_setup(dask_worker: Worker):
        dask_worker.log_messages = []
        handler = python_logging.StreamHandler()
        handler.setFormatter(python_logging.Formatter('%(message)s'))
        def emit(record):
            dask_worker.log_messages.append(record.getMessage())
        handler.emit = emit
        # Attach to root logger to capture all messages
        root_logger = python_logging.getLogger()
        root_logger.addHandler(handler)
        # Ensure all loggers propagate up
        for name in ['drapto', 'distributed']:
            logger = python_logging.getLogger(name)
            logger.propagate = True

    client = Client(set_as_default=False)
    client.register_worker_callbacks(setup=worker_setup)
    
    segments_dir = WORKING_DIR / "segments"
    encoded_dir = WORKING_DIR / "encoded_segments"
    encoded_dir.mkdir(parents=True, exist_ok=True)
    
    try:
        segments = list(segments_dir.glob("*.mkv"))
        if not segments:
            log.error("No segments found to encode")
            return False

        # Log encoding parameters
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

        # Submit all segments as Dask tasks
        futures = []
        for segment in segments:
            output_segment = encoded_dir / segment.name
            future = client.submit(encode_segment, segment, output_segment, crop_filter, 0, dv_flag)
            futures.append(future)

        try:
            results = client.gather(futures)
            for stats, log_messages in results:
                # Print captured worker logs
                for msg in log_messages:
                    log.info(msg)
                log.info("Successfully encoded segment: %s", stats.get('segment'))
        except Exception as e:
            log.error("Failed to encode segments: %s", e)
            return False
                
        # Print summary statistics
        if results:
            segment_stats = [s for s, _ in results]
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
        try:
            # Ensure graceful shutdown of all workers
            with logging.getLogger("distributed").handlers[0].console.capture() as capture:
                client.shutdown()
                client.close(timeout=5)
            # Only log actual errors, not expected cleanup messages
            if capture.get().strip():
                cleanup_log = capture.get().lower()
                if "error" in cleanup_log and "commclosed" not in cleanup_log:
                    log.debug("Dask cleanup messages: %s", capture.get())
        except Exception as e:
            # Only log unexpected errors
            if not str(e).startswith(("CommClosedError", "CancelledError")):
                log.debug("Unexpected error during Dask cleanup: %s", e)

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


