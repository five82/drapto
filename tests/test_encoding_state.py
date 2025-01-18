#!/usr/bin/env python3

"""
Tests for encoding state management
"""

import json
import os
import shutil
import tempfile
import time
import unittest
from pathlib import Path

from drapto.scripts.common.encoding_state import (
    EncodingState,
    JobStatus,
    SegmentStatus,
    EncodingJob,
    EncodingStats,
    Segment,
)

class TestEncodingState(unittest.TestCase):
    def setUp(self):
        # Create a temporary directory for test state
        self.temp_dir = tempfile.mkdtemp()
        self.state = EncodingState(self.temp_dir)
        
        # Sample job data
        self.input_file = "/path/to/input.mkv"
        self.output_file = "/path/to/output.mkv"
        self.strategy = "chunked"
    
    def tearDown(self):
        # Clean up temporary directory
        shutil.rmtree(self.temp_dir)
    
    def test_create_job(self):
        """Test creating a new encoding job"""
        job_id = self.state.create_job(self.input_file, self.output_file, self.strategy)
        
        # Verify job was created
        self.assertIn(job_id, self.state.jobs)
        
        job = self.state.get_job(job_id)
        self.assertEqual(job.input_file, self.input_file)
        self.assertEqual(job.output_file, self.output_file)
        self.assertEqual(job.strategy, self.strategy)
        self.assertEqual(job.status, JobStatus.PENDING)
        self.assertEqual(len(job.segments), 0)
    
    def test_add_segment(self):
        """Test adding segments to a job"""
        # Create a job
        job_id = self.state.create_job(self.input_file, self.output_file, self.strategy)
        
        # Add segments
        segments = [
            {
                "index": 0,
                "input": "/path/to/segment_0.mkv",
                "output": "/path/to/encoded_0.mkv",
                "start": 0.0,
                "duration": 10.0,
            },
            {
                "index": 1,
                "input": "/path/to/segment_1.mkv",
                "output": "/path/to/encoded_1.mkv",
                "start": 10.0,
                "duration": 10.0,
            },
        ]
        
        for seg in segments:
            self.state.add_segment(
                job_id,
                seg["index"],
                seg["input"],
                seg["output"],
                seg["start"],
                seg["duration"],
            )
        
        # Verify segments were added
        job = self.state.get_job(job_id)
        self.assertEqual(len(job.segments), 2)
        self.assertEqual(job.stats.segment_count, 2)
        
        # Verify segment data
        for seg in segments:
            segment = job.segments[seg["index"]]
            self.assertEqual(segment.input_path, seg["input"])
            self.assertEqual(segment.output_path, seg["output"])
            self.assertEqual(segment.start_time, seg["start"])
            self.assertEqual(segment.duration, seg["duration"])
            self.assertEqual(segment.status, SegmentStatus.PENDING)
    
    def test_update_segment_status(self):
        """Test updating segment status"""
        # Create a job with a segment
        job_id = self.state.create_job(self.input_file, self.output_file, self.strategy)
        self.state.add_segment(
            job_id,
            0,
            "/path/to/segment_0.mkv",
            "/path/to/encoded_0.mkv",
            0.0,
            10.0,
        )
        
        # Update segment status
        self.state.update_segment_status(job_id, 0, SegmentStatus.ENCODING)
        segment = self.state.get_segment(job_id, 0)
        self.assertEqual(segment.status, SegmentStatus.ENCODING)
        
        # Complete segment
        self.state.update_segment_status(job_id, 0, SegmentStatus.COMPLETED)
        segment = self.state.get_segment(job_id, 0)
        self.assertEqual(segment.status, SegmentStatus.COMPLETED)
        
        # Verify completed segment count
        job = self.state.get_job(job_id)
        self.assertEqual(job.stats.completed_segments, 1)
    
    def test_state_persistence(self):
        """Test that state is properly saved and loaded"""
        # Create a job with segments
        job_id = self.state.create_job(self.input_file, self.output_file, self.strategy)
        self.state.add_segment(
            job_id,
            0,
            "/path/to/segment_0.mkv",
            "/path/to/encoded_0.mkv",
            0.0,
            10.0,
        )
        self.state.update_segment_status(job_id, 0, SegmentStatus.COMPLETED)
        
        # Create a new state instance
        new_state = EncodingState(self.temp_dir)
        
        # Verify job was loaded
        self.assertIn(job_id, new_state.jobs)
        
        job = new_state.get_job(job_id)
        self.assertEqual(job.input_file, self.input_file)
        self.assertEqual(len(job.segments), 1)
        
        segment = job.segments[0]
        self.assertEqual(segment.status, SegmentStatus.COMPLETED)
        self.assertEqual(job.stats.completed_segments, 1)

    def test_job_progress(self):
        """Test updating job progress"""
        # Create a job
        job_id = self.state.create_job(self.input_file, self.output_file, self.strategy)
        
        # Update progress
        self.state.update_job_progress(job_id, 50, 100, 30.0)
        
        # Get progress
        progress = self.state.get_progress(job_id)
        
        # Verify progress
        self.assertEqual(progress.current_frame, 50)
        self.assertEqual(progress.total_frames, 100)
        self.assertEqual(progress.percent, 50.0)
        self.assertEqual(progress.fps, 30.0)
        self.assertGreater(progress.started_at, 0)
        self.assertGreater(progress.updated_at, 0)
        self.assertAlmostEqual(progress.eta_seconds, 1.67, places=2)  # (100-50)/30
        
        # Verify job stats
        job = self.state.get_job(job_id)
        self.assertEqual(job.stats.total_frames, 100)
        self.assertEqual(job.stats.encoded_frames, 50)
    
    def test_segment_progress(self):
        """Test updating segment progress"""
        # Create a job with segments
        job_id = self.state.create_job(self.input_file, self.output_file, self.strategy)
        
        # Add two segments
        segments = [
            {
                "index": 0,
                "input": "/path/to/segment_0.mkv",
                "output": "/path/to/encoded_0.mkv",
                "frames": 100,
            },
            {
                "index": 1,
                "input": "/path/to/segment_1.mkv",
                "output": "/path/to/encoded_1.mkv",
                "frames": 100,
            },
        ]
        
        for seg in segments:
            self.state.add_segment(
                job_id,
                seg["index"],
                seg["input"],
                seg["output"]
            )
        
        # Update progress for first segment
        self.state.update_segment_progress(job_id, 0, 50, 100, 30.0)
        
        # Get segment progress
        progress = self.state.get_segment_progress(job_id, 0)
        
        # Verify segment progress
        self.assertEqual(progress.current_frame, 50)
        self.assertEqual(progress.total_frames, 100)
        self.assertEqual(progress.percent, 50.0)
        self.assertEqual(progress.fps, 30.0)
        self.assertGreater(progress.started_at, 0)
        self.assertGreater(progress.updated_at, 0)
        self.assertAlmostEqual(progress.eta_seconds, 1.67, places=2)  # (100-50)/30
        
        # Update progress for second segment
        self.state.update_segment_progress(job_id, 1, 25, 100, 25.0)
        
        # Get job progress
        progress = self.state.get_progress(job_id)
        
        # Verify overall job progress
        self.assertEqual(progress.current_frame, 75)  # 50 + 25
        self.assertEqual(progress.total_frames, 200)  # 100 + 100
        self.assertEqual(progress.percent, 37.5)  # (75/200) * 100
        self.assertEqual(progress.fps, 25.0)  # Latest FPS
        
        # Verify job stats
        job = self.state.get_job(job_id)
        self.assertEqual(job.stats.total_frames, 200)
        self.assertEqual(job.stats.encoded_frames, 75)
    
    def test_progress_persistence(self):
        """Test that progress is properly saved and loaded"""
        # Create a job with a segment
        job_id = self.state.create_job(self.input_file, self.output_file, self.strategy)
        self.state.add_segment(
            job_id,
            0,
            "/path/to/segment_0.mkv",
            "/path/to/encoded_0.mkv"
        )
        
        # Update progress
        self.state.update_segment_progress(job_id, 0, 50, 100, 30.0)
        
        # Create a new state instance
        new_state = EncodingState(self.temp_dir)
        
        # Get progress from new instance
        progress = new_state.get_segment_progress(job_id, 0)
        
        # Verify progress was loaded
        self.assertEqual(progress.current_frame, 50)
        self.assertEqual(progress.total_frames, 100)
        self.assertEqual(progress.percent, 50.0)
        self.assertEqual(progress.fps, 30.0)
        
        # Get job progress
        progress = new_state.get_progress(job_id)
        
        # Verify job progress was loaded
        self.assertEqual(progress.current_frame, 50)
        self.assertEqual(progress.total_frames, 100)
        self.assertEqual(progress.percent, 50.0)
        self.assertEqual(progress.fps, 30.0)

if __name__ == "__main__":
    unittest.main()
