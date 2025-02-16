"""Utility functions for the drapto encoding pipeline"""

import subprocess
import sys
import logging
from datetime import datetime
from pathlib import Path
from typing import List, Union, Optional

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

def run_cmd_with_progress(cmd: List[str]) -> int:
    """
    Run an ffmpeg command with the -progress pipe:1 option,
    reading progress status line by line and logging it.
    """
    import time

    # Append the progress flag so that ffmpeg writes progress output to stdout
    cmd_with_progress = cmd + ["-progress", "pipe:1"]
    
    logger.info("Running ffmpeg command with progress:\n%s", " \\\n    ".join(cmd_with_progress))
    
    process = subprocess.Popen(cmd_with_progress, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    
    # Continuously read progress lines
    while True:
        # Read a single line from stdout
        line = process.stdout.readline()
        if line == "":
            # If the process has terminated, break
            if process.poll() is not None:
                break
            else:
                time.sleep(0.1)
                continue
        line = line.strip()
        # ffmpeg progress output is in KEY=VALUE format; you can parse it
        # For this example, we'll just log the progress line.
        logger.info("ffmpeg progress: %s", line)
    
    # Wait for any remaining output and get the return code
    process.wait()
    return process.returncode

def run_cmd_interactive(cmd: List[str]) -> int:
    """Run a command interactively so that its output (including progress bar)
    is printed directly to the console."""
    logger.info("Running interactive command: %s", " ".join(cmd))
    process = subprocess.Popen(cmd, stdout=sys.stdout, stderr=sys.stderr)
    return process.wait()

def run_cmd(cmd: List[str], capture_output: bool = True, 
            check: bool = True) -> subprocess.CompletedProcess:
    """Run a command and handle errors"""
    logger.info("Running command: %s", " ".join(cmd))
    try:
        result = subprocess.run(
            cmd,
            capture_output=capture_output,
            check=check,
            text=True
        )
        if result.stdout:
            logger.debug("Command stdout: %s", result.stdout)
        if result.stderr:
            logger.debug("Command stderr: %s", result.stderr)
        return result
    except subprocess.CalledProcessError as e:
        logger.error("Command failed: %s", " ".join(cmd))
        logger.error("Error output: %s", e.stderr)
        raise

def get_file_size(path: Union[str, Path]) -> int:
    """Get file size in bytes"""
    return Path(path).stat().st_size

def get_timestamp() -> str:
    """Get current timestamp in YYYYMMDD_HHMMSS format"""
    return datetime.now().strftime("%Y%m%d_%H%M%S")

def format_size(size: int) -> str:
    """Format file size for display"""
    for unit in ['B', 'KiB', 'MiB', 'GiB', 'TiB']:
        if size < 1024:
            return f"{size:.1f}{unit}"
        size /= 1024
    return f"{size:.1f}TiB"

def check_dependencies() -> bool:
    """Check for required dependencies"""
    required = ['ffmpeg', 'ffprobe', 'mediainfo', 'bc']
    
    for cmd in required:
        try:
            subprocess.run(['which', cmd], 
                         capture_output=True, 
                         check=True)
        except subprocess.CalledProcessError:
            logger.error(f"Required dependency not found: {cmd}")
            return False
    
    return True

def cleanup_working_dirs():
    """Clean up all encoding working directories and files in the working root."""
    from .config import WORKING_ROOT
    import shutil
    try:
        if WORKING_ROOT.exists():
            shutil.rmtree(WORKING_ROOT)
            logger.info("Cleaned up working directories in %s", WORKING_ROOT)
    except Exception as e:
        logger.error("Failed to clean up working directories: %s", e)
