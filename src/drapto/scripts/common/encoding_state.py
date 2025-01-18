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
    input_path: str
    output_path: str
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
    input_file: str
    output_file: str
    status: JobStatus
    strategy: str
    stats: EncodingStats
    segments: Dict[int, Segment] = field(default_factory=dict)
    progress: Progress = field(default_factory=Progress)
    error_message: Optional[str] = None

class EncodingState:
    """Manages state for all encoding jobs"""
    
    def __init__(self, state_dir: str):
        """Initialize encoding state manager
        
        Args:
            state_dir: Directory to store state files
        """
        self.state_dir = Path(state_dir)
        self.state_dir.mkdir(parents=True, exist_ok=True)
        self.jobs: Dict[str, EncodingJob] = {}
        self._load_state()
    
    def create_job(self, input_file: str, output_file: str, strategy: str) -> str:
        """Create a new encoding job
        
        Args:
            input_file: Path to input video file
            output_file: Path to output video file
            strategy: Name of encoding strategy to use
            
        Returns:
            job_id: Unique identifier for the job
        """
        job_id = str(int(time.time()))
        job = EncodingJob(
            job_id=job_id,
            input_file=input_file,
            output_file=output_file,
            status=JobStatus.PENDING,
            strategy=strategy,
            stats=EncodingStats(start_time=time.time())
        )
        self.jobs[job_id] = job
        self._save_state()
        return job_id
    
    def add_segment(self, job_id: str, index: int, input_path: str, output_path: str,
                   start_time: float = 0.0, duration: float = 0.0) -> None:
        """Add a new segment to a job
        
        Args:
            job_id: Job identifier
            index: Segment index
            input_path: Path to input segment file
            output_path: Path to output segment file
            start_time: Start time of segment in seconds
            duration: Duration of segment in seconds
        """
        if job_id not in self.jobs:
            raise KeyError(f"Job {job_id} not found")
        
        job = self.jobs[job_id]
        segment = Segment(
            index=index,
            input_path=input_path,
            output_path=output_path,
            start_time=start_time,
            duration=duration
        )
        job.segments[index] = segment
        job.stats.segment_count = len(job.segments)
        self._save_state()
    
    def update_segment_status(self, job_id: str, index: int, status: SegmentStatus,
                            error: str = None) -> None:
        """Update status of a segment
        
        Args:
            job_id: Job identifier
            index: Segment index
            status: New segment status
            error: Optional error message if segment failed
        """
        if job_id not in self.jobs:
            raise KeyError(f"Job {job_id} not found")
        
        job = self.jobs[job_id]
        if index not in job.segments:
            raise KeyError(f"Segment {index} not found in job {job_id}")
        
        segment = job.segments[index]
        segment.status = status
        if error:
            segment.error_message = error
        
        # Update completed segment count
        job.stats.completed_segments = sum(
            1 for s in job.segments.values()
            if s.status == SegmentStatus.COMPLETED
        )
        self._save_state()
    
    def get_segments(self, job_id: str) -> List[Segment]:
        """Get all segments for a job
        
        Args:
            job_id: Job identifier
            
        Returns:
            List[Segment]: List of all segments
        """
        if job_id not in self.jobs:
            raise KeyError(f"Job {job_id} not found")
        
        return list(self.jobs[job_id].segments.values())
    
    def get_segment(self, job_id: str, index: int) -> Segment:
        """Get a specific segment
        
        Args:
            job_id: Job identifier
            index: Segment index
            
        Returns:
            Segment: Segment information
        """
        if job_id not in self.jobs:
            raise KeyError(f"Job {job_id} not found")
        
        job = self.jobs[job_id]
        if index not in job.segments:
            raise KeyError(f"Segment {index} not found in job {job_id}")
        
        return job.segments[index]

    def update_job_status(self, job_id: str, status: JobStatus, error: str = None) -> None:
        """Update status of an encoding job
        
        Args:
            job_id: Job identifier
            status: New job status
            error: Optional error message if job failed
        """
        if job_id not in self.jobs:
            raise KeyError(f"Job {job_id} not found")
        
        job = self.jobs[job_id]
        job.status = status
        if error:
            job.error_message = error
        if status == JobStatus.COMPLETED:
            job.stats.end_time = time.time()
        self._save_state()
    
    def update_job_stats(self, job_id: str, **kwargs) -> None:
        """Update statistics for an encoding job
        
        Args:
            job_id: Job identifier
            **kwargs: Statistics to update (input_size, output_size, vmaf_score)
        """
        if job_id not in self.jobs:
            raise KeyError(f"Job {job_id} not found")
        
        job = self.jobs[job_id]
        for key, value in kwargs.items():
            if hasattr(job.stats, key):
                setattr(job.stats, key, value)
        self._save_state()
    
    def update_job_progress(self, job_id: str, current_frame: int, total_frames: int,
                          fps: float = 0.0) -> None:
        """Update progress of an encoding job
        
        Args:
            job_id: Job identifier
            current_frame: Current frame being encoded
            total_frames: Total frames to encode
            fps: Current encoding speed in frames per second
        """
        if job_id not in self.jobs:
            raise KeyError(f"Job {job_id} not found")
        
        job = self.jobs[job_id]
        now = time.time()
        
        # Update job progress
        if job.progress.started_at == 0.0:
            job.progress.started_at = now
        
        job.progress.current_frame = current_frame
        job.progress.total_frames = total_frames
        job.progress.fps = fps
        job.progress.updated_at = now
        
        # Calculate overall progress
        if total_frames > 0:
            job.progress.percent = (current_frame / total_frames) * 100
            
            # Calculate ETA
            if fps > 0:
                remaining_frames = total_frames - current_frame
                job.progress.eta_seconds = remaining_frames / fps
        
        # Update job stats
        job.stats.total_frames = total_frames
        job.stats.encoded_frames = current_frame
        
        self._save_state()
    
    def update_segment_progress(self, job_id: str, index: int, current_frame: int,
                              total_frames: int, fps: float = 0.0) -> None:
        """Update progress of a segment
        
        Args:
            job_id: Job identifier
            index: Segment index
            current_frame: Current frame being encoded
            total_frames: Total frames to encode
            fps: Current encoding speed in frames per second
        """
        if job_id not in self.jobs:
            raise KeyError(f"Job {job_id} not found")
        
        job = self.jobs[job_id]
        if index not in job.segments:
            raise KeyError(f"Segment {index} not found in job {job_id}")
        
        segment = job.segments[index]
        now = time.time()
        
        # Update segment progress
        if segment.progress.started_at == 0.0:
            segment.progress.started_at = now
        
        segment.progress.current_frame = current_frame
        segment.progress.total_frames = total_frames
        segment.progress.fps = fps
        segment.progress.updated_at = now
        
        # Calculate segment progress
        if total_frames > 0:
            segment.progress.percent = (current_frame / total_frames) * 100
            
            # Calculate ETA
            if fps > 0:
                remaining_frames = total_frames - current_frame
                segment.progress.eta_seconds = remaining_frames / fps
        
        # Update segment total frames
        segment.total_frames = total_frames
        
        # Update overall job progress
        total_job_frames = sum(s.total_frames for s in job.segments.values())
        encoded_frames = sum(s.progress.current_frame for s in job.segments.values())
        self.update_job_progress(job_id, encoded_frames, total_job_frames, fps)
    
    def get_progress(self, job_id: str) -> Progress:
        """Get progress information for a job
        
        Args:
            job_id: Job identifier
            
        Returns:
            Progress: Job progress information
        """
        if job_id not in self.jobs:
            raise KeyError(f"Job {job_id} not found")
        
        return self.jobs[job_id].progress
    
    def get_segment_progress(self, job_id: str, index: int) -> Progress:
        """Get progress information for a segment
        
        Args:
            job_id: Job identifier
            index: Segment index
            
        Returns:
            Progress: Segment progress information
        """
        if job_id not in self.jobs:
            raise KeyError(f"Job {job_id} not found")
        
        job = self.jobs[job_id]
        if index not in job.segments:
            raise KeyError(f"Segment {index} not found in job {job_id}")
        
        return job.segments[index].progress
    
    def get_job(self, job_id: str) -> EncodingJob:
        """Get information about an encoding job
        
        Args:
            job_id: Job identifier
            
        Returns:
            EncodingJob: Job information
        """
        if job_id not in self.jobs:
            raise KeyError(f"Job {job_id} not found")
        return self.jobs[job_id]
    
    def get_all_jobs(self) -> List[EncodingJob]:
        """Get information about all encoding jobs
        
        Returns:
            List[EncodingJob]: List of all jobs
        """
        return list(self.jobs.values())
    
    def _save_state(self) -> None:
        """Save state to disk"""
        state_file = self.state_dir / "state.json"
        
        # Convert jobs to dict for serialization
        jobs_dict = {}
        for job_id, job in self.jobs.items():
            job_dict = asdict(job)
            
            # Convert enums to strings
            job_dict["status"] = job.status.value
            
            # Convert segments
            segments_dict = {}
            for idx, segment in job.segments.items():
                segment_dict = asdict(segment)
                segment_dict["status"] = segment.status.value
                segments_dict[idx] = segment_dict
            job_dict["segments"] = segments_dict
            
            jobs_dict[job_id] = job_dict
        
        with open(state_file, "w") as f:
            json.dump(jobs_dict, f, indent=2)
    
    def _load_state(self) -> None:
        """Load state from disk"""
        state_file = self.state_dir / "state.json"
        if not state_file.exists():
            return
        
        with open(state_file) as f:
            jobs_dict = json.load(f)
        
        # Convert dict back to jobs
        for job_id, job_dict in jobs_dict.items():
            # Convert status string back to enum
            job_dict["status"] = JobStatus(job_dict["status"])
            
            # Convert segments
            segments = {}
            for idx, segment_dict in job_dict["segments"].items():
                # Convert segment status string back to enum
                segment_dict["status"] = SegmentStatus(segment_dict["status"])
                
                # Convert progress dict to Progress object
                segment_dict["progress"] = Progress(**segment_dict["progress"])
                
                # Create Segment object
                segments[int(idx)] = Segment(**segment_dict)
            job_dict["segments"] = segments
            
            # Convert progress dict to Progress object
            job_dict["progress"] = Progress(**job_dict["progress"])
            
            # Convert stats dict to EncodingStats object
            job_dict["stats"] = EncodingStats(**job_dict["stats"])
            
            # Create EncodingJob object
            self.jobs[job_id] = EncodingJob(**job_dict)
