#!/usr/bin/env python3

"""
Encoding State Management

This module handles state tracking for video encoding jobs, including:
- Job status and progress
- Input/output file information
- Encoding statistics
- Segment tracking
- Progress tracking
"""

import json
import os
import time
from dataclasses import dataclass, asdict, field
from enum import Enum
from pathlib import Path
from typing import Dict, List, Optional

from ...config import Settings
from ...utils.paths import (
    ensure_directory,
    normalize_path,
    get_relative_path,
    get_temp_path
)
from ...monitoring.paths import path_monitor

class JobStatus(Enum):
    """Status of an encoding job"""
    PENDING = "pending"
    INITIALIZING = "initializing"
    PREPARING = "preparing"
    ENCODING = "encoding"
    FINALIZING = "finalizing"
    COMPLETED = "completed"
    FAILED = "failed"

class SegmentStatus(Enum):
    """Status of a video segment"""
    PENDING = "pending"
    ENCODING = "encoding"
    COMPLETED = "completed"
    FAILED = "failed"

@dataclass
class Progress:
    """Progress information"""
    percent: float = 0.0
    current_frame: int = 0
    total_frames: int = 0
    fps: float = 0.0
    eta_seconds: float = 0.0
    started_at: float = 0.0
    updated_at: float = 0.0

@dataclass
class EncodingStats:
    """Statistics for an encoding job"""
    input_size: int = 0
    output_size: int = 0
    start_time: float = 0.0
    end_time: float = 0.0
    vmaf_score: float = 0.0
    segment_count: int = 0
    completed_segments: int = 0
    total_frames: int = 0
    encoded_frames: int = 0

@dataclass
class Segment:
    """Represents a video segment"""
    index: int
    input_path: Path
    output_path: Path
    status: SegmentStatus = SegmentStatus.PENDING
    start_time: float = 0.0
    duration: float = 0.0
    total_frames: int = 0
    progress: Progress = field(default_factory=Progress)
    error_message: Optional[str] = None

@dataclass
class EncodingJob:
    """Represents a single encoding job"""
    job_id: str
    input_file: Path
    output_file: Path
    status: JobStatus
    strategy: str
    stats: EncodingStats
    segments: Dict[int, Segment] = field(default_factory=dict)
    progress: Progress = field(default_factory=Progress)
    error_message: Optional[str] = None

