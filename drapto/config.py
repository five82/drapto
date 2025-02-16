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

# Chunked encoding settings
ENABLE_CHUNKED_ENCODING = True
SEGMENT_LENGTH = 15
TARGET_VMAF = 93
VMAF_SAMPLE_COUNT = 3
VMAF_SAMPLE_LENGTH = 1

# Scene detection settings
SCENE_THRESHOLD = 0.4  # Lowering threshold increases sensitivity, producing more scene changes and ~10s segments
MIN_SCENE_INTERVAL = 5.0  # Minimum seconds between scene changes

# Temporary directories for chunked encoding
SEGMENTS_DIR = WORKING_ROOT / "segments"
ENCODED_SEGMENTS_DIR = WORKING_ROOT / "encoded_segments" 
WORKING_DIR = WORKING_ROOT / "working"

# Create log directory
LOG_DIR.mkdir(parents=True, exist_ok=True)
