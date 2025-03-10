"""Unit tests for FFProbe session management

This test suite verifies the behavior of the FFProbe session context manager
and caching functionality.
"""

import unittest
from unittest.mock import patch, MagicMock
from pathlib import Path
from drapto.ffprobe.session import FFProbeSession, probe_session
from drapto.ffprobe.exec import MetadataError

class TestFFProbeSession(unittest.TestCase):
    def setUp(self):
        self.test_file = Path("/tmp/test.mkv")
        self.mock_property = "width"
        self.mock_value = "1920"

    @patch("drapto.ffprobe.session.get_media_property")
    def test_session_get_caches_results(self, mock_get_property):
        """Test that session.get() caches results"""
        mock_get_property.return_value = self.mock_value
        
        session = FFProbeSession(self.test_file)
        
        # First call - should call get_media_property
        result1 = session.get(self.mock_property, "video")
        mock_get_property.assert_called_once_with(
            self.test_file, "video", self.mock_property, 0
        )
        self.assertEqual(result1, self.mock_value)
        
        # Second call - should use cached value
        result2 = session.get(self.mock_property, "video")
        mock_get_property.assert_called_once()  # Still only called once
        self.assertEqual(result2, self.mock_value)

    @patch("drapto.ffprobe.session.get_media_property")
    def test_session_get_different_streams(self, mock_get_property):
        """Test that different stream types/indexes are cached separately"""
        mock_get_property.side_effect = ["1920", "1080"]
        
        session = FFProbeSession(self.test_file)
        
        # Get video width
        video_width = session.get("width", "video")
        # Get audio channels
        audio_channels = session.get("channels", "audio", 1)
        
        self.assertEqual(video_width, "1920")
        self.assertEqual(audio_channels, "1080")
        self.assertEqual(mock_get_property.call_count, 2)

    @patch("drapto.ffprobe.session.get_media_property")
    def test_probe_session_context_manager(self, mock_get_property):
        """Test that probe_session context manager works correctly"""
        mock_get_property.return_value = self.mock_value
        
        with probe_session(self.test_file) as session:
            result = session.get(self.mock_property, "video")
            self.assertEqual(result, self.mock_value)
        
        # Verify cleanup
        self.assertFalse(hasattr(session, "_cache"))

    @patch("drapto.ffprobe.session.get_media_property")
    def test_probe_session_error_handling(self, mock_get_property):
        """Test that probe_session handles errors properly"""
        mock_get_property.side_effect = MetadataError("Test error")
        
        with self.assertRaises(MetadataError):
            with probe_session(self.test_file) as session:
                session.get(self.mock_property, "video")

if __name__ == "__main__":
    unittest.main()
