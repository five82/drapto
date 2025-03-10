"""Unit tests for segment encoding functionality"""

import unittest
from unittest.mock import patch, MagicMock
from pathlib import Path
from drapto.video.segment_encoding import (
    encode_segment,
    estimate_memory_weight,
    validate_encoded_segments
)
from drapto.exceptions import SegmentEncodingError

class TestSegmentEncoding(unittest.TestCase):
    @patch('drapto.video.segment_encoding.run_cmd')
    def test_encode_segment_retry_logic(self, mock_run_cmd):
        """Test that segment encoding properly implements retry logic"""
        # Mock successful encode after first retry
        mock_run_cmd.side_effect = [
            SegmentEncodingError("First attempt failed"),
            MagicMock(stderr="VMAF score: 95.0")
        ]
        
        segment = Path("/tmp/test_segment.mkv")
        output = Path("/tmp/output.mkv")
        
        # Should succeed on retry
        stats, logs = encode_segment(segment, output, None, 0, False, False)
        self.assertEqual(mock_run_cmd.call_count, 2)
        self.assertIn("retry", logs[0].lower())

    def test_memory_weight_estimation(self):
        """Test memory weight calculation based on resolution"""
        weights = {'SDR': 1, '1080p': 2, '4k': 4}
        segment = MagicMock()
        
        # Test 4K weight
        with patch('drapto.video.segment_encoding.get_video_info') as mock_info:
            mock_info.return_value = {'width': 3840}
            weight = estimate_memory_weight(segment, weights)
            self.assertEqual(weight, 4)
            
        # Test 1080p weight
        with patch('drapto.video.segment_encoding.get_video_info') as mock_info:
            mock_info.return_value = {'width': 1920}
            weight = estimate_memory_weight(segment, weights)
            self.assertEqual(weight, 2)

if __name__ == '__main__':
    unittest.main()
