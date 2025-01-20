"""Tests for path configuration."""
import os
import pytest
from pathlib import Path
from typing import Generator

from drapto.config.types import PathConfig
from drapto.config.validation import validate_path_config


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
def path_config(script_dir: Path, temp_dir: Path) -> PathConfig:
    """Create a basic path configuration."""
    return PathConfig(script_dir=script_dir, temp_dir=temp_dir)


def test_path_config_basic_initialization(path_config: PathConfig) -> None:
    """Test basic path configuration initialization."""
    assert path_config.script_dir.exists()
    assert path_config.temp_dir.exists()
    assert path_config.log_dir == path_config.temp_dir / "logs"
    assert path_config.temp_data_dir == path_config.temp_dir / "encode_data"
    assert path_config.segments_dir == path_config.temp_dir / "segments"
    assert path_config.encoded_segments_dir == path_config.temp_dir / "encoded_segments"
    assert path_config.working_dir == path_config.temp_dir / "working"


def test_path_config_string_paths(script_dir: Path, temp_dir: Path) -> None:
    """Test path configuration with string paths."""
    config = PathConfig(
        script_dir=str(script_dir),
        temp_dir=str(temp_dir)
    )
    assert isinstance(config.script_dir, Path)
    assert isinstance(config.temp_dir, Path)
    assert config.script_dir == script_dir
    assert config.temp_dir == temp_dir


def test_path_config_custom_paths(script_dir: Path, temp_dir: Path) -> None:
    """Test path configuration with custom paths."""
    custom_log = temp_dir / "custom_logs"
    custom_data = temp_dir / "custom_data"
    custom_segments = temp_dir / "custom_segments"
    custom_encoded = temp_dir / "custom_encoded"
    custom_working = temp_dir / "custom_working"

    config = PathConfig(
        script_dir=script_dir,
        temp_dir=temp_dir,
        log_dir=custom_log,
        temp_data_dir=custom_data,
        segments_dir=custom_segments,
        encoded_segments_dir=custom_encoded,
        working_dir=custom_working
    )

    assert config.log_dir == custom_log
    assert config.temp_data_dir == custom_data
    assert config.segments_dir == custom_segments
    assert config.encoded_segments_dir == custom_encoded
    assert config.working_dir == custom_working


def test_path_config_validation_script_dir_not_exists(temp_root: Path, temp_dir: Path) -> None:
    """Test validation fails when script directory doesn't exist."""
    non_existent = temp_root / "non_existent"
    with pytest.raises(ValueError, match="Script directory.*does not exist"):
        validate_path_config(PathConfig(script_dir=non_existent, temp_dir=temp_dir))


def test_path_config_validation_script_dir_not_readable(script_dir: Path, temp_dir: Path) -> None:
    """Test validation fails when script directory is not readable."""
    os.chmod(script_dir, 0o000)  # Remove all permissions
    try:
        with pytest.raises(ValueError, match="Script directory.*not readable"):
            validate_path_config(PathConfig(script_dir=script_dir, temp_dir=temp_dir))
    finally:
        os.chmod(script_dir, 0o755)  # Restore permissions


def test_path_config_validation_temp_dir_creation(script_dir: Path, temp_root: Path) -> None:
    """Test validation creates temp directory if it doesn't exist."""
    new_temp = temp_root / "new_temp"
    config = PathConfig(script_dir=script_dir, temp_dir=new_temp)
    validate_path_config(config)
    assert new_temp.exists()
    assert new_temp.is_dir()


def test_path_config_validation_temp_dir_not_writable(script_dir: Path, temp_dir: Path) -> None:
    """Test validation fails when temp directory is not writable."""
    os.chmod(temp_dir, 0o500)  # Read + execute only
    try:
        with pytest.raises(ValueError, match="Temp directory.*not writable"):
            validate_path_config(PathConfig(script_dir=script_dir, temp_dir=temp_dir))
    finally:
        os.chmod(temp_dir, 0o755)  # Restore permissions


def test_path_config_validation_subdirs_under_temp(script_dir: Path, temp_dir: Path, temp_root: Path) -> None:
    """Test validation fails when subdirectories are not under temp directory."""
    outside_dir = temp_root / "outside"
    outside_dir.mkdir()
    
    with pytest.raises(ValueError, match="Log directory.*must be under temp directory"):
        validate_path_config(PathConfig(
            script_dir=script_dir,
            temp_dir=temp_dir,
            log_dir=outside_dir
        ))


def test_path_config_validation_input_extensions(script_dir: Path, temp_dir: Path) -> None:
    """Test validation of input extensions."""
    # Test empty extensions
    with pytest.raises(ValueError, match="At least one input extension must be specified"):
        validate_path_config(PathConfig(
            script_dir=script_dir,
            temp_dir=temp_dir,
            input_extensions=()
        ))

    # Test invalid extension
    with pytest.raises(ValueError, match="Input extension.*must be alphanumeric"):
        validate_path_config(PathConfig(
            script_dir=script_dir,
            temp_dir=temp_dir,
            input_extensions=("mkv", ".mp4")
        ))


def test_path_config_validation_success(path_config: PathConfig) -> None:
    """Test successful validation of a valid configuration."""
    validate_path_config(path_config)  # Should not raise any exceptions 