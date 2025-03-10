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
    @patch("drapto.ffprobe.media.probe_session")
    def test_get_video_info(self, mock_probe_session):
        # Create a fake probe session that returns desired video info.
        from unittest.mock import MagicMock
        fake_probe = MagicMock()
        fake_probe.get.side_effect = lambda prop, stream_type="video", stream_index=0: {
            "codec_name": "av1",
            "width": 1920,
            "height": 1080,
            "r_frame_rate": "30/1",
            "start_time": 0.0,
            "duration": 120.0,
            "pix_fmt": "yuv420p",
            "color_transfer": None,
            "color_primaries": None,
            "color_space": None
        }[prop]
        mock_probe_session.return_value.__enter__.return_value = fake_probe

        info = get_video_info(Path("/tmp/fake.mkv"))
        self.assertEqual(info["codec_name"], "av1")
        self.assertEqual(info["width"], 1920)

    @patch("drapto.ffprobe.media.probe_session")
    def test_get_audio_info(self, mock_probe_session):
        # Create a fake probe session that returns desired audio info.
        from unittest.mock import MagicMock
        fake_probe = MagicMock()
        fake_probe.get.side_effect = lambda prop, stream_type="audio", stream_index=0: {
            "codec_name": "aac",
            "channels": 2,
            "bit_rate": None,
            "start_time": 0.0,
            "duration": 120.0
        }[prop]
        mock_probe_session.return_value.__enter__.return_value = fake_probe

        info = get_audio_info(Path("/tmp/fake.mkv"), stream_index=0)
        self.assertEqual(info["codec_name"], "aac")
        self.assertEqual(info["channels"], 2)

    @patch("drapto.ffprobe.media.ffprobe_query")
    def test_get_format_info(self, mock_query):
        # Simulate format info query
        mock_query.return_value = {"format": {"duration": "120.0"}}
        info = get_format_info(Path("/tmp/fake.mkv"))
        self.assertEqual(info["format"]["duration"], "120.0")

    @patch("drapto.ffprobe.media.ffprobe_query")
    def test_get_subtitle_info(self, mock_query):
        # Simulate subtitle info query: return a dict with a streams key
        mock_query.return_value = {"streams": [{"codec_name": "subrip"}]}
        info = get_subtitle_info(Path("/tmp/fake.mkv"))
        self.assertIsInstance(info, dict)
        self.assertIn("streams", info)

    @patch("drapto.ffprobe.media.ffprobe_query")
    def test_get_all_audio_info(self, mock_query):
        # Simulate returning a list of audio streams
        mock_query.return_value = {"streams": [{"codec_name": "aac"}, {"codec_name": "aac"}]}
        info = get_all_audio_info(Path("/tmp/fake.mkv"))
        self.assertIsInstance(info, list)
        self.assertEqual(len(info), 2)

    @patch("drapto.ffprobe.exec.subprocess.run")
    def test_get_duration(self, mock_run):
        from subprocess import CompletedProcess
        # Simulate a successful ffprobe call for format duration.
        mock_run.return_value = CompletedProcess(
            args=["ffprobe"],
            returncode=0,
            stdout="120.0\n",
            stderr=""
        )
        duration = get_duration(Path("/tmp/fake.mkv"), stream_type="format")
        self.assertAlmostEqual(duration, 120.0)

    @patch("drapto.ffprobe.exec.subprocess.run")
    def test_get_resolution(self, mock_run):
        from subprocess import CompletedProcess
        # First call: width. Second call: height.
        mock_run.side_effect = [
            CompletedProcess(args=["ffprobe"], returncode=0, stdout="1920\n", stderr=""),
            CompletedProcess(args=["ffprobe"], returncode=0, stdout="1080\n", stderr=""),
        ]
        width, height = get_resolution(Path("/tmp/fake.mkv"))
        self.assertEqual(width, 1920)
        self.assertEqual(height, 1080)

    @patch("drapto.ffprobe.exec.subprocess.run")
    def test_get_audio_channels(self, mock_run):
        from subprocess import CompletedProcess
        # Simulate a successful ffprobe call returning "2" for channels.
        mock_run.return_value = CompletedProcess(
            args=["ffprobe"],
            returncode=0,
            stdout="2\n",
            stderr=""
        )
        channels = get_audio_channels(Path("/tmp/fake.mkv"), track_index=0)
        self.assertEqual(channels, 2)

if __name__ == "__main__":
    unittest.main()
