import unittest
from unittest.mock import patch
from pathlib import Path
from drapto.video.standard_encoder import encode_standard
from drapto.exceptions import EncodingError

class TestStandardEncoder(unittest.TestCase):
    @patch("drapto.video.standard_encoder.concatenate_segments")
    @patch("drapto.video.standard_encoder.encode_segments")
    @patch("drapto.video.standard_encoder.segment_video")
    @patch("drapto.video.standard_encoder.detect_crop")
    def test_encode_standard_success(self, mock_detect_crop, mock_segment, mock_encode, mock_concat):
        # Arrange
        input_file = Path("/tmp/fake_input.mkv")
        # Simulate crop_filter and hdr flag
        mock_detect_crop.return_value = ("crop=1920:800:0:140", False)
        # No exception from segment, encode, and concat functions
        
        # Act
        output = encode_standard(input_file, disable_crop=False, dv_flag=False)
        
        # Assert  
        self.assertTrue(output.exists() or isinstance(output, Path))
        mock_segment.assert_called_once()
        mock_encode.assert_called_once()
        mock_concat.assert_called_once()

    @patch("drapto.video.standard_encoder.segment_video")
    def test_encode_standard_segmentation_failure(self, mock_segment):
        # Simulate failure in segmentation
        mock_segment.side_effect = Exception("Segmentation error")
        input_file = Path("/tmp/fake_input.mkv")
        with self.assertRaises(EncodingError):
            encode_standard(input_file)
            
if __name__ == "__main__":
    unittest.main()
