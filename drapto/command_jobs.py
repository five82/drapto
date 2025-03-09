"""
command_jobs.py

Defines a base class for command jobs and specialized implementations for
various pipeline steps (e.g. segmentation, audio encoding, muxing).
"""

import logging
from typing import List, Optional
from pathlib import Path
from .utils import run_cmd, run_cmd_with_progress

logger = logging.getLogger(__name__)

class CommandJob:
    """
    Base class representing a command job.
    
    Attributes:
        cmd (List[str]): The command to run
    """
    def __init__(self, cmd: List[str]):
        self.cmd = cmd

    def execute(self) -> any:
        """
        Execute the stored command.
        
        Returns:
            The result of run_cmd().
            
        Raises:
            subprocess.CalledProcessError if the command fails.
        """
        logger.debug("Executing command: %s", " ".join(self.cmd))
        result = run_cmd(self.cmd)
        return result

class ProgressCommandJob(CommandJob):
    """Job for running commands that support progress reporting."""
    
    def execute(self, total_duration: Optional[float] = None, log_interval: float = 3.0) -> int:
        """Execute with progress reporting."""
        return run_cmd_with_progress(self.cmd, total_duration, log_interval)

class SegmentationJob(CommandJob):
    """Job for running video segmentation."""
    pass

class AudioEncodeJob(ProgressCommandJob):
    """Job for encoding an audio track."""
    pass

class MuxJob(CommandJob):
    """Job for muxing audio and video tracks."""
    pass

class ConcatJob(CommandJob):
    """Job for concatenating video segments."""
    pass
