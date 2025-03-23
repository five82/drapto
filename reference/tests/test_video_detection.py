"""Unit tests for video detection functionality

This test suite verifies the behavior of video detection utilities,
including crop detection and Dolby Vision identification.
"""

import unittest
import subprocess
from unittest.mock import patch, MagicMock
from pathlib import Path
from drapto.video.detection import (
    detect_crop,
    detect_dolby_vision,
    _determine_crop_threshold,
    _run_hdr_blackdetect,
    _get_video_properties
)
from drapto.ffprobe.exec import MetadataError

class TestVideoDetection(unittest.TestCase):
    def setUp(self):
        self.test_file = Path("/tmp/test.mkv")
        self.mock_probe = MagicMock()
        self.mock_probe.get.side_effect = ["1920", "1080", 120.0]  # width, height, duration

    @patch("drapto.video.detection._get_video_properties")
    @patch("drapto.video.detection._run_cropdetect")
    def test_detect_crop_success(self, mock_cropdetect, mock_properties):
        """Test successful crop detection"""
        mock_properties.return_value = (
            {"transfer": "bt709", "primaries": "bt709", "space": "bt709"},
            (1920, 1080),
            120.0
        )
        mock_cropdetect.return_value = "crop=1920:800:0:140"
        
        crop_filter, is_hdr = detect_crop(self.test_file)
        self.assertEqual(crop_filter, "crop=1920:800:0:140")
        self.assertFalse(is_hdr)

    @patch("drapto.video.detection._get_video_properties")
    @patch("drapto.video.detection._run_hdr_blackdetect")
    @patch("drapto.video.detection._run_cropdetect")
    def test_detect_crop_hdr(self, mock_cropdetect, mock_hdr_blackdetect, mock_properties):
        """Test HDR crop detection"""
        mock_properties.return_value = (
            {"transfer": "smpte2084", "primaries": "bt2020", "space": "bt2020"},
            (3840, 2160),
            120.0
        )
        mock_hdr_blackdetect.return_value = 128
        mock_cropdetect.return_value = "crop=3840:1600:0:280"
        
        crop_filter, is_hdr = detect_crop(self.test_file)
        self.assertEqual(crop_filter, "crop=3840:1600:0:280")
        self.assertTrue(is_hdr)

    @patch("drapto.video.detection.subprocess.run")
    def test_detect_dolby_vision_success(self, mock_run):
        """Test Dolby Vision detection"""
        mock_run.return_value.stdout = "Dolby Vision"
        result = detect_dolby_vision(self.test_file)
        self.assertTrue(result)

    @patch("drapto.video.detection.subprocess.run")
    def test_detect_dolby_vision_failure(self, mock_run):
        """Test Dolby Vision detection failure"""
        mock_run.side_effect = subprocess.CalledProcessError(1, "mediainfo")
        result = detect_dolby_vision(self.test_file)
        self.assertFalse(result)

    def test_determine_crop_threshold(self):
        """Test crop threshold determination"""
        # Test SDR
        threshold, is_hdr = _determine_crop_threshold("bt709", "bt709", "bt709")
        self.assertEqual(threshold, 16)
        self.assertFalse(is_hdr)
        
        # Test HDR
        threshold, is_hdr = _determine_crop_threshold("smpte2084", "bt2020", "bt2020")
        self.assertEqual(threshold, 128)
        self.assertTrue(is_hdr)

    @patch("drapto.video.detection.run_cmd")
    def test_run_hdr_blackdetect(self, mock_run_cmd):
        """Test HDR black level detection"""
        # Need a value where int(black_level * 3 / 2) = 128
        # int(85.333) * 3 / 2 = 85 * 3 / 2 = 127.5 -> 128 would be 85.34
        mock_run_cmd.return_value.stderr = "black_level: 85.34\nblack_level: 85.34"
        result = _run_hdr_blackdetect(self.test_file, 128)
        # The calculation result should be int(85.34) * 3 / 2 = 85 * 3 / 2 = 127.5 -> 127
        # But line 56 in detection.py rounds it: return int(black_level * 3 / 2)
        self.assertEqual(result, 127)

    @patch("drapto.video.detection.probe_session")
    @patch("drapto.video.detection.get_resolution")
    def test_get_video_properties(self, mock_get_resolution, mock_session):
        """Test video property extraction"""
        mock_session.return_value.__enter__.return_value = self.mock_probe
        # Mock color properties
        self.mock_probe.get.side_effect = [
            "bt709",  # color_transfer
            "bt709",  # color_primaries
            "bt709",  # color_space
            120.0     # duration
        ]
        mock_get_resolution.return_value = (1920, 1080)
        
        props, dims, duration = _get_video_properties(self.test_file)
        self.assertEqual(props["transfer"], "bt709")
        self.assertEqual(props["primaries"], "bt709")
        self.assertEqual(props["space"], "bt709")
        self.assertEqual(dims, (1920, 1080))
        self.assertEqual(duration, 120.0)

if __name__ == "__main__":
    unittest.main()
