"""Default configuration values for drapto."""
import os
import tempfile
from pathlib import Path

from .types import ColorConfig, PathConfig, ProcessConfig


def get_default_color_config() -> ColorConfig:
    """Get default color configuration."""
    current_term = os.environ.get("TERM")
    # Only set a default term if the current one exists but doesn't support color
    if current_term and not any(x in current_term for x in ["color", "xterm", "vt100"]):
        current_term = "xterm-256color"
        
    return ColorConfig(
        force_color=True,
        cli_color=True,
        cli_color_force=True,
        no_color=False,
        color_term="truecolor",
        term=current_term
    )


def get_default_path_config() -> PathConfig:
    """Get default path configuration."""
    # Use package scripts directory
    script_dir = Path(__file__).parent.parent / "scripts"
    
    # Use system temp directory for drapto
    temp_dir = Path(tempfile.gettempdir()) / "drapto"
    
    return PathConfig(
        script_dir=script_dir,
        temp_dir=temp_dir,
        input_extensions=("mkv",)  # Only MKV files are supported
    )


def get_default_process_config() -> ProcessConfig:
    """Get default process configuration."""
    return ProcessConfig(
        buffer_size=1,  # Line buffered
        process_timeout=2.0,  # Seconds
        thread_timeout=1.0  # Seconds
    ) 