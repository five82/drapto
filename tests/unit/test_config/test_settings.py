"""Unit tests for drapto configuration."""
import os
import tempfile
from pathlib import Path
import pytest

from drapto.config import Settings
from drapto.config.types import ColorConfig, PathConfig, ProcessConfig


def test_settings_from_environment():
    """Test creating settings from environment."""
    settings = Settings.from_environment()
    assert isinstance(settings.color, ColorConfig)
    assert isinstance(settings.paths, PathConfig)
    assert isinstance(settings.process, ProcessConfig)


def test_settings_with_overrides():
    """Test creating settings with path overrides."""
    script_dir = Path(__file__).parent
    temp_dir = Path(tempfile.gettempdir()) / "test_drapto"
    
    settings = Settings.from_environment(script_dir=script_dir, temp_dir=temp_dir)
    assert settings.paths.script_dir == script_dir
    assert settings.paths.temp_dir == temp_dir
    assert settings.paths.log_dir == temp_dir / "logs"
    assert settings.paths.temp_data_dir == temp_dir / "encode_data"


def test_settings_validation():
    """Test settings validation."""
    settings = Settings.from_environment()
    
    # Test color config validation
    with pytest.raises(ValueError):
        settings.color.force_color = "invalid"  # type: ignore
        settings.validate()
    
    # Reset and test path config validation
    settings = Settings.from_environment()
    with pytest.raises(ValueError):
        settings.paths.script_dir = "invalid"  # type: ignore
        settings.validate()
    
    # Reset and test process config validation
    settings = Settings.from_environment()
    with pytest.raises(ValueError):
        settings.process.buffer_size = -1
        settings.validate()


def test_get_environment():
    """Test getting environment variables."""
    settings = Settings.from_environment()
    env = settings.get_environment()
    
    assert "PYTHONPATH" not in env
    assert env["FORCE_COLOR"] == "1"
    assert env["CLICOLOR"] == "1"
    assert env["CLICOLOR_FORCE"] == "1"
    assert env["NO_COLOR"] == "0"
    assert env["COLORTERM"] == "truecolor"
    assert "TERM" in env


def test_get_encode_environment():
    """Test getting encode environment variables."""
    settings = Settings.from_environment()
    input_path = Path("/test/input.mkv")
    output_path = Path("/test/output.mkv")
    
    env = settings.get_encode_environment(input_path, output_path)
    
    assert env["SCRIPT_DIR"] == str(settings.paths.script_dir.resolve())
    assert env["INPUT_DIR"] == str(input_path.parent.resolve())
    assert env["OUTPUT_DIR"] == str(output_path.parent.resolve())
    assert env["LOG_DIR"] == str(settings.paths.log_dir.resolve())
    assert env["TEMP_DATA_DIR"] == str(settings.paths.temp_data_dir.resolve())
    assert env["INPUT_FILE"] == str(input_path.resolve())
    assert env["OUTPUT_FILE"] == str(output_path.resolve()) 