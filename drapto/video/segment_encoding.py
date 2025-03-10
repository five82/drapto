"""
Functions for encoding video segments in parallel.

Responsibilities:
  - High-level orchestration of parallel segment encoding
  - Building and executing the encoder command
  - Parsing encoder output for quality metrics (e.g. VMAF) and timing
  - Aggregating per-segment statistics
  - Managing memory-aware task scheduling
  - Validating encoded segment outputs

Note: Low-level tasks (like command construction and metric parsing) have been 
factored out into helper functions.
"""

import logging
import re
import shutil
import time
import resource
import psutil
from pathlib import Path
from typing import List, Optional, Dict
from ..ffprobe_utils import (
    get_video_info, get_format_info, get_media_property,
    probe_session, MetadataError
)
from ..exceptions import DependencyError, SegmentEncodingError

# Maximum concurrent memory tokens (8 total):
# - Up to 2 concurrent 4K segments (4 tokens each)
# - Up to 4 concurrent 1080p segments (2 tokens each) 
# - Up to 8 concurrent SD segments (1 token each)
MAX_MEMORY_TOKENS = 8

def estimate_memory_weight(segment: Path, resolution_weights: dict) -> int:
    """
    Estimate memory weight based on segment resolution using dynamic weights
    from warmup analysis.
    """
    try:
        try:
            with probe_session(segment) as probe:
                width = int(probe.get("width", "video"))
            if width >= 3840:  # 4K
                return resolution_weights['4k']
            elif width >= 1920:  # 1080p/2K
                return resolution_weights['1080p']
            return resolution_weights['SDR']  # SD/HD
        except MetadataError as e:
            logger.warning("Failed to get segment width: %s", e)
            return resolution_weights['SDR']  # Default to SD/HD weight
    except Exception as e:
        logger.warning("Failed to get segment width, using minimum weight: %s", e)
        return min(resolution_weights.values())

import logging
logger = logging.getLogger(__name__)

from ..config import (
    PRESET, TARGET_VMAF, TARGET_VMAF_HDR, SVT_PARAMS, 
    VMAF_SAMPLE_COUNT, VMAF_SAMPLE_LENGTH,
    WORKING_DIR, TASK_STAGGER_DELAY
)
from ..utils import run_cmd, check_dependencies
from ..formatting import print_check
from ..validation import validate_ab_av1

def _build_encode_command(
    segment: Path,
    output_segment: Path,
    crop_filter: Optional[str],
    retry_count: int,
    is_hdr: bool,
    dv_flag: bool
) -> list[str]:
    """Build the ab-av1 encode command with appropriate parameters."""
    # Determine sampling parameters based on retry count
    if retry_count == 0:
        sample_count = 3
        sample_duration_value = 1
        min_vmaf_value = str(TARGET_VMAF_HDR if is_hdr else TARGET_VMAF)
    elif retry_count == 1:
        sample_count = 4
        sample_duration_value = 2
        min_vmaf_value = str(TARGET_VMAF_HDR if is_hdr else TARGET_VMAF)
    else:  # retry_count == 2
        sample_count = 4
        sample_duration_value = 2
        min_vmaf_value = "95"  # Force highest quality

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
    return cmd

def _parse_vmaf_scores(stderr_output: str) -> tuple[Optional[float], Optional[float], Optional[float]]:
    """Parse VMAF scores from encoder output."""
    vmaf_values = []
    for line in stderr_output.splitlines():
        match = re.search(r"VMAF\s+([0-9.]+)", line)
        if match:
            try:
                vmaf_values.append(float(match.group(1)))
            except ValueError:
                continue
    if vmaf_values:
        return (
            sum(vmaf_values) / len(vmaf_values),  # average
            min(vmaf_values),                     # minimum
            max(vmaf_values)                      # maximum
        )
    return (None, None, None)

