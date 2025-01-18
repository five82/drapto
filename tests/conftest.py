"""Common test fixtures and utilities."""
import os
import pytest
from pathlib import Path
from drapto.core import Encoder

@pytest.fixture
def temp_dir(tmp_path):
    """Create a temporary directory for tests."""
    return tmp_path

@pytest.fixture
def mock_video_file(temp_dir):
    """Create a mock video file for testing."""
    video_file = temp_dir / "test.mp4"
    video_file.write_text("mock video content")
    return video_file

@pytest.fixture
def encoder():
    """Create an Encoder instance for testing."""
    return Encoder()
