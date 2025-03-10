"""Unit tests for segment encoding functionality"""

import unittest
from unittest.mock import patch, MagicMock, call
from pathlib import Path
from drapto.video.segment_encoding import (
    encode_segment,
    estimate_memory_weight,
    validate_encoded_segments,
    _build_encode_command,
    _parse_vmaf_scores,
    calculate_memory_requirements
)
from drapto.exceptions import SegmentEncodingError, ValidationError

class TestSegmentEncoding(unittest.TestCase):
    def setUp(self):
        """Set up test fixtures"""
        self.test_segment = Path("/tmp/test_segment.mkv")
        self.test_output = Path("/tmp/output.mkv")
        self.mock_probe = MagicMock()
        self.mock_probe.get.return_value = "1920"

    def test_build_encode_command(self):
        """Test command building with various parameters"""
        # Test basic command
        cmd = _build_encode_command(self.test_segment, self.test_output, None, 0, False, False)
        self.assertIn("--min-vmaf", cmd)
        self.assertIn("--preset", cmd)
        
        # Test with crop filter
        crop_filter = "crop=1920:800:0:140"
        cmd = _build_encode_command(self.test_segment, self.test_output, crop_filter, 0, False, False)
        self.assertIn("--vfilter", cmd)
        self.assertIn(crop_filter, cmd)
        
        # Test HDR parameters
        cmd = _build_encode_command(self.test_segment, self.test_output, None, 0, True, False)
        self.assertIn("95", cmd)  # HDR VMAF target
        
        # Test retry parameters
        cmd = _build_encode_command(self.test_segment, self.test_output, None, 2, False, False)
        self.assertIn("--samples", cmd)
        self.assertIn("4", cmd)  # Increased samples on final retry

    def test_parse_vmaf_scores(self):
        """Test VMAF score parsing from encoder output"""
        # Test normal output
        output = "frame=100 VMAF 95.5\nframe=200 VMAF 93.2\nframe=300 VMAF 94.8\n"
        avg, min_score, max_score = _parse_vmaf_scores(output)
        self.assertAlmostEqual(avg, 94.5)
        self.assertAlmostEqual(min_score, 93.2)
        self.assertAlmostEqual(max_score, 95.5)
        
        # Test empty output
        avg, min_score, max_score = _parse_vmaf_scores("")
        self.assertIsNone(avg)
        self.assertIsNone(min_score)
        self.assertIsNone(max_score)
        
        # Test malformed output
        output = "frame=100 Invalid VMAF\nframe=200\n"
        avg, min_score, max_score = _parse_vmaf_scores(output)
        self.assertIsNone(avg)
        self.assertIsNone(min_score)
        self.assertIsNone(max_score)

    @patch('drapto.video.segment_encoding.run_cmd')
    def test_encode_segment_retry_logic(self, mock_run_cmd):
        """Test segment encoding retry logic and error handling"""
        # Mock successful encode after first retry
        mock_run_cmd.side_effect = [
            SegmentEncodingError("First attempt failed"),
            MagicMock(stderr="VMAF score: 95.0")
        ]
        
        with patch('drapto.video.segment_encoding.probe_session') as mock_session:
            mock_session.return_value.__enter__.return_value = self.mock_probe
            mock_session.return_value.__exit__.return_value = None
            
            # Should succeed on retry
            stats, logs = encode_segment(self.test_segment, self.test_output, None, 0, False, False)
            self.assertEqual(mock_run_cmd.call_count, 2)
            self.assertIn("retry", logs[0].lower())
            
            # Test max retries
            mock_run_cmd.reset_mock()
            mock_run_cmd.side_effect = [SegmentEncodingError("Failed")] * 3
            
            with self.assertRaises(SegmentEncodingError):
                encode_segment(self.test_segment, self.test_output, None, 2, False, False)

    def test_memory_weight_estimation(self):
        """Test memory weight calculation based on resolution"""
        weights = {'SDR': 1, '1080p': 2, '4k': 4}
        
        with patch('drapto.video.segment_encoding.probe_session') as mock_session:
            # Test 4K weight
            self.mock_probe.get.return_value = "3840"
            mock_session.return_value.__enter__.return_value = self.mock_probe
            weight = estimate_memory_weight(self.test_segment, weights)
            self.assertEqual(weight, 4)
            
            # Test 1080p weight
            self.mock_probe.get.return_value = "1920"
            weight = estimate_memory_weight(self.test_segment, weights)
            self.assertEqual(weight, 2)
            
            # Test SD weight
            self.mock_probe.get.return_value = "1280"
            weight = estimate_memory_weight(self.test_segment, weights)
            self.assertEqual(weight, 1)
            
            # Test error handling
            self.mock_probe.get.side_effect = Exception("Probe failed")
            weight = estimate_memory_weight(self.test_segment, weights)
            self.assertEqual(weight, 1)  # Should return minimum weight on error

    def test_calculate_memory_requirements(self):
        """Test memory requirement calculation from warmup results"""
        # Mock warmup results
        warmup_results = [
            ({'resolution_category': '4k', 'peak_memory_bytes': 2048000000}, []),
            ({'resolution_category': '1080p', 'peak_memory_bytes': 1024000000}, []),
            ({'resolution_category': 'SDR', 'peak_memory_bytes': 512000000}, [])
        ]
        
        base_size, weights = calculate_memory_requirements(warmup_results)
        self.assertTrue(base_size > 0)
        self.assertEqual(weights['SDR'], 1)
        self.assertTrue(weights['4k'] > weights['1080p'])

    @patch('drapto.video.segment_encoding.probe_session')
    def test_validate_encoded_segments(self, mock_session):
        """Test encoded segment validation"""
        mock_session.return_value.__enter__.return_value = self.mock_probe
        segments_dir = Path("/tmp/segments")
        
        with patch('pathlib.Path.glob') as mock_glob:
            # Mock segment files
            mock_glob.return_value = [
                Path("/tmp/segments/seg1.mkv"),
                Path("/tmp/segments/seg2.mkv")
            ]
            
            # Test successful validation
            self.mock_probe.get.side_effect = ["av1", 1920, 1080, 10.0]
            self.assertTrue(validate_encoded_segments(segments_dir))
            
            # Test codec validation failure
            self.mock_probe.get.side_effect = ["h264", 1920, 1080, 10.0]
            with self.assertRaises(ValidationError):
                validate_encoded_segments(segments_dir)

if __name__ == '__main__':
    unittest.main()