def encode_segment(segment: Path, output_segment: Path, crop_filter: Optional[str] = None, retry_count: int = 0, is_hdr: bool = False, dv_flag: bool = False) -> tuple[dict, list[str]]:
    """
    Encode a single video segment using ab-av1.
    
    Args:
        segment: Input segment path
        output_segment: Output segment path
        crop_filter: Optional crop filter string
        retry_count: Number of previous retry attempts
        is_hdr: Whether input is HDR content
        dv_flag: Whether input is Dolby Vision
        
    Returns:
        Tuple of (encoding statistics dict, list of log messages)
        
    Raises:
        SegmentEncodingError: If encoding fails
    """
    import time
    output_logs = []  # List to collect detailed log messages
    
    def capture_log(msg, *args, **kwargs):
        formatted = msg % args if args else msg
        logger.info(formatted)
        output_logs.append(formatted)
        
    start_time = time.time()
    
    # Get input segment details using probe session
    try:
        with probe_session(segment) as probe:
            input_duration = float(probe.get("duration", "format"))
            width = int(probe.get("width", "video"))
    except MetadataError as e:
        logger.error("Failed to get segment info: %s", e)
        if retry_count < 2:
            logger.warning("Retrying segment due to metadata error")
            return encode_segment(segment, output_segment, crop_filter, retry_count + 1, is_hdr, dv_flag)
    except Exception as e:
        logger.warning("Failed to parse width from video info: %s. Assuming non-4k.", e)
        width = 0

    # Initial attempt: Standard sampling parameters
    if retry_count == 0:
        sample_count = 3
        sample_duration_value = 1
        min_vmaf_value = str(TARGET_VMAF_HDR if is_hdr else TARGET_VMAF)
        
    # First retry: Increase samples and duration for more accurate analysis
    elif retry_count == 1:
        sample_count = 4
        sample_duration_value = 2
        min_vmaf_value = str(TARGET_VMAF_HDR if is_hdr else TARGET_VMAF)
        
    # Final retry: Force maximum quality to ensure successful encode
    elif retry_count == 2:
        sample_count = 4
        sample_duration_value = 2
        min_vmaf_value = "95"  # Force highest quality
    
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
            logger.warning("Segment encoding failed, retrying (%d): %s", retry_count + 1, e)
            # Remove failed output if it exists
            if output_segment.exists():
                output_segment.unlink()
            # Retry with incremented retry count
            return encode_segment(segment, output_segment, crop_filter, retry_count + 1, dv_flag)
        else:
            logger.error("Segment encoding failed after %d retries", retry_count)
            raise
    end_time = time.time()
    encoding_time = end_time - start_time
    
    # Get output details
    video_info_out = get_video_info(output_segment)
    format_info_out = get_format_info(output_segment)
    output_duration = float(format_info_out.get("duration", 0))
    output_size = int(format_info_out.get("size", 0))
    
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
            capture_log("Segment analysis complete: %s – VMAF Avg: %.2f, Min: %.2f, Max: %.2f (CRF target determined)",
                     segment.name, vmaf_score, vmaf_min, vmaf_max)
        else:
            capture_log("Segment analysis complete: %s – No VMAF scores parsed", segment.name)
    except Exception as e:
        logger.debug("Could not parse VMAF scores: %s", e)
        logger.info("Segment analysis complete: %s – No VMAF scores parsed", segment.name)
    
    # Compile segment statistics
    stats = {
        'segment': segment.name,
        'duration': output_duration,
        'size_mb': output_size / (1024 * 1024),
        'bitrate_kbps': bitrate_kbps,
        'encoding_time': encoding_time,
        'speed_factor': speed_factor,
        'resolution': f"{video_info_out.get('width', '0')}x{video_info_out.get('height', '0')}",
        'framerate': video_info_out.get('r_frame_rate', 'unknown'),
        'crop_filter': crop_filter or "none",
        'vmaf_score': vmaf_score,
        'vmaf_min': vmaf_min,
        'vmaf_max': vmaf_max
    }
    
    # Log detailed segment info
    capture_log("Segment encoding complete: %s", segment.name)
    capture_log("  Duration: %.2fs", stats['duration'])
    capture_log("  Size: %.2f MB", stats['size_mb'])
    capture_log("  Bitrate: %.2f kbps", stats['bitrate_kbps'])
    capture_log("  Encoding time: %.2fs (%.2fx realtime)", 
             stats['encoding_time'], stats['speed_factor'])
    capture_log("  Resolution: %s @ %s", stats['resolution'], stats['framerate'])
    
    # Get peak memory usage (in kilobytes)
    peak_memory_kb = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss

    # Determine resolution category from output width
    try:
        try:
            with probe_session(output_segment) as probe:
                width = int(probe.get("width", "video"))
        except MetadataError:
            logger.warning("Could not determine output width, using fallback value")
            width = 1280  # Fallback
    except Exception as e:
        logger.error("Error determining resolution: %s", e)
        width = 1280  # Fallback on any error

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

    return stats, output_logs


# Number of segments to process sequentially for warm-up
WARMUP_COUNT = 3  # Increased to get better average

