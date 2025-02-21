"""Configuration settings for the drapto encoding pipeline"""

import os
from pathlib import Path

# Get script directory 
SCRIPT_DIR = Path(__file__).parent.resolve()

# Working root directory in /tmp
WORKING_ROOT = Path(os.environ.get("DRAPTO_WORKDIR", "/tmp/drapto"))

# Paths for binaries
FFMPEG = SCRIPT_DIR / "ffmpeg"
FFPROBE = SCRIPT_DIR / "ffprobe"

# INPUT_DIR and OUTPUT_DIR are provided via command line
INPUT_DIR = None
OUTPUT_DIR = None

# LOG_DIR: user definable with default of "$HOME/drapto_logs"
LOG_DIR = Path(os.environ.get("DRAPTO_LOG_DIR", str(Path.home() / "drapto_logs")))

# Encoding settings
PRESET = 6
CRF_SD = 25      # For videos with width <= 1280 (720p)
CRF_HD = 25      # For videos with width <= 1920 (1080p) 
CRF_UHD = 29     # For videos with width > 1920 (4K and above)
SVT_PARAMS = "tune=0:film-grain=0:film-grain-denoise=0"
PIX_FMT = "yuv420p10le"

# Hardware acceleration options
HWACCEL_OPTS = ""

# Dolby Vision detection flag
IS_DOLBY_VISION = False

# Cropping settings
DISABLE_CROP = False

# Standard encoding settings
ENABLE_STANDARD_ENCODING = True
# Fixed segmentation mode is removed; using only dynamic, scene-based segmentation.
TARGET_VMAF = 93
VMAF_SAMPLE_COUNT = 3
VMAF_SAMPLE_LENGTH = 1

# Scene detection settings
SCENE_THRESHOLD = 40.0  # Content detection threshold (higher = less sensitive)
MIN_SCENE_INTERVAL = 5.0  # Minimum time between scene changes (seconds)
CLUSTER_WINDOW = 2.0  # Window size in seconds for clustering nearby scene changes
TARGET_SEGMENT_LENGTH = 15.0  # Target segment duration in seconds
MAX_SEGMENT_LENGTH = 30.0  # Maximum segment length before forcing a split
ADAPTIVE_CLUSTER_WINDOW = 2.0  # Default window for adaptive scene clustering

# Temporary directories for chunked encoding
SEGMENTS_DIR = WORKING_ROOT / "segments"
ENCODED_SEGMENTS_DIR = WORKING_ROOT / "encoded_segments" 
WORKING_DIR = WORKING_ROOT / "working"

# Logging configuration
LOG_LEVEL = "INFO"  # Default logging level; valid values: DEBUG, INFO, WARNING, ERROR, CRITICAL

# Create log directory
LOG_DIR.mkdir(parents=True, exist_ok=True)
