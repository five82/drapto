"""Unit tests for muxer functionality

This test suite verifies the behavior of the muxer module,
including successful muxing and error conditions.
"""

import unittest
from unittest.mock import patch, MagicMock
from pathlib import Path
from drapto.muxer import mux_tracks
from drapto.exceptions import MuxingError
from drapto.ffprobe.exec import MetadataError

class TestMuxer(unittest.TestCase):
    def setUp(self):
        self.video_track = Path("/tmp/video.mkv")
        self.audio_tracks = [Path("/tmp/audio1.mkv"), Path("/tmp/audio2.mkv")]
        self.output_file = Path("/tmp/output.mkv")

    @patch("drapto.command_jobs.run_cmd")
    @patch("drapto.muxer.get_video_info")
    @patch("drapto.muxer.get_audio_info")
    @patch("drapto.muxer.get_duration")
    def test_mux_tracks_success(self, mock_duration, mock_audio_info, mock_video_info, mock_run_cmd):
        """Test successful muxing operation"""
        # Setup mock returns
        mock_video_info.return_value = {"start_time": 0.0, "duration": 120.0}
        mock_audio_info.return_value = {"start_time": 0.0, "duration": 120.0}
        mock_duration.side_effect = [120.0, 120.0]  # Video and audio durations
        
        # Should complete without raising
        mux_tracks(self.video_track, self.audio_tracks, self.output_file)
        
        # Verify command was executed
        mock_run_cmd.assert_called()

    @patch("drapto.command_jobs.run_cmd")
    @patch("drapto.muxer.get_video_info")
    def test_mux_tracks_av_sync_failure(self, mock_video_info, mock_run_cmd):
        """Test AV sync validation failure"""
        # Setup mock returns with sync mismatch
        mock_video_info.return_value = {"start_time": 0.0, "duration": 120.0}
        
        # Mock audio info with different duration
        with patch("drapto.muxer.get_audio_info") as mock_audio_info:
            mock_audio_info.return_value = {"start_time": 0.0, "duration": 125.0}
            
            with self.assertRaises(MuxingError):
                mux_tracks(self.video_track, self.audio_tracks, self.output_file)

    @patch("drapto.command_jobs.run_cmd")
    def test_mux_tracks_command_failure(self, mock_run_cmd):
        """Test command execution failure"""
        mock_run_cmd.side_effect = Exception("Command failed")
        
        with self.assertRaises(MuxingError):
            mux_tracks(self.video_track, self.audio_tracks, self.output_file)

    @patch("drapto.muxer.run_cmd")
    @patch("drapto.muxer.get_video_info")
    def test_mux_tracks_metadata_error(self, mock_video_info, mock_run_cmd):
        """Test metadata retrieval failure"""
        mock_video_info.side_effect = MetadataError("Test error")
        
        with self.assertRaises(MuxingError):
            mux_tracks(self.video_track, self.audio_tracks, self.output_file)

if __name__ == "__main__":
    unittest.main()