def calculate_memory_requirements(warmup_results):
    """
    Calculate base memory token size from warmup results and dynamically
    adjust based on actual peak memory usage during warmup.
    """
    memory_by_resolution = {'SDR': [], '1080p': [], '4k': []}
    
    for stats, _ in warmup_results:
        category = stats['resolution_category']
        memory_bytes = stats['peak_memory_bytes']
        memory_by_resolution[category].append(memory_bytes)
    
    # Calculate averages for each resolution category
    averages = {}
    for category, values in memory_by_resolution.items():
        if values:
            averages[category] = sum(values) / len(values)
    
    # Calculate the average peak memory usage during warmup
    peak_memories = [stats['peak_memory_bytes'] for stats, _ in warmup_results if 'peak_memory_bytes' in stats]
    if peak_memories:
        actual_peak = max(peak_memories)
        # Use the larger of the calculated average or actual peak memory
        base_size = max(
            min((size for size in averages.values() if size > 0), default=512 * 1024 * 1024),
            actual_peak // 4  # Divide by 4 since we'll multiply by weights later
        )
    else:
        # Fallback to original calculation if no peak memory data
        base_size = min((size for size in averages.values() if size > 0), default=512 * 1024 * 1024)
    
    # Calculate relative weights
    weights = {
        'SDR': 1,
        '1080p': max(1, int(averages.get('1080p', base_size) / base_size)),
        '4k': max(2, int(averages.get('4k', base_size * 2) / base_size))
    }
    
    return base_size, weights

