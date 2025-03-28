"""Utility functions for the drapto encoding pipeline

Responsibilities:
  - Execute shell commands with or without progress reporting.
  - Format file sizes, timestamps, and paths.
  - Check for required dependencies and perform cleanup of working directories.
  - Log command execution details to aid debugging.
"""

import subprocess
import sys
import logging
from datetime import datetime
from pathlib import Path
from typing import List, Union, Optional

from .exceptions import DependencyError

# (Removed basicConfig call so that __main__.py can fully control logging configuration)
logger = logging.getLogger(__name__)

def run_cmd_with_progress(cmd: List[str], total_duration: Optional[float] = None, log_interval: float = 3.0) -> int:
    """
    Run an ffmpeg command with the -progress pipe:1 option.
    It reads progress output and logs a concise update (percentage and fps)
    only when progress increases by a given interval (in percent).

    Args:
        cmd: Command list (without the -progress flag)
        total_duration: Total duration of the video in seconds.
        log_interval: Minimum percentage interval for logging progress updates.
        
    Returns:
        The process return code.
    """
    import time

    # Append the progress flag so that ffmpeg writes progress information to stdout
    cmd_with_progress = cmd + ["-progress", "pipe:1"]
    logger.debug("Running ffmpeg command with progress:\n%s", " \\\n    ".join(cmd_with_progress))
    
    process = subprocess.Popen(cmd_with_progress, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    
    last_logged_percent = 0.0
    current_fps = "N/A"
    
    while True:
        line = process.stdout.readline()
        if line == "":
            if process.poll() is not None:
                break
            else:
                time.sleep(0.1)
                continue
        line = line.strip()
        
        # Parse out_time= field (e.g., out_time=00:01:23.456)
        if total_duration and line.startswith("out_time="):
            try:
                # Extract the HH:MM:SS.xxx string
                time_str = line.split("=")[1]
                parts = time_str.split(":")
                if len(parts) == 3:
                    hours, minutes, seconds = parts
                    current_time = int(hours)*3600 + int(minutes)*60 + float(seconds)
                    percent = (current_time / total_duration) * 100
                    # Calculate estimated remaining time
                    remaining_seconds = total_duration - current_time
                    if remaining_seconds < 0:
                        remaining_seconds = 0
                    remaining_hours = int(remaining_seconds // 3600)
                    remaining_minutes = int((remaining_seconds % 3600) // 60)
                    remaining_secs = int(remaining_seconds % 60)
                    formatted_remaining = f"{remaining_hours:02d}h {remaining_minutes:02d}m {remaining_secs:02d}s"
                    if percent - last_logged_percent >= log_interval:
                        logger.info("Progress: %.2f%%, fps: %s, remaining: %s", percent, current_fps, formatted_remaining)
                        last_logged_percent = percent
                else:
                    logger.debug("Unexpected out_time format: %s", time_str)
            except Exception as e:
                logger.debug("Error parsing out_time: %s", e)
        elif line.startswith("fps="):
            try:
                # Extract fps value
                current_fps = line.split("=")[1]
            except Exception as e:
                logger.debug("Error parsing fps: %s", e)
        elif line.startswith("progress="):
            # When we see progress=end, we log final status.
            if line.strip() == "progress=end":
                logger.info("Progress: 100%%, fps: %s", current_fps)
        # Optionally, you can also log other key lines at debug level.
    process.wait()
    return process.returncode

def run_cmd(cmd: List[str], capture_output: bool = True,
            check: bool = True) -> subprocess.CompletedProcess:
    """Run a command and handle errors"""
    logger.debug("Running command: %s", " ".join(cmd))
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
    """Get current timestamp in YYYY-MM-DD_HH-MM-SS format"""
    return datetime.now().strftime("%Y-%m-%d_%H-%M-%S")

def format_size(size: int) -> str:
    """Format file size for display"""
    for unit in ['B', 'KiB', 'MiB', 'GiB', 'TiB']:
        if size < 1024:
            return f"{size:.1f}{unit}"
        size /= 1024
    return f"{size:.1f}TiB"

def check_dependencies() -> bool:
    """Check for required dependencies"""
    required = ['ffmpeg', 'ffprobe', 'mediainfo']
    import shutil
    missing = []
    for cmd in required:
        if shutil.which(cmd) is None:
            missing.append(cmd)
    
    if missing:
        raise DependencyError(
            f"Missing dependencies: {', '.join(missing)}",
            module="dependencies"
        )
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
