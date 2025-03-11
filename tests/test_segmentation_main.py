"""Unit tests for video segmentation functionality"""

def fake_get_media_property(*args, **kwargs):
    """
    Fake get_media_property for tests. Expected signature:
      get_media_property(path: Path, stream_type: str, property_name: str, stream_index: int = 0)
    """
    # Unpack expected arguments.
    # We assume: args[0] is path, args[1] is stream_type, args[2] is property_name,
    # and args[3] is stream_index if provided.
    path = args[0]
    stream_type = args[1] if len(args) > 1 else kwargs.get('stream_type')
    property_name = args[2] if len(args) > 2 else kwargs.get('property_name')
    stream_index = args[3] if len(args) > 3 else kwargs.get('stream_index', 0)
    
    # If the requested property is duration
    if property_name == 'duration':
        if path == Path("/tmp/test.mkv"):
            return 120.0
        else:
            return 30.0
    elif property_name == 'codec_name':
        return "av1"
    elif property_name == 'width':
        return 1920
    elif property_name == 'height':
        return 1080
    elif property_name == 'channels':
        return 2
    return None

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

    @patch('drapto.ffprobe.media.get_media_property', side_effect=fake_get_media_property)
    @patch('drapto.video.segmentation.segmentation_main.SegmentationJob')
    @patch('drapto.ffprobe.session.probe_session')
    def test_segment_video_success(self, mock_session, mock_job, mock_get_media):
        """Test successful video segmentation"""
        from unittest.mock import MagicMock
        
        # Create mock segment files with ordering support
        fake_seg1 = MagicMock(spec=Path)
        fake_seg1.name = "seg1.mkv"
        fake_seg1.exists.return_value = True
        fake_stat = MagicMock()
        fake_stat.st_size = 2048  # >1024 bytes
        fake_seg1.stat.return_value = fake_stat
        fake_seg1.__lt__ = lambda other: fake_seg1.name < other.name  # Enable ordering

        fake_seg2 = MagicMock(spec=Path)
        fake_seg2.name = "seg2.mkv"
        fake_seg2.exists.return_value = True
        fake_seg2.stat.return_value = fake_stat
        fake_seg2.__lt__ = lambda other: fake_seg2.name < other.name  # Enable ordering

        mock_session.return_value.__enter__.return_value = self.mock_probe
        mock_job.return_value.execute.return_value = None

        # Patch scene detection
        with patch('drapto.video.segmentation.segmentation_main.detect_scenes') as mock_detect:
            mock_detect.return_value = [0.0, 30.0, 60.0, 90.0, 120.0]
            # Patch Path.glob to return our mock segment files
            with patch('pathlib.Path.glob', return_value=[fake_seg1, fake_seg2]):
                result = segment_video(self.test_file)
                self.assertTrue(result)
                mock_job.return_value.execute.assert_called_once()

    @patch('drapto.ffprobe.media.get_media_property', side_effect=lambda path, stream_type, property_name, stream_index=0, test_file=Path("/tmp/test.mkv"):
        120.0 if (property_name == 'duration' and path == test_file) else
        (30.0 if property_name == 'duration' else
         ("av1" if property_name == 'codec_name' else
          (1920 if property_name == 'width' else
           (1080 if property_name == 'height' else
            (2 if property_name == 'channels' else None)))))
    )
    @patch('drapto.ffprobe.session.probe_session')
    def test_validate_segments_success(self, mock_session, mock_get_media):
        """Test successful segment validation"""
        from unittest.mock import MagicMock
        
        # Create mock segment files with ordering support
        fake_seg1 = MagicMock(spec=Path)
        fake_seg1.name = "seg1.mkv"
        fake_seg1.exists.return_value = True
        fake_stat = MagicMock()
        fake_stat.st_size = 2048  # >1024 bytes
        fake_seg1.stat.return_value = fake_stat
        fake_seg1.__lt__ = lambda other: fake_seg1.name < other.name  # Enable ordering

        fake_seg2 = MagicMock(spec=Path)
        fake_seg2.name = "seg2.mkv"
        fake_seg2.exists.return_value = True
        fake_seg2.stat.return_value = fake_stat
        fake_seg2.__lt__ = lambda other: fake_seg2.name < other.name  # Enable ordering

        mock_session.return_value.__enter__.return_value = self.mock_probe

        # Patch Path.glob to return our mock segment files
        with patch('pathlib.Path.glob', return_value=[fake_seg1, fake_seg2]):
            self.assertTrue(validate_segments(self.test_file))

    @patch('drapto.ffprobe.media.get_media_property', side_effect=fake_get_media_property)
    @patch('drapto.ffprobe.session.probe_session')
    def test_validate_segments_failure(self, mock_session, mock_get_media):
        """Test segment validation failure"""
        # Define the mock inside the test method where self is available
        mock_get_duration = patch(
            'drapto.ffprobe.media.get_duration',
            side_effect=lambda path, *args, **kwargs: 120.0 if path == self.test_file else 200.0
        ).start()
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
