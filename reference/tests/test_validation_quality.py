"""Unit tests for video quality validation functionality

This test suite verifies the behavior of quality metric validation,
including VMAF score checking and quality threshold enforcement.
"""

import unittest
from pathlib import Path
from drapto.validation.validation_quality import validate_quality_metrics

class TestValidationQuality(unittest.TestCase):
    """Test cases for quality metric validation"""
    def test_validate_quality_metrics(self):
        input_file = Path("/tmp/input.mkv")
        output_file = Path("/tmp/output.mkv")
        report = []
        validate_quality_metrics(input_file, output_file, report)
        self.assertIn("Quality target", report[0])

if __name__ == "__main__":
    unittest.main()
