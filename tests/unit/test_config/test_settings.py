"""Unit tests for drapto configuration."""
import os
import tempfile
from pathlib import Path
import pytest

from drapto.config import Settings
from drapto.config.types import ColorConfig, PathConfig, ProcessConfig


@pytest.fixture
def temp_root(tmp_path: Path) -> Path:
    """Create a temporary root directory for testing."""
    return tmp_path


@pytest.fixture
def script_dir(temp_root: Path) -> Path:
    """Create a temporary script directory."""
    script_dir = temp_root / "scripts"
    script_dir.mkdir()
    return script_dir


@pytest.fixture
def temp_dir(temp_root: Path) -> Path:
    """Create a temporary directory for testing."""
    temp_dir = temp_root / "temp"
    temp_dir.mkdir()
    return temp_dir


@pytest.fixture
def clean_term_settings(script_dir: Path, temp_dir: Path) -> Settings:
    """Create a settings instance with clean TERM environment."""
    # Save original TERM value
    original_term = os.environ.get("TERM")
    if "TERM" in os.environ:
        del os.environ["TERM"]

    try:
        return Settings.from_environment(script_dir=script_dir, temp_dir=temp_dir)
    finally:
        # Restore original TERM value
        if original_term is not None:
            os.environ["TERM"] = original_term


@pytest.fixture
def settings(script_dir: Path, temp_dir: Path) -> Settings:
    """Create a settings instance for testing."""
    return Settings.from_environment(script_dir=script_dir, temp_dir=temp_dir)


def test_settings_from_environment_basic(settings: Settings) -> None:
    """Test basic settings creation from environment."""
    assert isinstance(settings.color, ColorConfig)
    assert isinstance(settings.paths, PathConfig)
    assert isinstance(settings.process, ProcessConfig)


def test_settings_get_environment_color(clean_term_settings: Settings) -> None:
    """Test color environment variables."""
    env = clean_term_settings.get_environment()
    assert env["FORCE_COLOR"] == "1"
    assert env["CLICOLOR"] == "1"
    assert env["CLICOLOR_FORCE"] == "1"
    assert env["NO_COLOR"] == "0"
    assert env["COLORTERM"] == "truecolor"
    assert "TERM" not in env  # Default term is None


def test_settings_get_environment_pythonpath_removed(settings: Settings) -> None:
    """Test PYTHONPATH is removed from environment."""
    os.environ["PYTHONPATH"] = "/some/path"
    env = settings.get_environment()
    assert "PYTHONPATH" not in env


def test_settings_get_encode_environment_paths(settings: Settings, temp_root: Path) -> None:
    """Test path environment variables for encode process."""
    input_path = temp_root / "input.mkv"
    output_path = temp_root / "output.mkv"
    
    env = settings.get_encode_environment(input_path, output_path)
    
    # Check all path variables are set and resolved
    assert env["SCRIPT_DIR"] == str(settings.paths.script_dir.resolve())
    assert env["INPUT_DIR"] == str(input_path.parent.resolve())
    assert env["OUTPUT_DIR"] == str(output_path.parent.resolve())
    assert env["LOG_DIR"] == str(settings.paths.log_dir.resolve())
    assert env["TEMP_DIR"] == str(settings.paths.temp_dir.resolve())
    assert env["TEMP_DATA_DIR"] == str(settings.paths.temp_data_dir.resolve())
    assert env["SEGMENTS_DIR"] == str(settings.paths.segments_dir.resolve())
    assert env["ENCODED_SEGMENTS_DIR"] == str(settings.paths.encoded_segments_dir.resolve())
    assert env["WORKING_DIR"] == str(settings.paths.working_dir.resolve())
    assert env["INPUT_FILE"] == str(input_path.resolve())
    assert env["OUTPUT_FILE"] == str(output_path.resolve())


def test_settings_get_encode_environment_inherits_color(clean_term_settings: Settings, temp_root: Path) -> None:
    """Test encode environment inherits color settings."""
    input_path = temp_root / "input.mkv"
    output_path = temp_root / "output.mkv"
    
    env = clean_term_settings.get_encode_environment(input_path, output_path)
    
    assert env["FORCE_COLOR"] == "1"
    assert env["CLICOLOR"] == "1"
    assert env["CLICOLOR_FORCE"] == "1"
    assert env["NO_COLOR"] == "0"
    assert env["COLORTERM"] == "truecolor"
    assert "TERM" not in env  # Default term is None


def test_settings_validation_on_creation(script_dir: Path) -> None:
    """Test settings validation on creation."""
    non_existent = script_dir / "non_existent"
    with pytest.raises(ValueError, match="Script directory.*does not exist"):
        Settings.from_environment(script_dir=non_existent)


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