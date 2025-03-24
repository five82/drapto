"""Unit tests for command builder functionality

This test suite verifies the construction of ffmpeg commands
for various encoding operations including segmentation and audio encoding.
"""

import unittest
from pathlib import Path
from drapto.video.command_builders import build_segment_command, build_audio_encode_command

class TestCommandBuilders(unittest.TestCase):
    """Test cases for command builder utilities"""
    def test_build_segment_command(self):
        input_file = Path("/tmp/input.mkv")
        segments_dir = Path("/tmp/segments")
        scenes = [1.0, 2.5, 4.0]
        hw_opt = "-hwaccel cuda"
        cmd = build_segment_command(input_file, segments_dir, scenes, hw_opt)
        self.assertIn("ffmpeg", cmd)
        self.assertIn("-segment_times", cmd)
        # Verify that scene times are properly formatted (e.g., "1.00,2.50,4.00")
        self.assertTrue(any("1.00" in item for item in cmd))
    
    def test_build_audio_encode_command(self):
        input_file = Path("/tmp/input.mkv")
        output_file = Path("/tmp/output.mkv")
        cmd = build_audio_encode_command(input_file, output_file, 0, "128k")
        self.assertIn("libopus", " ".join(cmd))
        self.assertIn("128k", " ".join(cmd))
    
if __name__ == "__main__":
    unittest.main()
