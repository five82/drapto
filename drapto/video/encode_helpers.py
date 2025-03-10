"""Helper functions for segment encoding

Responsibilities:
- Parse VMAF scores from encoder output
- Calculate encoding metrics and statistics
- Handle retry logic for failed encodes
- Format and compile segment statistics
"""

import logging
import re
import resource
from pathlib import Path
from typing import Optional, Tuple, Dict
from ..exceptions import SegmentEncodingError

from ..ffprobe.session import probe_session
from ..ffprobe.media import get_duration, get_video_info, get_format_info
from ..ffprobe.exec import MetadataError

logger = logging.getLogger(__name__)

def parse_vmaf_scores(stderr_output: str) -> tuple[Optional[float], Optional[float], Optional[float]]:
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

def get_segment_properties(segment: Path) -> tuple[float, int]:
    """Get input segment duration and width."""
    try:
        with probe_session(segment) as probe:
            input_duration = float(probe.get("duration", "format"))
            width = int(probe.get("width", "video"))
            return input_duration, width
    except MetadataError as e:
        raise
    except Exception as e:
        logger.warning("Failed to parse width from video info: %s. Assuming non-4k.", e)
        return 0.0, 0

def calculate_output_metrics(output_segment: Path, input_duration: float, encoding_time: float) -> dict:
    """Calculate output metrics like bitrate and speed factor."""
    video_info_out = get_video_info(output_segment)
    format_info_out = get_format_info(output_segment)
    output_duration = float(format_info_out.get("duration", 0))
    output_size = int(format_info_out.get("size", 0))
    
    return {
        'duration': output_duration,
        'size_mb': output_size / (1024 * 1024),
        'bitrate_kbps': (output_size * 8) / (output_duration * 1000),
        'speed_factor': input_duration / encoding_time,
        'resolution': f"{video_info_out.get('width', '0')}x{video_info_out.get('height', '0')}",
        'framerate': video_info_out.get('r_frame_rate', 'unknown'),
    }

def get_resolution_category(output_segment: Path) -> tuple[str, int]:
    """Determine resolution category and width."""
    try:
        with probe_session(output_segment) as probe:
            width = int(probe.get("width", "video"))
    except (MetadataError, Exception) as e:
        logger.warning("Could not determine output width, using fallback value: %s", e)
        width = 1280  # Fallback

    if width >= 3840:
        return "4k", width
    elif width >= 1920:
        return "1080p", width
    return "SDR", width

def compile_segment_stats(segment: Path, encoding_time: float, crop_filter: str,
                       vmaf_metrics: tuple, metrics: dict) -> dict:
    """Compile all segment statistics into a single dict."""
    vmaf_score, vmaf_min, vmaf_max = vmaf_metrics
    peak_memory_kb = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    resolution_category, output_width = get_resolution_category(segment)
    
    return {
        'segment': segment.name,
        'encoding_time': encoding_time,
        'crop_filter': crop_filter or "none",
        'vmaf_score': vmaf_score,
        'vmaf_min': vmaf_min,
        'vmaf_max': vmaf_max,
        'peak_memory_kb': peak_memory_kb,
        'peak_memory_bytes': peak_memory_kb * 1024,
        'resolution_category': resolution_category,
        **metrics
    }

def log_segment_progress(stats: dict, output_logs: list, segment_name: str,
                      vmaf_score: Optional[float] = None,
                      vmaf_min: Optional[float] = None,
                      vmaf_max: Optional[float] = None) -> None:
    """Log segment encoding progress and stats."""
    def capture_log(msg, *args):
        formatted = msg % args
        logger.info(formatted)
        output_logs.append(formatted)

    if vmaf_score is not None:
        capture_log("Segment analysis complete: %s – VMAF Avg: %.2f, Min: %.2f, Max: %.2f (CRF target determined)",
                   segment_name, vmaf_score, vmaf_min, vmaf_max)
    else:
        capture_log("Segment analysis complete: %s – No VMAF scores parsed", segment_name)
    
    capture_log("Segment encoding complete: %s", segment_name)
    capture_log("  Duration: %.2fs", stats['duration'])
    capture_log("  Size: %.2f MB", stats['size_mb'])
    capture_log("  Bitrate: %.2f kbps", stats['bitrate_kbps'])
    capture_log("  Encoding time: %.2fs (%.2fx realtime)", 
             stats['encoding_time'], stats['speed_factor'])
    capture_log("  Resolution: %s @ %s", stats['resolution'], stats['framerate'])

def handle_segment_retry(error: Exception, segment: Path, output_segment: Path, 
                         crop_filter: Optional[str], retry_count: int,
                         is_hdr: bool, dv_flag: bool) -> tuple[dict, list[str]]:
    # Use a local import to avoid circular dependency
    from .segment_encoding import encode_segment
    MAX_RETRIES = 2
    logger.warning("Encoding segment %s failed on attempt %d with error: %s", 
                   segment.name, retry_count, str(error))
    if retry_count < MAX_RETRIES:
        retry_count += 1
        logger.info("Retrying segment %s (attempt %d)", segment.name, retry_count)
        return encode_segment(segment, output_segment, crop_filter, retry_count, is_hdr, dv_flag)
    else:
        raise SegmentEncodingError(
            f"Segment {segment.name} failed after {MAX_RETRIES + 1} attempts: {error}",
            module="segment_encoding"
        )
