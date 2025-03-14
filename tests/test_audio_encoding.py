"""Unit tests for audio encoding functionality

This test suite verifies the behavior of audio track encoding,
including stereo channel handling and command execution.
"""

import unittest
from unittest.mock import patch, MagicMock
from pathlib import Path
from drapto.audio.encoding import encode_audio_track

class TestAudioEncoding(unittest.TestCase):
    """Test cases for audio encoding"""
    def setUp(self):
        # Use temporary file paths to simulate real files
        self.input_file = Path("/tmp/fake_input.mkv")
        self.fake_output = Path("/tmp/fake_output.mkv")
    
    @patch("drapto.validation.validate_encoded_audio", return_value=None)
    @patch("drapto.audio.encoding.get_duration", return_value=60.0)
    @patch("drapto.audio.encoding.get_audio_channels")
    @patch("drapto.command_jobs.run_cmd_with_progress", return_value=0)
    def test_encode_audio_track_stereo(self, mock_run_cmd_with_progress, mock_get_channels, mock_get_duration, mock_validate):
        # Set up to simulate stereo channel (2 channels)
        mock_get_channels.return_value = 2

        output = encode_audio_track(self.input_file, 1)
        self.assertTrue(isinstance(output, Path))
        mock_run_cmd_with_progress.assert_called()
    
if __name__ == "__main__":
    unittest.main()
