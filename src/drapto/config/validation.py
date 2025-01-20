"""Validation functions for drapto configuration."""
import os
from pathlib import Path
from typing import Optional

from ..utils.paths import ensure_directory, is_under_directory, normalize_path
from .types import ColorConfig, PathConfig, ProcessConfig


def validate_path_exists_or_creatable(path: Path, path_type: str = "Path") -> None:
    """Validate that a path exists or can be created."""
    try:
        ensure_directory(path)
    except ValueError as e:
        raise ValueError(f"{path_type} validation failed: {e}")


def validate_path_config(config: PathConfig) -> None:
    """Validate path configuration."""
    # Convert string paths to Path objects if needed
    script_dir = normalize_path(config.script_dir)
    temp_dir = normalize_path(config.temp_dir)

    # Validate script directory
    if not script_dir.is_absolute():
        raise ValueError(f"Script directory '{script_dir}' must be an absolute path")
    if not script_dir.exists():
        raise ValueError(f"Script directory '{script_dir}' does not exist")
    if not script_dir.is_dir():
        raise ValueError(f"Script directory '{script_dir}' is not a directory")
    if not os.access(script_dir, os.R_OK):
        raise ValueError(f"Script directory '{script_dir}' is not readable")

    # Validate temp directory and its subdirectories
    validate_path_exists_or_creatable(temp_dir, "Temp directory")
    validate_path_exists_or_creatable(config.log_dir, "Log directory")
    validate_path_exists_or_creatable(config.temp_data_dir, "Temp data directory")
    validate_path_exists_or_creatable(config.segments_dir, "Segments directory")
    validate_path_exists_or_creatable(config.encoded_segments_dir, "Encoded segments directory")
    validate_path_exists_or_creatable(config.working_dir, "Working directory")

    # Validate temp directory relationships
    temp_path = temp_dir.resolve()
    for path, name in [
        (config.log_dir, "Log directory"),
        (config.temp_data_dir, "Temp data directory"),
        (config.segments_dir, "Segments directory"),
        (config.encoded_segments_dir, "Encoded segments directory"),
        (config.working_dir, "Working directory")
    ]:
        if not is_under_directory(path, temp_path):
            raise ValueError(f"{name} '{path}' must be under temp directory '{temp_path}'")

    # Validate input extensions
    if not config.input_extensions:
        raise ValueError("At least one input extension must be specified")
    for ext in config.input_extensions:
        if not isinstance(ext, str) or not ext:
            raise ValueError(f"Invalid input extension: {ext}")
        if not ext.isalnum():
            raise ValueError(f"Input extension '{ext}' must be alphanumeric")


def validate_color_config(config: ColorConfig) -> None:
    """Validate color configuration."""
    if not isinstance(config.force_color, bool):
        raise ValueError("force_color must be a boolean")
    if not isinstance(config.cli_color, bool):
        raise ValueError("cli_color must be a boolean")
    if not isinstance(config.cli_color_force, bool):
        raise ValueError("cli_color_force must be a boolean")
    if not isinstance(config.no_color, bool):
        raise ValueError("no_color must be a boolean")
    if config.color_term and not isinstance(config.color_term, str):
        raise ValueError(f"Invalid color term: {config.color_term}")
    if config.term is not None and not isinstance(config.term, str):
        raise ValueError(f"Invalid term: {config.term}")


def validate_process_config(config: ProcessConfig) -> None:
    """Validate process configuration."""
    if config.buffer_size < 0:
        raise ValueError(f"Buffer size must be non-negative: {config.buffer_size}")
    if config.process_timeout <= 0:
        raise ValueError(f"Process timeout must be positive: {config.process_timeout}")
    if config.thread_timeout <= 0:
        raise ValueError(f"Thread timeout must be positive: {config.thread_timeout}") 