class EncodingState:
    """Manages encoding state for multiple jobs."""
    
    def __init__(self, settings: Optional[Settings] = None):
        """Initialize encoding state manager.
        
        Args:
            settings: Optional settings object. If not provided, settings will be loaded
                    from environment variables.
        """
        # Initialize settings
        self.settings = settings or Settings.from_environment()
        
        # Ensure state directory exists
        self.state_dir = ensure_directory(self.settings.paths.temp_data_dir / "state")
        path_monitor.track_path(self.state_dir)
        
        # Initialize state
        self.jobs: Dict[str, EncodingJob] = {}
        self._load_state()
    
    def create_job(self, input_file: Path | str, output_file: Path | str, strategy: str) -> str:
        """Create a new encoding job.
        
        Args:
            input_file: Path to input file
            output_file: Path to output file
            strategy: Encoding strategy to use
            
        Returns:
            Job ID
            
        Raises:
            ValueError: If input file doesn't exist or output path is invalid
        """
        # Normalize paths
        input_file = normalize_path(input_file)
        output_file = normalize_path(output_file)
        
        # Track paths
        path_monitor.track_path(input_file)
        path_monitor.track_path(output_file)
        
        # Validate input file
        if not input_file.exists():
            raise ValueError(f"Input file does not exist: {input_file}")
        if not input_file.is_file():
            raise ValueError(f"Input path is not a file: {input_file}")
        
        # Create job ID
        job_id = f"job_{int(time.time())}_{os.urandom(4).hex()}"
        
        # Create job
        job = EncodingJob(
            job_id=job_id,
            input_file=input_file,
            output_file=output_file,
            status=JobStatus.PENDING,
            strategy=strategy,
            stats=EncodingStats(
                input_size=input_file.stat().st_size,
                start_time=time.time()
            )
        )
        
        # Add job to state
        self.jobs[job_id] = job
        self._save_state()
        
        return job_id
    
    def add_segment(self, job_id: str, index: int, input_path: Path | str,
                   output_path: Path | str, start_time: float = 0.0,
                   duration: float = 0.0) -> None:
        """Add a segment to a job.
        
        Args:
            job_id: Job ID
            index: Segment index
            input_path: Path to input segment
            output_path: Path to output segment
            start_time: Segment start time in seconds
            duration: Segment duration in seconds
            
        Raises:
            ValueError: If job doesn't exist or paths are invalid
        """
        # Get job
        job = self.get_job(job_id)
        
        # Normalize paths
        input_path = normalize_path(input_path)
        output_path = normalize_path(output_path)
        
        # Track paths
        path_monitor.track_path(input_path)
        path_monitor.track_path(output_path)
        
        # Create segment
        segment = Segment(
            index=index,
            input_path=input_path,
            output_path=output_path,
            start_time=start_time,
            duration=duration
        )
        
        # Add segment to job
        job.segments[index] = segment
        job.stats.segment_count = len(job.segments)
        self._save_state()
    
    def update_segment_status(self, job_id: str, index: int,
                            status: SegmentStatus, error: str = None) -> None:
        """Update segment status.
        
        Args:
            job_id: Job ID
            index: Segment index
            status: New segment status
            error: Optional error message
            
        Raises:
            ValueError: If job or segment doesn't exist
        """
        # Get segment
        segment = self.get_segment(job_id, index)
        
        # Update status
        segment.status = status
        segment.error_message = error
        
        # Update job stats
        job = self.get_job(job_id)
        job.stats.completed_segments = sum(
            1 for s in job.segments.values()
            if s.status == SegmentStatus.COMPLETED
        )
        
        # Record path status
        if status == SegmentStatus.COMPLETED:
            path_monitor.record_access(segment.output_path)
        elif status == SegmentStatus.FAILED and error:
            path_monitor.record_error(segment.input_path, error)
        
        self._save_state()
    
    def get_segments(self, job_id: str) -> List[Segment]:
        """Get all segments for a job.
        
        Args:
            job_id: Job ID
            
        Returns:
            List of segments sorted by index
            
        Raises:
            ValueError: If job doesn't exist
        """
        job = self.get_job(job_id)
        return sorted(job.segments.values(), key=lambda s: s.index)
    
    def get_segment(self, job_id: str, index: int) -> Segment:
        """Get a specific segment.
        
        Args:
            job_id: Job ID
            index: Segment index
            
        Returns:
            Segment object
            
        Raises:
            ValueError: If job or segment doesn't exist
        """
        job = self.get_job(job_id)
        if index not in job.segments:
            raise ValueError(f"Segment {index} not found in job {job_id}")
        return job.segments[index]
    
    def update_job_status(self, job_id: str, status: JobStatus,
                         error: str = None) -> None:
        """Update job status.
        
        Args:
            job_id: Job ID
            status: New job status
            error: Optional error message
            
        Raises:
            ValueError: If job doesn't exist
        """
        # Get job
        job = self.get_job(job_id)
        
        # Update status
        job.status = status
        job.error_message = error
        
        # Update stats
        if status == JobStatus.COMPLETED:
            job.stats.end_time = time.time()
            if job.output_file.exists():
                job.stats.output_size = job.output_file.stat().st_size
                path_monitor.record_access(job.output_file)
        elif status == JobStatus.FAILED and error:
            path_monitor.record_error(job.input_file, error)
        
        self._save_state()
    
    def update_job_stats(self, job_id: str, **kwargs) -> None:
        """Update job statistics.
        
        Args:
            job_id: Job ID
            **kwargs: Statistics to update
            
        Raises:
            ValueError: If job doesn't exist
        """
        job = self.get_job(job_id)
        for key, value in kwargs.items():
            if hasattr(job.stats, key):
                setattr(job.stats, key, value)
        self._save_state()
    
    def update_job_progress(self, job_id: str, current_frame: int,
                          total_frames: int, fps: float = 0.0) -> None:
        """Update job progress.
        
        Args:
            job_id: Job ID
            current_frame: Current frame number
            total_frames: Total number of frames
            fps: Current frames per second
            
        Raises:
            ValueError: If job doesn't exist
        """
        job = self.get_job(job_id)
        
        # Initialize progress if needed
        if job.progress.started_at == 0:
            job.progress.started_at = time.time()
            job.stats.total_frames = total_frames
        
        # Update progress
        job.progress.current_frame = current_frame
        job.progress.total_frames = total_frames
        job.progress.fps = fps
        job.progress.updated_at = time.time()
        
        # Calculate percentage and ETA
        if total_frames > 0:
            job.progress.percent = (current_frame / total_frames) * 100
            if fps > 0:
                frames_remaining = total_frames - current_frame
                job.progress.eta_seconds = frames_remaining / fps
        
        # Update stats
        job.stats.encoded_frames = current_frame
        
        self._save_state()
    
    def update_segment_progress(self, job_id: str, index: int,
                              current_frame: int, total_frames: int,
                              fps: float = 0.0) -> None:
        """Update segment progress.
        
        Args:
            job_id: Job ID
            index: Segment index
            current_frame: Current frame number
            total_frames: Total number of frames
            fps: Current frames per second
            
        Raises:
            ValueError: If job or segment doesn't exist
        """
        segment = self.get_segment(job_id, index)
        
        # Initialize progress if needed
        if segment.progress.started_at == 0:
            segment.progress.started_at = time.time()
            segment.total_frames = total_frames
        
        # Update progress
        segment.progress.current_frame = current_frame
        segment.progress.total_frames = total_frames
        segment.progress.fps = fps
        segment.progress.updated_at = time.time()
        
        # Calculate percentage and ETA
        if total_frames > 0:
            segment.progress.percent = (current_frame / total_frames) * 100
            if fps > 0:
                frames_remaining = total_frames - current_frame
                segment.progress.eta_seconds = frames_remaining / fps
        
        self._save_state()
    
    def get_progress(self, job_id: str) -> Progress:
        """Get job progress.
        
        Args:
            job_id: Job ID
            
        Returns:
            Progress object
            
        Raises:
            ValueError: If job doesn't exist
        """
        job = self.get_job(job_id)
        return job.progress
    
    def get_segment_progress(self, job_id: str, index: int) -> Progress:
        """Get segment progress.
        
        Args:
            job_id: Job ID
            index: Segment index
            
        Returns:
            Progress object
            
        Raises:
            ValueError: If job or segment doesn't exist
        """
        segment = self.get_segment(job_id, index)
        return segment.progress
    
    def get_job(self, job_id: str) -> EncodingJob:
        """Get a job by ID.
        
        Args:
            job_id: Job ID
            
        Returns:
            EncodingJob object
            
        Raises:
            ValueError: If job doesn't exist
        """
        if job_id not in self.jobs:
            raise ValueError(f"Job not found: {job_id}")
        return self.jobs[job_id]
    
    def get_all_jobs(self) -> List[EncodingJob]:
        """Get all jobs.
        
        Returns:
            List of all jobs
        """
        return list(self.jobs.values())
    
    def _save_state(self) -> None:
        """Save state to disk."""
        state_file = self.state_dir / "state.json"
        path_monitor.track_path(state_file)
        
        # Convert state to JSON-serializable format
        state = {
            job_id: {
                **asdict(job),
                "input_file": str(job.input_file),
                "output_file": str(job.output_file),
                "segments": {
                    str(idx): {
                        **asdict(segment),
                        "input_path": str(segment.input_path),
                        "output_path": str(segment.output_path)
                    }
                    for idx, segment in job.segments.items()
                }
            }
            for job_id, job in self.jobs.items()
        }
        
        # Save state
        try:
            with state_file.open("w") as f:
                json.dump(state, f, indent=2)
            path_monitor.record_access(state_file)
        except Exception as e:
            path_monitor.record_error(state_file, str(e))
            raise
    
    def _load_state(self) -> None:
        """Load state from disk."""
        state_file = self.state_dir / "state.json"
        path_monitor.track_path(state_file)
        
        if not state_file.exists():
            return
        
        try:
            with state_file.open("r") as f:
                state = json.load(f)
            
            # Convert JSON data back to objects
            for job_id, job_data in state.items():
                # Convert paths
                job_data["input_file"] = Path(job_data["input_file"])
                job_data["output_file"] = Path(job_data["output_file"])
                
                # Convert segments
                segments = {}
                for idx, segment_data in job_data["segments"].items():
                    segment_data["input_path"] = Path(segment_data["input_path"])
                    segment_data["output_path"] = Path(segment_data["output_path"])
                    segments[int(idx)] = Segment(**segment_data)
                job_data["segments"] = segments
                
                # Create job object
                self.jobs[job_id] = EncodingJob(**job_data)
            
            path_monitor.record_access(state_file)
        except Exception as e:
            path_monitor.record_error(state_file, str(e))
            raise
