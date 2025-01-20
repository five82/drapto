"""Validation functions for drapto configuration."""
from pathlib import Path
from typing import Optional

from .types import ColorConfig, PathConfig, ProcessConfig


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
    if not isinstance(config.color_term, str):
        raise ValueError("color_term must be a string")
    if config.term is not None and not isinstance(config.term, str):
        raise ValueError("term must be None or a string")


def validate_path_config(config: PathConfig) -> None:
    """Validate path configuration."""
    if not isinstance(config.script_dir, Path):
        raise ValueError("script_dir must be a Path")
    if not config.script_dir.exists():
        raise ValueError(f"Script directory not found: {config.script_dir}")
        
    if not isinstance(config.temp_dir, Path):
        raise ValueError("temp_dir must be a Path")
        
    if config.log_dir is not None and not isinstance(config.log_dir, Path):
        raise ValueError("log_dir must be None or a Path")
        
    if config.temp_data_dir is not None and not isinstance(config.temp_data_dir, Path):
        raise ValueError("temp_data_dir must be None or a Path")
        
    if not isinstance(config.input_extensions, tuple):
        raise ValueError("input_extensions must be a tuple")
    if not all(isinstance(ext, str) for ext in config.input_extensions):
        raise ValueError("All input extensions must be strings")


def validate_process_config(config: ProcessConfig) -> None:
    """Validate process configuration."""
    if not isinstance(config.buffer_size, int) or config.buffer_size < 0:
        raise ValueError("buffer_size must be a non-negative integer")
    if not isinstance(config.process_timeout, (int, float)) or config.process_timeout <= 0:
        raise ValueError("process_timeout must be a positive number")
    if not isinstance(config.thread_timeout, (int, float)) or config.thread_timeout <= 0:
        raise ValueError("thread_timeout must be a positive number") 