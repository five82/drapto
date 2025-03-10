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
    
    @patch("drapto.audio.encoding.get_audio_channels")
    @patch("drapto.audio.encoding.run_cmd_with_progress", return_value=0)
    def test_encode_audio_track_stereo(self, mock_run_cmd_with_progress, mock_get_channels):
        # Set up to simulate stereo channel (2 channels)
        mock_get_channels.return_value = 2
        
        # The function uses build_audio_encode_command - you may want to patch it if needed.
        # Simulate a successful run.
        
        # Patch validate_encoded_audio if needed
        with patch("drapto.validation.validation_audio.validate_encoded_audio") as mock_validate:
            output = encode_audio_track(self.input_file, 1)
            self.assertTrue(isinstance(output, Path))
            # Validate the bitrate chosen, if the command builder is visible.
            # You can also check that run_cmd_with_progress was called.
            mock_run_cmd_with_progress.assert_called()
    
if __name__ == "__main__":
    unittest.main()
