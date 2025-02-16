"""Hardware acceleration detection and configuration (for decoding only)"""

import logging
import platform
from typing import Optional

from ..utils import run_cmd

log = logging.getLogger(__name__)

def check_hardware_acceleration() -> Optional[str]:
    """
    Check if decoding acceleration is supported on macOS.
    Returns 'videotoolbox' if available, otherwise None.
    """
    if platform.system() == "Darwin":
        try:
            # VideoToolbox is used for decoding on macOS
            result = run_cmd(["ffmpeg", "-hide_banner", "-hwaccels"])
            if "videotoolbox" in result.stdout:
                log.info("Found VideoToolbox for hardware decoding")
                return "videotoolbox"
        except Exception as e:
            log.warning("Error checking hardware acceleration: %s", e)
    log.info("No supported hardware decoding acceleration found")
    return None

def get_hwaccel_options(accel_type: Optional[str] = None) -> str:
    """
    Return hardware acceleration options for decoding only.
    Only returns '-hwaccel videotoolbox' if accel_type is 'videotoolbox'.
    """
    if accel_type == "videotoolbox":
        return "-hwaccel videotoolbox"
    return ""
