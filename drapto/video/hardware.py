"""Hardware acceleration detection and configuration"""

import logging
import platform
from typing import Optional

from ..utils import run_cmd

log = logging.getLogger(__name__)

def check_hardware_acceleration() -> Optional[str]:
    """
    Check for supported hardware acceleration
    
    Returns:
        Optional[str]: Hardware acceleration type if supported, None otherwise
    """
    if platform.system() == "Darwin":
        try:
            result = run_cmd(["ffmpeg", "-hide_banner", "-hwaccels"])
            if "videotoolbox" in result.stdout:
                log.info("Found VideoToolbox hardware acceleration")
                return "videotoolbox"
        except Exception as e:
            log.warning("Error checking hardware acceleration: %s", e)
    
    log.info("No supported hardware acceleration found")
    return None

def get_hwaccel_options(accel_type: Optional[str] = None) -> str:
    """
    Get ffmpeg hardware acceleration options
    
    Args:
        accel_type: Hardware acceleration type
        
    Returns:
        str: ffmpeg hardware acceleration options
    """
    if accel_type == "videotoolbox":
        return "-hwaccel videotoolbox"
    return ""
