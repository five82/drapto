import unittest
from pathlib import Path
from unittest.mock import patch, MagicMock
from drapto.ffprobe.media import (
    get_video_info, get_audio_info, get_format_info,
    get_subtitle_info, get_all_audio_info, get_duration,
    get_resolution, get_audio_channels
)
from drapto.ffprobe.exec import MetadataError

class TestFFProbeMedia(unittest.TestCase):
    @patch("drapto.ffprobe.exec.ffprobe_query")
    def test_get_video_info(self, mock_query):
        # Simulate a successful ffprobe video query
        mock_query.return_value = {
            "streams": [{
                "codec_name": "av1",
                "width": 1920,
                "height": 1080,
                "r_frame_rate": "30/1"
            }]
        }
        info = get_video_info(Path("/tmp/fake.mkv"))
        self.assertEqual(info["streams"][0]["codec_name"], "av1")
        self.assertEqual(info["streams"][0]["width"], 1920)

    @patch("drapto.ffprobe.exec.ffprobe_query")
    def test_get_audio_info(self, mock_query):
        # Simulate audio query returning a valid dictionary
        mock_query.return_value = {"streams": [{"codec_name": "aac", "channels": 2}]}
        info = get_audio_info(Path("/tmp/fake.mkv"), stream_index=0)
        self.assertEqual(info["streams"][0]["codec_name"], "aac")
        self.assertEqual(info["streams"][0]["channels"], 2)

    @patch("drapto.ffprobe.exec.ffprobe_query")
    def test_get_format_info(self, mock_query):
        # Simulate format info query
        mock_query.return_value = {"format": {"duration": "120.0"}}
        info = get_format_info(Path("/tmp/fake.mkv"))
        self.assertEqual(info["format"]["duration"], "120.0")

    @patch("drapto.ffprobe.exec.ffprobe_query")
    def test_get_subtitle_info(self, mock_query):
        # Simulate subtitle info query: return a dict with a streams key
        mock_query.return_value = {"streams": [{"codec_name": "subrip"}]}
        info = get_subtitle_info(Path("/tmp/fake.mkv"))
        self.assertIsInstance(info, dict)
        self.assertIn("streams", info)

    @patch("drapto.ffprobe.exec.ffprobe_query")
    def test_get_all_audio_info(self, mock_query):
        # Simulate returning a list of audio streams
        mock_query.return_value = {"streams": [{"codec_name": "aac"}, {"codec_name": "aac"}]}
        info = get_all_audio_info(Path("/tmp/fake.mkv"))
        self.assertIsInstance(info, list)
        self.assertEqual(len(info), 2)

    @patch("drapto.ffprobe.exec.ffprobe_query")
    def test_get_duration(self, mock_query):
        # Simulate returning duration as float
        mock_query.return_value = {"format": {"duration": "120.0"}}
        duration = get_duration(Path("/tmp/fake.mkv"), stream_type="format")
        self.assertAlmostEqual(duration, 120.0)

    @patch("drapto.ffprobe.exec.ffprobe_query")
    def test_get_resolution(self, mock_query):
        # Simulate resolution query by returning a stream with width and height
        mock_query.return_value = {"streams": [{"width": 1920, "height": 1080}]}
        width, height = get_resolution(Path("/tmp/fake.mkv"))
        self.assertEqual(width, 1920)
        self.assertEqual(height, 1080)

    @patch("drapto.ffprobe.exec.ffprobe_query")
    def test_get_audio_channels(self, mock_query):
        # Simulate audio channels query with correct conversion
        mock_query.return_value = {"streams": [{"channels": "2"}]}
        channels = get_audio_channels(Path("/tmp/fake.mkv"), track_index=0)
        self.assertEqual(channels, 2)

if __name__ == "__main__":
    unittest.main()
