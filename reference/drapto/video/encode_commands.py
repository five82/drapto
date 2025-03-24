"""Command building functions for segment encoding

Responsibilities:
- Build ab-av1 encode commands with appropriate parameters
- Handle retry parameters and HDR/DV settings
- Configure VMAF sampling and analysis options
"""

import logging
from pathlib import Path
from typing import List, Optional

from ..config import (
    PRESET, TARGET_VMAF, TARGET_VMAF_HDR,
    SVT_PARAMS, VMAF_SAMPLE_COUNT, VMAF_SAMPLE_LENGTH
)

logger = logging.getLogger(__name__)

def build_encode_command(
    segment: Path,
    output_segment: Path,
    crop_filter: Optional[str],
    retry_count: int,
    is_hdr: bool,
    dv_flag: bool
) -> List[str]:
    """Build the ab-av1 encode command with appropriate parameters."""
    # Get retry-specific parameters
    sample_count, sample_duration_value, min_vmaf_value = get_retry_params(retry_count, is_hdr)

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

def get_retry_params(retry_count: int, is_hdr: bool) -> tuple[int, int, str]:
    """Get encoding parameters based on retry count"""
    if retry_count == 0:
        return 3, 1, str(TARGET_VMAF_HDR if is_hdr else TARGET_VMAF)
    elif retry_count == 1:
        return 4, 2, str(TARGET_VMAF_HDR if is_hdr else TARGET_VMAF)
    else:  # retry_count == 2
        return 4, 2, "95"  # Force highest quality
