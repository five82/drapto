"""Unit tests for video segmentation functionality"""

import unittest
from unittest.mock import patch, MagicMock
from pathlib import Path
from drapto.video.segmentation.segmentation_main import (
    segment_video,
    validate_segments,
    _prepare_segmentation
)
from drapto.exceptions import SegmentationError, ValidationError

class TestSegmentationMain(unittest.TestCase):
    def setUp(self):
        self.test_file = Path("/tmp/test.mkv")
        self.mock_probe = MagicMock()
        self.mock_probe.get.side_effect = ["1920", "1080", 120.0]  # width, height, duration

    @patch('drapto.video.segmentation.segmentation_main.SegmentationJob')
    @patch('drapto.video.segmentation.segmentation_main.probe_session')
    def test_segment_video_success(self, mock_session, mock_job):
        """Test successful video segmentation"""
        mock_session.return_value.__enter__.return_value = self.mock_probe
        mock_job.return_value.execute.return_value = None
        
        # Mock scene detection
        with patch('drapto.video.segmentation.segmentation_main.detect_scenes') as mock_detect:
            mock_detect.return_value = [0.0, 30.0, 60.0, 90.0, 120.0]
            result = segment_video(self.test_file)
            self.assertTrue(result)
            mock_job.return_value.execute.assert_called_once()

    @patch('drapto.video.segmentation.segmentation_main.probe_session')
    def test_validate_segments_success(self, mock_session):
        """Test successful segment validation"""
        mock_session.return_value.__enter__.return_value = self.mock_probe
        
        # Mock segment files
        with patch('pathlib.Path.glob') as mock_glob:
            mock_glob.return_value = [
                Path("/tmp/segments/seg1.mkv"),
                Path("/tmp/segments/seg2.mkv")
            ]
            self.assertTrue(validate_segments(self.test_file))

    @patch('drapto.video.segmentation.segmentation_main.probe_session')
    def test_validate_segments_failure(self, mock_session):
        """Test segment validation failure"""
        mock_session.return_value.__enter__.return_value = self.mock_probe
        
        # Mock segment files with invalid duration
        with patch('pathlib.Path.glob') as mock_glob:
            mock_glob.return_value = [Path("/tmp/segments/seg1.mkv")]
            self.mock_probe.get.side_effect = ["av1", 1920, 1080, 200.0]  # Duration exceeds input
            
            with self.assertRaises(ValidationError):
                validate_segments(self.test_file)

    @patch('drapto.video.segmentation.segmentation_main.check_hardware_acceleration')
    @patch('drapto.video.segmentation.segmentation_main.detect_scenes')
    def test_prepare_segmentation(self, mock_detect, mock_hw):
        """Test segmentation preparation"""
        mock_hw.return_value = "cuda"
        mock_detect.return_value = [0.0, 30.0, 60.0]
        
        hw_opt, scenes = _prepare_segmentation(self.test_file)
        self.assertEqual(hw_opt, "-hwaccel cuda")
        self.assertEqual(scenes, [0.0, 30.0, 60.0])

if __name__ == '__main__':
    unittest.main()
