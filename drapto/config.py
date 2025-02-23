"""Configuration settings for the drapto encoding pipeline"""

import os
from pathlib import Path

# Get script directory
SCRIPT_DIR = Path(__file__).parent.resolve()

# Working root directory in /tmp
WORKING_ROOT = Path(os.environ.get("DRAPTO_WORKDIR", "/tmp/drapto"))


# INPUT_DIR and OUTPUT_DIR are provided via command line
INPUT_DIR = None
OUTPUT_DIR = None

# LOG_DIR: user definable with default of "$HOME/drapto_logs"
LOG_DIR = Path(os.environ.get("DRAPTO_LOG_DIR", str(Path.home() / "drapto_logs")))

# Encoding settings
PRESET = 6
SVT_PARAMS = "tune=0:film-grain=0:film-grain-denoise=0"
PIX_FMT = "yuv420p10le"

# Memory management settings
MEMORY_THRESHOLD = 0.7  # Lower threshold to reserve 30% free memory
MAX_MEMORY_TOKENS = 8  # Maximum concurrent memory tokens
TASK_STAGGER_DELAY = 0.2  # Delay between task submissions in seconds

# Hardware acceleration options
HWACCEL_OPTS = ""

# Dolby Vision detection flag
IS_DOLBY_VISION = False

# Cropping settings
DISABLE_CROP = False

# Standard encoding settings
# Fixed segmentation mode is removed; using only dynamic, scene-based segmentation.
TARGET_VMAF = 93
VMAF_SAMPLE_COUNT = 3
VMAF_SAMPLE_LENGTH = 1

# Scene detection settings
SCENE_THRESHOLD = 40.0  # Content detection threshold for SDR content
HDR_SCENE_THRESHOLD = 30.0  # Lower threshold for HDR content to yield more scenes
TARGET_MIN_SEGMENT_LENGTH = 5.0  # Minimum segment length (seconds)
MAX_SEGMENT_LENGTH = 15.0  # Maximum segment length before forcing a split

# Temporary directories for encoding
SEGMENTS_DIR = WORKING_ROOT / "segments"
ENCODED_SEGMENTS_DIR = WORKING_ROOT / "encoded_segments"
WORKING_DIR = WORKING_ROOT / "working"

# Logging configuration
LOG_LEVEL = "INFO"  # Default logging level; valid values: DEBUG, INFO, WARNING, ERROR, CRITICAL

# Create log directory
LOG_DIR.mkdir(parents=True, exist_ok=True)
