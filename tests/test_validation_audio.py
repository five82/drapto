import unittest
from pathlib import Path
from drapto.validation.validation_audio import validate_input_audio

class TestValidationAudio(unittest.TestCase):
    def test_validate_input_audio_failure(self):
        # Test with a file that you know will not pass validation.
        fake_file = Path("/tmp/nonexistent.mkv")
        with self.assertRaises(Exception):
            validate_input_audio(fake_file)

if __name__ == "__main__":
    unittest.main()
