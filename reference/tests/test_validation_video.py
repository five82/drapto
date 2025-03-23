import unittest
from pathlib import Path
from unittest.mock import patch
from drapto.validation.validation_video import (
    validate_video_stream, validate_crop_dimensions, validate_av_sync, validate_container
)
from drapto.exceptions import ValidationError

# Dummy implementations to simulate ffprobe responses
def dummy_get_video_info(file):
    return {
        "codec_name": "av1",
        "width": 1920,
        "height": 1080,
        "pix_fmt": "yuv420p",
        "r_frame_rate": "30/1",
        "start_time": 0.0,
        "duration": 120.0
    }

def dummy_get_format_info(file):
    return {"duration": "120.0"}

def dummy_get_audio_info(file, index):
    return {"start_time": 0.0, "duration": "120.0"}

def dummy_get_resolution(file):
    return (1920, 1080)

class TestValidationVideo(unittest.TestCase):
    @patch("drapto.validation.validation_video.get_video_info", side_effect=dummy_get_video_info)
    def test_validate_video_stream_success(self, mock_video_info):
        report = []
        # Should execute without raising error
        validate_video_stream(Path("/tmp/fake.mkv"), Path("/tmp/fake_output.mkv"), report)
        self.assertTrue(report[0].startswith("Video:"))

    @patch("drapto.validation.validation_video.get_resolution", side_effect=dummy_get_resolution)
    def test_validate_crop_dimensions_success(self, mock_resolution):
        report = []
        # With identical resolution values the function should not add error messages
        try:
            validate_crop_dimensions(Path("/tmp/input.mkv"), Path("/tmp/input.mkv"), report)
        except Exception:
            self.fail("validate_crop_dimensions raised Exception unexpectedly!")
        self.assertEqual(len(report), 0)

    @patch("drapto.validation.validation_video.get_video_info", side_effect=dummy_get_video_info)
    @patch("drapto.validation.validation_video.get_audio_info", side_effect=dummy_get_audio_info)
    def test_validate_av_sync_success(self, mock_audio_info, mock_video_info):
        report = []
        validate_av_sync(Path("/tmp/output.mkv"), report)
        self.assertIn("AV sync validated", report[0])

    @patch("drapto.validation.validation_video.get_format_info", side_effect=dummy_get_format_info)
    def test_validate_container_success(self, mock_format_info):
        report = []
        validate_container(Path("/tmp/output.mkv"), report)
        self.assertIn("Container:", report[0])

if __name__ == "__main__":
    unittest.main()
