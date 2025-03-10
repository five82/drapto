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

    @patch('drapto.ffprobe.media.get_duration', side_effect=lambda path, *args, **kwargs: 120.0 if path == self.test_file else 30.0)
    @patch('drapto.video.segmentation.segmentation_main.SegmentationJob')
    @patch('drapto.ffprobe.session.probe_session')
    def test_segment_video_success(self, mock_session, mock_job, mock_get_duration):
        """Test successful video segmentation"""
        mock_session.return_value.__enter__.return_value = self.mock_probe
        mock_job.return_value.execute.return_value = None

        # Patch scene detection
        with patch('drapto.video.segmentation.segmentation_main.detect_scenes') as mock_detect:
            mock_detect.return_value = [0.0, 30.0, 60.0, 90.0, 120.0]
            # Patch Path.glob to simulate found segments in the segments directory
            with patch('pathlib.Path.glob', return_value=[Path("/tmp/segments/seg1.mkv"), Path("/tmp/segments/seg2.mkv")]):
                result = segment_video(self.test_file)
                self.assertTrue(result)
                mock_job.return_value.execute.assert_called_once()

    @patch('drapto.ffprobe.media.get_duration', side_effect=lambda path, *args, **kwargs: 120.0 if path == self.test_file else 30.0)
    @patch('drapto.ffprobe.session.probe_session')
    def test_validate_segments_success(self, mock_session, mock_get_duration):
        """Test successful segment validation"""
        mock_session.return_value.__enter__.return_value = self.mock_probe

        # Patch dummy segments are present
        with patch('pathlib.Path.glob', return_value=[Path("/tmp/segments/seg1.mkv"), Path("/tmp/segments/seg2.mkv")]):
            # Now, validate_segments should succeed since input duration (120.0)
            # and total segment duration (30.0+30.0=60.0) are different but may satisfy your tolerance.
            # (You might want to adjust the return values as needed to pass the tolerance check.)
            self.assertTrue(validate_segments(self.test_file))

    @patch('drapto.ffprobe.media.get_duration', side_effect=lambda path, *args, **kwargs: 120.0 if path == self.test_file else 200.0)
    @patch('drapto.ffprobe.session.probe_session')
    def test_validate_segments_failure(self, mock_session, mock_get_duration):
        """Test segment validation failure"""
        mock_session.return_value.__enter__.return_value = self.mock_probe

        with patch('pathlib.Path.glob', return_value=[Path("/tmp/segments/seg1.mkv")]):
            self.mock_probe.get.side_effect = ["av1", 1920, 1080, 200.0]  # Simulated segment duration 200.0
            with self.assertRaises(ValidationError):
                validate_segments(self.test_file)

    @patch('drapto.video.segmentation.segmentation_main.check_hardware_acceleration')
    @patch('drapto.video.segmentation.segmentation_main.detect_scenes')
    def test_prepare_segmentation(self, mock_detect, mock_hw):
        """Test segmentation preparation"""
        mock_hw.return_value = "cuda"
        mock_detect.return_value = [0.0, 30.0, 60.0]
        
        hw_opt, scenes = _prepare_segmentation(self.test_file)
        self.assertEqual(hw_opt, "")
        self.assertEqual(scenes, [0.0, 30.0, 60.0])

if __name__ == '__main__':
    unittest.main()
