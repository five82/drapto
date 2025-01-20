"""Type definitions for drapto configuration."""
from dataclasses import dataclass
from pathlib import Path
from typing import Optional


@dataclass
class ColorConfig:
    """Configuration for terminal color support."""
    force_color: bool = True
    cli_color: bool = True
    cli_color_force: bool = True
    no_color: bool = False
    color_term: str = "truecolor"
    term: Optional[str] = None


@dataclass
class PathConfig:
    """Configuration for file and directory paths."""
    script_dir: Path
    temp_dir: Path
    log_dir: Optional[Path] = None
    temp_data_dir: Optional[Path] = None
    segments_dir: Optional[Path] = None
    encoded_segments_dir: Optional[Path] = None
    working_dir: Optional[Path] = None
    input_extensions: tuple[str, ...] = ("mkv",)

    def __post_init__(self) -> None:
        """Convert string paths to Path objects and create computed paths."""
        # Convert string paths to Path objects
        if isinstance(self.script_dir, str):
            self.script_dir = Path(self.script_dir)
        if isinstance(self.temp_dir, str):
            self.temp_dir = Path(self.temp_dir)
            
        # Set computed paths if not provided
        if self.log_dir is None:
            self.log_dir = self.temp_dir / "logs"
        elif isinstance(self.log_dir, str):
            self.log_dir = Path(self.log_dir)
            
        if self.temp_data_dir is None:
            self.temp_data_dir = self.temp_dir / "encode_data"
        elif isinstance(self.temp_data_dir, str):
            self.temp_data_dir = Path(self.temp_data_dir)

        # Set additional computed paths
        if self.segments_dir is None:
            self.segments_dir = self.temp_dir / "segments"
        elif isinstance(self.segments_dir, str):
            self.segments_dir = Path(self.segments_dir)

        if self.encoded_segments_dir is None:
            self.encoded_segments_dir = self.temp_dir / "encoded_segments"
        elif isinstance(self.encoded_segments_dir, str):
            self.encoded_segments_dir = Path(self.encoded_segments_dir)

        if self.working_dir is None:
            self.working_dir = self.temp_dir / "working"
        elif isinstance(self.working_dir, str):
            self.working_dir = Path(self.working_dir)


@dataclass
class ProcessConfig:
    """Configuration for process management."""
    buffer_size: int = 1  # Line buffered
    process_timeout: float = 2.0  # Seconds
    thread_timeout: float = 1.0  # Seconds 