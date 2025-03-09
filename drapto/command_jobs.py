"""
command_jobs.py

Defines a base class for command jobs and specialized implementations for
various pipeline steps (e.g. segmentation, audio encoding, muxing).
"""

import logging
import subprocess
from typing import List, Optional
from pathlib import Path
from .utils import run_cmd, run_cmd_with_progress
from .exceptions import (
    CommandExecutionError, SegmentationError,
    AudioEncodingError
)

logger = logging.getLogger(__name__)

class CommandJob:
    """
    Base class representing a command job.
    
    Attributes:
        cmd (List[str]): The command to run
    """
    def __init__(self, cmd: List[str]):
        self.cmd = cmd

    def execute(self) -> None:
        """
        Execute the stored command.
        
        Raises:
            CommandExecutionError: If command fails
        """
        logger.debug("Executing command: %s", " ".join(self.cmd))
        try:
            run_cmd(self.cmd)
        except subprocess.CalledProcessError as e:
            raise CommandExecutionError(
                f"Command failed: {e.cmd}",
                module="command_jobs"
            ) from e

class ProgressCommandJob(CommandJob):
    """Job for running commands that support progress reporting."""
    
    def execute(self, total_duration: Optional[float] = None, log_interval: float = 3.0) -> None:
        """Execute with progress reporting."""
        return_code = run_cmd_with_progress(self.cmd, total_duration, log_interval)
        if return_code != 0:
            raise CommandExecutionError(
                f"Command failed with exit code {return_code}",
                module="progress_command_job"
            )

class SegmentationJob(CommandJob):
    """Job for running video segmentation."""
    def execute(self) -> None:
        try:
            super().execute()
        except CommandExecutionError as e:
            raise SegmentationError(
                f"Segmentation failed: {str(e)}",
                module="segmentation"
            ) from e

class AudioEncodeJob(ProgressCommandJob):
    """Job for encoding an audio track."""
    def execute(self, total_duration: Optional[float] = None, log_interval: float = 3.0) -> None:
        try:
            super().execute(total_duration, log_interval)
        except CommandExecutionError as e:
            raise AudioEncodingError(
                f"Audio encoding failed: {str(e)}", 
                module="audio_encoding"
            ) from e

class MuxJob(CommandJob):
    """Job for muxing audio and video tracks."""
    pass

class ConcatJob(CommandJob):
    """Job for concatenating video segments."""
    pass
