"""Video segment encoding orchestration

This module coordinates parallel video segment encoding with:

High-Level Orchestration:
- Parallel encoding job scheduling and monitoring
- Memory-aware task distribution
- Progress tracking and statistics aggregation
- Overall encode validation

Low-Level Operations (delegated to helpers):
- Individual segment encoding via ab-av1
- VMAF score parsing and analysis
- Memory usage estimation
- Hardware resource monitoring

The separation ensures the main encode_segments() function focuses purely on
orchestration while delegating implementation details to specialized helpers.
"""

import logging
logger = logging.getLogger(__name__)
import shutil
import time
import resource
import psutil
from pathlib import Path
from typing import List, Optional, Dict, Tuple

def _validate_single_encoded_segment(segment: Path, tolerance: float = 0.2) -> Tuple[bool, Optional[str]]:
    """
    Validate a single encoded segment's properties.
    
    Args:
        segment: Path to the encoded segment
        tolerance: Duration comparison tolerance in seconds
        
    Returns:
        Tuple of (is_valid, error_message)
    """
    try:
        if not segment.exists() or segment.stat().st_size == 0:
            return False, f"Missing or empty segment: {segment.name}"
            
        with probe_session(segment) as probe:
            codec = probe.get("codec_name", "video")
            if codec != "av1":
                return False, f"Wrong codec '{codec}' in segment: {segment.name}"
                
            duration = float(probe.get("duration", "format"))
            if duration <= 0:
                return False, f"Invalid duration in segment: {segment.name}"
                
        return True, None
        
    except Exception as e:
        return False, f"Failed to validate segment {segment.name}: {str(e)}"
from ..ffprobe.utils import (
    get_video_info, get_format_info, get_media_property,
    probe_session, MetadataError
)
from ..exceptions import DependencyError, SegmentEncodingError
from .encode_helpers import (
    build_encode_command,
    parse_vmaf_scores,
    get_segment_properties,
    calculate_output_metrics,
    get_resolution_category,
    compile_segment_stats,
    log_segment_progress,
    handle_segment_retry
)

# Maximum concurrent memory tokens (8 total):
# - Up to 2 concurrent 4K segments (4 tokens each)
# - Up to 4 concurrent 1080p segments (2 tokens each) 
# - Up to 8 concurrent SD segments (1 token each)
MAX_MEMORY_TOKENS = 8


from ..config import (
    PRESET, TARGET_VMAF, TARGET_VMAF_HDR, SVT_PARAMS, 
    VMAF_SAMPLE_COUNT, VMAF_SAMPLE_LENGTH,
    WORKING_DIR, TASK_STAGGER_DELAY
)
from ..utils import run_cmd, check_dependencies
from ..formatting import print_check
from ..validation import validate_ab_av1


def encode_segment(segment: Path, output_segment: Path, crop_filter: Optional[str] = None,
                  retry_count: int = 0, is_hdr: bool = False, dv_flag: bool = False) -> tuple[dict, list[str]]:
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
    output_logs = []
    start_time = time.time()
    
    try:
        # Get input properties
        input_duration, width = get_segment_properties(segment)
    except MetadataError as e:
        return handle_segment_retry(e, segment, output_segment, crop_filter,
                                  retry_count, is_hdr, dv_flag)
    
    try:
        # Run encoding
        cmd = build_encode_command(segment, output_segment, crop_filter,
                                 retry_count, is_hdr, dv_flag)
        result = run_cmd(cmd)
    except Exception as e:
        return handle_segment_retry(e, segment, output_segment, crop_filter,
                                  retry_count, is_hdr, dv_flag)

    # Calculate metrics
    encoding_time = time.time() - start_time
    metrics = calculate_output_metrics(output_segment, input_duration, encoding_time)
    vmaf_metrics = parse_vmaf_scores(result.stderr)
    
    # Compile stats and log progress
    stats = compile_segment_stats(output_segment, encoding_time, crop_filter, vmaf_metrics, metrics)
    log_segment_progress(stats, output_logs, segment.name, *vmaf_metrics)

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
    from ..utils import check_dependencies
    from .segment_encoding_scheduler import orchestrate_parallel_encoding
    
    check_dependencies()  # Will raise DependencyError if any issues
    validate_ab_av1()    # Will raise DependencyError if ab-av1 missing
    
    segments_dir = WORKING_DIR / "segments"
    encoded_dir = WORKING_DIR / "encoded_segments"
    encoded_dir.mkdir(parents=True, exist_ok=True)
    
    try:
        segments = list(segments_dir.glob("*.mkv"))
        if not segments:
            logger.error("No segments found to encode")
            return False

        # Log common encoding parameters
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

        # Orchestrate parallel encoding
        success = orchestrate_parallel_encoding(
            segments=segments,
            encoded_dir=encoded_dir,
            crop_filter=crop_filter,
            is_hdr=is_hdr,
            dv_flag=dv_flag,
            encode_segment_fn=encode_segment
        )
        
        if not success:
            raise SegmentEncodingError("Parallel encoding failed", module="segment_encoding")
            
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
        is_valid, error_msg = _validate_single_encoded_segment(encoded)
        if not is_valid:
            logger.error(error_msg)
            return False
            
        # Compare durations with original
        try:
            with probe_session(orig) as probe:
                orig_duration = float(probe.get("duration", "format"))
            with probe_session(encoded) as probe:
                enc_duration = float(probe.get("duration", "format"))
                
            # Allow a relative tolerance of 5% (or at least 0.2 sec)
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
            
    logger.info("Successfully validated %d encoded segments", len(encoded_segments))
    return True


