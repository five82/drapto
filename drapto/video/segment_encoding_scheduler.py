"""Segment encoding scheduler and memory management

Responsibilities:
- Estimate memory requirements for segments
- Schedule and coordinate parallel encoding tasks
- Monitor system resources during encoding
- Validate encoded segment outputs
"""

import logging
import time
import psutil
from pathlib import Path
from typing import Dict, List, Tuple
from concurrent.futures import ThreadPoolExecutor, Future

from ..ffprobe.exec import MetadataError
from ..ffprobe.media import get_duration, get_video_info
from ..ffprobe.session import probe_session
from ..exceptions import SegmentEncodingError
from ..config import (
    WORKING_DIR, TASK_STAGGER_DELAY, MAX_MEMORY_TOKENS
)
from .encode_helpers import estimate_memory_weight
from ..scheduler import MemoryAwareScheduler

logger = logging.getLogger(__name__)

def calculate_memory_requirements(warmup_results):
    """Calculate base memory token size from warmup results."""
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
        base_size = max(
            min((size for size in averages.values() if size > 0), default=512 * 1024 * 1024),
            actual_peak // 4
        )
    else:
        base_size = min((size for size in averages.values() if size > 0), default=512 * 1024 * 1024)
    
    weights = {
        'SDR': 1,
        '1080p': max(1, int(averages.get('1080p', base_size) / base_size)),
        '4k': max(2, int(averages.get('4k', base_size * 2) / base_size))
    }
    
    return base_size, weights

def validate_encoded_segments(segments_dir: Path) -> bool:
    """Validate encoded segments after parallel encoding."""
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
            if not encoded.exists() or encoded.stat().st_size == 0:
                logger.error("Missing or empty segment: %s", encoded.name)
                return False

            with probe_session(encoded) as probe:
                codec = probe.get("codec_name", "video")
                if codec != "av1":
                    logger.error("Wrong codec '%s' in segment: %s", codec, encoded.name)
                    return False
                
                enc_duration = float(probe.get("duration", "format"))
                
            with probe_session(orig) as probe:
                orig_duration = float(probe.get("duration", "format"))
                
            # Allow a relative tolerance of 5% (or at least 0.2 sec)
            tolerance = max(0.2, orig_duration * 0.05)
            if abs(orig_duration - enc_duration) > tolerance:
                logger.error(
                    "Duration mismatch in %s: %.2f vs %.2f (tolerance: %.2f)",
                    encoded.name, orig_duration, enc_duration, tolerance
                )
                return False
        except Exception as e:
            logger.error("Failed to validate segment %s: %s", encoded.name, e)
            return False
            
    logger.info("Successfully validated %d encoded segments", len(encoded_segments))
    return True



def orchestrate_parallel_encoding(
    segments: List[Path],
    encoded_dir: Path,
    crop_filter: str,
    is_hdr: bool,
    dv_flag: bool,
    encode_segment_fn
) -> bool:
    """
    Orchestrate parallel encoding of segments with memory-aware scheduling.
    
    Args:
        segments: List of input segment paths
        encoded_dir: Output directory for encoded segments
        crop_filter: Optional crop filter string
        is_hdr: Whether content is HDR
        dv_flag: Whether content is Dolby Vision
        encode_segment_fn: Function to encode individual segments
        
    Returns:
        bool: True if all segments encoded successfully
    """
    # Process first segments sequentially for warmup
    WARMUP_COUNT = 3
    warmup_results = []
    for i in range(min(WARMUP_COUNT, len(segments))):
        segment = segments[i]
        output_segment = encoded_dir / segment.name
        logger.info("Warm-up encoding for segment: %s", segment.name)
        result = encode_segment_fn(segment, output_segment, crop_filter, 0, is_hdr, dv_flag)
        warmup_results.append(result)
        for msg in result[1]:
            logger.info(msg)
    next_segment_idx = min(WARMUP_COUNT, len(segments))

    # Calculate dynamic memory requirements
    base_memory_per_token, resolution_weights = calculate_memory_requirements(warmup_results)
    logger.info("Dynamic memory analysis:")
    logger.info("  Base memory per token: %.2f MB", base_memory_per_token / (1024 * 1024))
    logger.info("  Resolution weights: %s", resolution_weights)

    # Initialize thread pool and scheduler
    max_workers = psutil.cpu_count()
    completed_results = []
    scheduler = MemoryAwareScheduler(base_memory_per_token, MAX_MEMORY_TOKENS, TASK_STAGGER_DELAY)

    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        while next_segment_idx < len(segments) or scheduler.running_tasks:
            if psutil.virtual_memory().percent >= 90:
                logger.info("High memory usage (%d%%); pausing submissions...",
                        psutil.virtual_memory().percent)
                time.sleep(1)
                scheduler.update_completed()
                continue

            while next_segment_idx < len(segments):
                segment = segments[next_segment_idx]
                output_segment = encoded_dir / segment.name
                memory_weight = estimate_memory_weight(segment, resolution_weights)
                estimated_memory = memory_weight * base_memory_per_token

                if scheduler.can_submit(estimated_memory):
                    future = executor.submit(encode_segment_fn, segment, output_segment,
                                          crop_filter, 0, is_hdr, dv_flag)
                    scheduler.add_task(next_segment_idx, future, memory_weight)
                    next_segment_idx += 1
                else:
                    break

            for task_id, (future, _) in list(scheduler.running_tasks.items()):
                if future.done():
                    try:
                        result = future.result()
                        completed_results.append(result)
                        stats, log_messages = result
                    
                        for msg in log_messages:
                            logger.info(msg)
                        logger.info("Successfully encoded segment: %s",
                                stats.get('segment'))
                    except Exception as e:
                        logger.error("Task failed: %s", e)
                        return False

            scheduler.update_completed()

            if scheduler.running_tasks:
                time.sleep(0.1)
                
    # Print summary statistics
    if completed_results:
        segment_stats = [s for s, _ in completed_results]
        total_duration = sum(s['duration'] for s in segment_stats)
        total_size = sum(s['size_mb'] for s in segment_stats)
        avg_bitrate = sum(s['bitrate_kbps'] for s in segment_stats) / len(segment_stats)
        avg_speed = sum(s['speed_factor'] for s in segment_stats) / len(segment_stats)
        
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
    
    return validate_encoded_segments(segments_dir=WORKING_DIR / "segments")