def encode_segments(crop_filter: Optional[str] = None, is_hdr: bool = False, dv_flag: bool = False) -> None:
    """
    Encode video segments in parallel with dynamic memory-aware scheduling
    
    Args:
        crop_filter: Optional ffmpeg crop filter string
        dv_flag: Whether this is Dolby Vision content
        
    Raises:
        SegmentEncodingError: If encoding fails
        DependencyError: If required dependencies are missing
    """
    from ..validation import validate_ab_av1
    import psutil
    import time
    from concurrent.futures import ThreadPoolExecutor, as_completed
    
    check_dependencies()  # Will raise DependencyError if any issues
    validate_ab_av1()     # Will raise DependencyError if ab-av1 missing

    # Configure memory thresholds
    MEMORY_THRESHOLD = 0.8  # Use up to 80% of available memory
    BASE_MEMORY_PER_TOKEN = 512 * 1024 * 1024  # 512MB base memory per token
    
    segments_dir = WORKING_DIR / "segments"
    encoded_dir = WORKING_DIR / "encoded_segments"
    encoded_dir.mkdir(parents=True, exist_ok=True)
    
    try:
        segments = list(segments_dir.glob("*.mkv"))
        if not segments:
            logger.error("No segments found to encode")
            return False

        # Log common ab-av1 encoding parameters once before warmup
        # Use parameters corresponding to the first attempt:
        sample_count = 3
        sample_duration_value = 1
        min_vmaf_value = str(TARGET_VMAF_HDR if is_hdr else TARGET_VMAF)
        common_cmd = [
            "ab-av1", "auto-encode",
            "--input", "<input_segment>",
            "--output", "<output_segment>",
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
            common_cmd.extend(["--vfilter", crop_filter])
        if dv_flag:
            common_cmd.extend(["--enc", "dolbyvision=true"])
        formatted_common_cmd = " \\\n    ".join(common_cmd)
        logger.info("Common ab-av1 encoding parameters:\n    %s", formatted_common_cmd)

        # Warm-up: process first WARMUP_COUNT segments sequentially to gauge memory usage
        warmup_results = []
        for i in range(min(WARMUP_COUNT, len(segments))):
            segment = segments[i]
            output_segment = encoded_dir / segment.name
            logger.info("Warm-up encoding for segment: %s", segment.name)
            result = encode_segment(segment, output_segment, crop_filter, 0, is_hdr, dv_flag)
            warmup_results.append(result)
        next_segment_idx = min(WARMUP_COUNT, len(segments))

        # Calculate dynamic memory requirements from warmup results
        base_memory_per_token, resolution_weights = calculate_memory_requirements(warmup_results)
        logger.info("Dynamic memory analysis:")
        logger.info("  Base memory per token: %.2f MB", base_memory_per_token / (1024 * 1024))
        logger.info("  Resolution weights: %s", resolution_weights)


        # Initialize thread pool and scheduler
        max_workers = psutil.cpu_count()
        completed_results = []
    
        from ..scheduler import MemoryAwareScheduler
        scheduler = MemoryAwareScheduler(base_memory_per_token, MAX_MEMORY_TOKENS, TASK_STAGGER_DELAY)

        with ThreadPoolExecutor(max_workers=max_workers) as executor:
            while next_segment_idx < len(segments) or scheduler.running_tasks:
                # Check system memory and update completed tasks
                if psutil.virtual_memory().percent >= 90:
                    logger.info("High memory usage (%d%%); pausing task submissions...",
                            psutil.virtual_memory().percent)
                    time.sleep(1)
                    scheduler.update_completed()
                    continue

                # Submit new tasks as long as there are segments remaining
                while next_segment_idx < len(segments):
                    segment = segments[next_segment_idx]
                    output_segment = encoded_dir / segment.name
                    memory_weight = estimate_memory_weight(segment, resolution_weights)
                    estimated_memory = memory_weight * base_memory_per_token

                    if scheduler.can_submit(estimated_memory):
                        future = executor.submit(encode_segment, segment, output_segment,
                                              crop_filter, 0, dv_flag)
                        scheduler.add_task(next_segment_idx, future, memory_weight)
                        next_segment_idx += 1
                    else:
                        break

                # Check for completed tasks
                for task_id, (future, _) in list(scheduler.running_tasks.items()):
                    if future.done():
                        try:
                            result = future.result()
                            completed_results.append(result)
                            stats, log_messages = result
                        
                            # Print captured logs
                            for msg in log_messages:
                                logger.info(msg)
                            logger.info("Successfully encoded segment: %s",
                                    stats.get('segment'))
                        except Exception as e:
                            logger.error("Task failed: %s", e)
                            return False

                # Update completed tasks in scheduler
                scheduler.update_completed()

                # Short sleep before next loop iteration
                if scheduler.running_tasks:
                    time.sleep(0.1)
                
        # Print summary statistics
        if completed_results:
            segment_stats = [s for s, _ in completed_results]
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
                
            logger.info("Encoding Summary:")
            logger.info("  Total Duration: %.2f seconds", total_duration)
            logger.info("  Total Size: %.2f MB", total_size)
            logger.info("  Average Bitrate: %.2f kbps", avg_bitrate)
            logger.info("  Average Speed: %.2fx realtime", avg_speed)
            if 'avg_vmaf' in locals():
                logger.info("  VMAF Scores - Avg: %.2f, Min: %.2f, Max: %.2f",
                         avg_vmaf, min_vmaf, max_vmaf)
        
        # Validate encoded segments
        validate_encoded_segments(segments_dir)
            
    except Exception as e:
        logger.error("Parallel encoding failed: %s", e)
        raise SegmentEncodingError(f"Parallel encoding failed: {str(e)}", module="segment_encoding") from e

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
        logger.error(
            "Encoded segment count (%d) doesn't match original (%d)",
            len(encoded_segments), len(original_segments)
        )
        return False
        
    for orig, encoded in zip(original_segments, encoded_segments):
        try:
            # Check encoded segment exists and has size
            if not encoded.exists() or encoded.stat().st_size == 0:
                logger.error("Missing or empty encoded segment: %s", encoded.name)
                return False
                
            # Verify AV1 codec and basic stream properties
            try:
                with probe_session(encoded) as probe:
                    codec = probe.get("codec_name", "video")
                    width = probe.get("width", "video")
                    height = probe.get("height", "video")
                    duration = probe.get("duration", "format")
                
                # Verify codec
                if codec != "av1":
                    logger.error(
                        "Wrong codec '%s' in encoded segment: %s",
                        codec, encoded.name
                    )
                    return False
                
            except MetadataError as e:
                logger.error("Failed to get encoded segment properties: %s", e)
                return False

            # Compare durations (allow 0.1s difference)
            try:
                with probe_session(orig) as probe:
                    orig_duration = float(probe.get("duration", "format"))
                enc_duration = float(duration)
                # Allow a relative tolerance of 5% (or at least 0.2 sec) to account for slight discrepancies
                tolerance = max(0.2, orig_duration * 0.05)
                if abs(orig_duration - enc_duration) > tolerance:
                    logger.error(
                        "Duration mismatch in %s: %.2f vs %.2f (tolerance: %.2f)",
                        encoded.name, orig_duration, enc_duration, tolerance
                    )
                    return False
            except Exception as e:
                logger.error("Failed to compare segment durations: %s", e)
                return False
                
        except Exception as e:
            logger.error("Failed to validate encoded segment %s: %s", encoded.name, e)
            raise SegmentEncodingError("Failed to validate encoded segment", module="segment_encoding")
            
    logger.info("Successfully validated %d encoded segments", len(encoded_segments))
    return True


