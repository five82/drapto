"""Configuration settings for the drapto encoding pipeline"""

import os
from pathlib import Path

# Get script directory 
SCRIPT_DIR = Path(__file__).parent.resolve()

# Paths
FFMPEG = SCRIPT_DIR / "ffmpeg"
FFPROBE = SCRIPT_DIR / "ffprobe"
INPUT_DIR = SCRIPT_DIR / "videos" / "input"
OUTPUT_DIR = SCRIPT_DIR / "videos" / "output"
LOG_DIR = SCRIPT_DIR / "videos" / "logs"

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

# Temporary directories for chunked encoding
SEGMENTS_DIR = SCRIPT_DIR / "videos" / "segments"
ENCODED_SEGMENTS_DIR = SCRIPT_DIR / "videos" / "encoded_segments"
WORKING_DIR = SCRIPT_DIR / "videos" / "working"

# Create required directories
for directory in [INPUT_DIR, OUTPUT_DIR, LOG_DIR, SEGMENTS_DIR, 
                 ENCODED_SEGMENTS_DIR, WORKING_DIR]:
    directory.mkdir(parents=True, exist_ok=True)
