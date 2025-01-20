"""Tests for path monitoring."""
import os
import time
from pathlib import Path
import pytest

from drapto.monitoring.paths import PathUsage, PathMonitor


@pytest.fixture
def monitor():
    """Create a fresh path monitor for testing."""
    return PathMonitor()


@pytest.fixture
def temp_path(tmp_path):
    """Create a temporary file for testing."""
    path = tmp_path / "test_file.txt"
    path.write_text("test content")
    return path


def test_path_usage_update_access(temp_path):
    """Test PathUsage access tracking."""
    usage = PathUsage(temp_path)
    initial_time = time.time()
    
    usage.update_access()
    assert usage.access_count == 1
    assert usage.last_access >= initial_time
    assert usage.total_size == len("test content")
    assert not usage.is_error
    assert usage.error_message is None


def test_path_usage_update_access_directory(tmp_path):
    """Test PathUsage access tracking for directory."""
    # Create some files in the directory
    (tmp_path / "file1.txt").write_text("content1")
    (tmp_path / "file2.txt").write_text("content2")
    
    usage = PathUsage(tmp_path)
    usage.update_access()
    
    assert usage.access_count == 1
    assert usage.total_size == len("content1") + len("content2")
    assert not usage.is_error


def test_path_usage_mark_error(temp_path):
    """Test PathUsage error marking."""
    usage = PathUsage(temp_path)
    initial_time = time.time()
    
    usage.mark_error("test error")
    assert usage.is_error
    assert usage.error_message == "test error"
    assert usage.last_access >= initial_time


def test_monitor_track_path(monitor, temp_path):
    """Test PathMonitor path tracking."""
    monitor.track_path(temp_path)
    usage = monitor.get_usage(temp_path)
    
    assert usage is not None
    assert usage.path == temp_path.resolve()
    assert usage.access_count == 0


def test_monitor_record_access(monitor, temp_path):
    """Test PathMonitor access recording."""
    monitor.record_access(temp_path)
    usage = monitor.get_usage(temp_path)
    
    assert usage is not None
    assert usage.access_count == 1
    assert usage.total_size == len("test content")


def test_monitor_record_error(monitor, temp_path):
    """Test PathMonitor error recording."""
    monitor.record_error(temp_path, "test error")
    usage = monitor.get_usage(temp_path)
    
    assert usage is not None
    assert usage.is_error
    assert usage.error_message == "test error"


def test_monitor_get_all_usage(monitor, temp_path):
    """Test PathMonitor getting all usage stats."""
    monitor.record_access(temp_path)
    monitor.record_error(temp_path, "test error")
    
    all_usage = monitor.get_all_usage()
    assert len(all_usage) == 1
    assert all_usage[0].path == temp_path.resolve()
    assert all_usage[0].access_count == 1
    assert all_usage[0].is_error


def test_monitor_get_errors(monitor, temp_path):
    """Test PathMonitor getting error paths."""
    # Create two paths, mark one with error
    other_path = temp_path.parent / "other.txt"
    other_path.touch()
    
    monitor.record_access(temp_path)
    monitor.record_access(other_path)
    monitor.record_error(temp_path, "test error")
    
    errors = monitor.get_errors()
    assert len(errors) == 1
    assert errors[0].path == temp_path.resolve()
    assert errors[0].error_message == "test error"


def test_monitor_clear(monitor, temp_path):
    """Test PathMonitor clearing all tracked paths."""
    monitor.record_access(temp_path)
    assert len(monitor.get_all_usage()) == 1
    
    monitor.clear()
    assert len(monitor.get_all_usage()) == 0
    assert monitor.get_usage(temp_path) is None 