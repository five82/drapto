import unittest
from pathlib import Path
from unittest.mock import patch
from drapto.validation.validation_subtitles import validate_subtitle_tracks
from drapto.exceptions import ValidationError

def dummy_get_subtitle_info_success(file):
    return {"streams": [{"codec_name": "subrip"}, {"codec_name": "subrip"}]}

class TestValidationSubtitles(unittest.TestCase):
    @patch("drapto.validation.validation_subtitles.get_subtitle_info", side_effect=dummy_get_subtitle_info_success)
    def test_validate_subtitle_tracks_success(self, mock_subtitle_info):
        report = []
        # Should pass validation and report two subtitle tracks
        validate_subtitle_tracks(Path("/tmp/input.mkv"), Path("/tmp/output.mkv"), report)
        self.assertIn("Subtitles:", report[0])
    
    @patch("drapto.validation.validation_subtitles.get_subtitle_info", return_value={})
    def test_validate_subtitle_tracks_failure(self, mock_subtitle_info):
        report = []
        with self.assertRaises(Exception):
            validate_subtitle_tracks(Path("/tmp/input.mkv"), Path("/tmp/output.mkv"), report)

if __name__ == "__main__":
    unittest.main()
