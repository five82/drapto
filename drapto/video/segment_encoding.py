"""Functions for encoding video segments in parallel"""

import logging
import re
import shutil
import time
import resource
import psutil
from pathlib import Path
from typing import List, Optional, Dict
from ..ffprobe_utils import get_video_info, get_format_info

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
        info = get_video_info(segment)
        width = int(info.get("width", 0))
        if width >= 3840:  # 4K
            return resolution_weights['4k']
        elif width >= 1920:  # 1080p/2K
            return resolution_weights['1080p']
        return resolution_weights['SDR']  # SD/HD
    except Exception as e:
        log.warning("Failed to get segment width, using minimum weight: %s", e)
        return min(resolution_weights.values())

from ..config import (
    PRESET, TARGET_VMAF, SVT_PARAMS, 
    VMAF_SAMPLE_COUNT, VMAF_SAMPLE_LENGTH,
    WORKING_DIR, TASK_STAGGER_DELAY
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
    output_logs = []  # List to collect detailed log messages
    
    def capture_log(msg, *args, **kwargs):
        formatted = msg % args if args else msg
        log.info(formatted)  # Pass only the formatted string
        output_logs.append(formatted)
        
    start_time = time.time()
    
    # Get input segment details
    # Use ffprobe_utils to get segment info
    video_info = get_video_info(segment)
    format_info = get_format_info(segment)
    input_duration = float(format_info.get("duration", 0))
    try:
        width = int(video_info.get("width", 0))
    except Exception as e:
        log.warning("Failed to parse width from video info: %s. Assuming non-4k.", e)
        width = 0

    if retry_count == 0:
        sample_count = 3
        sample_duration_value = 1
        if width >= 3840:
            min_vmaf_value = "95"
        else:
            min_vmaf_value = str(TARGET_VMAF)
    elif retry_count == 1:
        # Second attempt: increase sample count and duration
        sample_count = 4
        sample_duration_value = 2
        if width >= 3840:
            min_vmaf_value = "95"
        else:
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

def encode_segments(crop_filter: Optional[str] = None, dv_flag: bool = False) -> bool:
    """
    Encode video segments in parallel with dynamic memory-aware scheduling
    
    Args:
        crop_filter: Optional ffmpeg crop filter string
        dv_flag: Whether this is Dolby Vision content
        
    Returns:
        bool: True if all segments encoded successfully
    """
    from ..validation import validate_ab_av1
    import psutil
    import time
    from concurrent.futures import ThreadPoolExecutor, as_completed
    
    if not check_dependencies() or not validate_ab_av1():
        return False

    # Configure memory thresholds
    MEMORY_THRESHOLD = 0.8  # Use up to 80% of available memory
    BASE_MEMORY_PER_TOKEN = 512 * 1024 * 1024  # 512MB base memory per token
    
    segments_dir = WORKING_DIR / "segments"
    encoded_dir = WORKING_DIR / "encoded_segments"
    encoded_dir.mkdir(parents=True, exist_ok=True)
    
    try:
        segments = list(segments_dir.glob("*.mkv"))
        if not segments:
            log.error("No segments found to encode")
            return False

        # Warm-up: process first WARMUP_COUNT segments sequentially to gauge memory usage
        warmup_results = []
        for i in range(min(WARMUP_COUNT, len(segments))):
            segment = segments[i]
            output_segment = encoded_dir / segment.name
            log.info("Warm-up encoding for segment: %s", segment.name)
            result = encode_segment(segment, output_segment, crop_filter, 0, dv_flag)
            warmup_results.append(result)
        next_segment_idx = min(WARMUP_COUNT, len(segments))

        # Calculate dynamic memory requirements from warmup results
        base_memory_per_token, resolution_weights = calculate_memory_requirements(warmup_results)
        log.info("Dynamic memory analysis:")
        log.info("  Base memory per token: %.2f MB", base_memory_per_token / (1024 * 1024))
        log.info("  Resolution weights: %s", resolution_weights)

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

        # Initialize thread pool and tracking variables
        max_workers = psutil.cpu_count()
        running_tasks = {}  # task_id -> (future, memory_weight)
        completed_results = []

        with ThreadPoolExecutor(max_workers=max_workers) as executor:
            while next_segment_idx < len(segments) or running_tasks:
                # Check overall system memory usage and wait if usage is 90% or higher
                if psutil.virtual_memory().percent >= 90:
                    log.info("High memory usage (%d%%); pausing new task submissions...", 
                            psutil.virtual_memory().percent)
                    time.sleep(1)
                    continue

                # Check available memory
                mem = psutil.virtual_memory()
                available_memory = mem.available
                target_available = mem.total * (1 - MEMORY_THRESHOLD)
                
                # Calculate current memory usage by our tasks
                current_task_memory = sum(weight * BASE_MEMORY_PER_TOKEN 
                                        for _, weight in running_tasks.values())
                
                # Submit new tasks if memory permits
                while (next_segment_idx < len(segments) and 
                       available_memory - current_task_memory > target_available and
                       available_memory > mem.total * 0.1):
                    segment = segments[next_segment_idx]
                    output_segment = encoded_dir / segment.name
                    
                    # Estimate memory requirements
                    memory_weight = estimate_memory_weight(segment, resolution_weights)
                    estimated_memory = memory_weight * base_memory_per_token
                    
                    # Calculate current token usage
                    current_tokens = sum(weight for (_, weight) in running_tasks.values())
                    
                    # Add 10% safety margin to estimated memory
                    with_margin = estimated_memory * 1.1
                    
                    # Check both memory availability and token limit
                    if (available_memory - (current_task_memory + with_margin) > target_available and
                        current_tokens + memory_weight <= MAX_MEMORY_TOKENS):
                        # Stagger task submissions
                        time.sleep(TASK_STAGGER_DELAY)
                        future = executor.submit(encode_segment, segment, output_segment, 
                                              crop_filter, 0, dv_flag)
                        running_tasks[next_segment_idx] = (future, memory_weight)
                        current_task_memory += estimated_memory
                        next_segment_idx += 1
                    else:
                        break
                
                # Check for completed tasks
                completed = []
                for task_id, (future, _) in running_tasks.items():
                    if future.done():
                        try:
                            result = future.result()
                            completed_results.append(result)
                            stats, log_messages = result
                            
                            # Print captured logs
                            for msg in log_messages:
                                log.info(msg)
                            log.info("Successfully encoded segment: %s", 
                                    stats.get('segment'))
                            
                        except Exception as e:
                            log.error("Task failed: %s", e)
                            return False
                        completed.append(task_id)
                
                # Remove completed tasks
                for task_id in completed:
                    running_tasks.pop(task_id)
                
                # Short sleep to prevent tight loop
                if running_tasks:
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
            # Allow a relative tolerance of 5% (or at least 0.2 sec) to account for slight discrepancies
            tolerance = max(0.2, orig_duration * 0.05)
            if abs(orig_duration - enc_duration) > tolerance:
                log.error(
                    "Duration mismatch in %s: %.2f vs %.2f (tolerance: %.2f)",
                    encoded.name, orig_duration, enc_duration, tolerance
                )
                return False
                
        except Exception as e:
            log.error("Failed to validate encoded segment %s: %s", encoded.name, e)
            return False
            
    log.info("Successfully validated %d encoded segments", len(encoded_segments))
    return True


