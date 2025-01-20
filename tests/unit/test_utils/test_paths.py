"""Tests for path utilities."""
import os
import tempfile
from pathlib import Path
import pytest

from drapto.utils.paths import (
    ensure_directory,
    is_under_directory,
    normalize_path,
    get_relative_path,
    get_temp_path
)


@pytest.fixture
def temp_dir():
    """Create a temporary directory for testing."""
    with tempfile.TemporaryDirectory() as tmp_dir:
        yield Path(tmp_dir)


def test_ensure_directory_creates_new(temp_dir):
    """Test ensure_directory creates a new directory."""
    new_dir = temp_dir / "new_dir"
    result = ensure_directory(new_dir)
    assert result.exists()
    assert result.is_dir()
    assert os.access(result, os.W_OK)


def test_ensure_directory_existing(temp_dir):
    """Test ensure_directory with existing directory."""
    result = ensure_directory(temp_dir)
    assert result == temp_dir.resolve()


def test_ensure_directory_not_writable(temp_dir):
    """Test ensure_directory with non-writable directory."""
    new_dir = temp_dir / "no_write"
    new_dir.mkdir()
    os.chmod(new_dir, 0o444)  # Read-only
    with pytest.raises(ValueError, match="not writable"):
        ensure_directory(new_dir)


def test_ensure_directory_file_exists(temp_dir):
    """Test ensure_directory when file exists at path."""
    file_path = temp_dir / "file"
    file_path.touch()
    with pytest.raises(ValueError, match="not a directory"):
        ensure_directory(file_path)


def test_is_under_directory(temp_dir):
    """Test is_under_directory function."""
    child = temp_dir / "child"
    child.mkdir()
    assert is_under_directory(child, temp_dir)
    assert not is_under_directory(temp_dir, child)
    assert not is_under_directory(temp_dir.parent, temp_dir)


def test_normalize_path():
    """Test normalize_path function."""
    path_str = "~/test/path"
    path_obj = Path(path_str)
    
    norm_str = normalize_path(path_str)
    norm_obj = normalize_path(path_obj)
    
    assert isinstance(norm_str, Path)
    assert isinstance(norm_obj, Path)
    assert norm_str.is_absolute()
    assert norm_obj.is_absolute()
    assert norm_str == norm_obj


def test_get_relative_path(temp_dir):
    """Test get_relative_path function."""
    child = temp_dir / "child" / "subdir"
    child.mkdir(parents=True)
    
    rel_path = get_relative_path(child, temp_dir)
    assert str(rel_path) == "child/subdir"
    
    with pytest.raises(ValueError, match="not under base directory"):
        get_relative_path(temp_dir.parent, temp_dir)


def test_get_temp_path(temp_dir):
    """Test get_temp_path function."""
    path1 = get_temp_path(temp_dir, prefix="test_", suffix=".tmp")
    path2 = get_temp_path(temp_dir, prefix="test_", suffix=".tmp")
    
    assert path1.parent == temp_dir
    assert path2.parent == temp_dir
    assert path1 != path2
    assert path1.name.startswith("test_")
    assert path1.name.endswith(".tmp")
    assert not path1.exists()
    assert not path2.exists()


def test_get_temp_path_invalid_base(temp_dir):
    """Test get_temp_path with invalid base directory."""
    invalid_dir = temp_dir / "nonexistent"
    with pytest.raises(ValueError, match="does not exist"):
        get_temp_path(invalid_dir, create=False) 