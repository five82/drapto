"""Main settings class for drapto configuration."""
import os
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

from .types import ColorConfig, PathConfig, ProcessConfig
from .validation import validate_color_config, validate_path_config, validate_process_config
from .defaults import get_default_color_config, get_default_path_config, get_default_process_config


@dataclass
class Settings:
    """Main configuration class for drapto."""
    color: ColorConfig
    paths: PathConfig
    process: ProcessConfig

    @classmethod
    def from_environment(cls, script_dir: Optional[Path] = None, temp_dir: Optional[Path] = None) -> "Settings":
        """Create settings from environment variables and optional overrides."""
        # Get default configurations
        color_config = get_default_color_config()
        path_config = get_default_path_config()
        process_config = get_default_process_config()

        # Override paths if provided
        if script_dir is not None:
            path_config.script_dir = script_dir
        if temp_dir is not None:
            # Create a new PathConfig to ensure computed paths are updated
            path_config = PathConfig(
                script_dir=path_config.script_dir,
                temp_dir=temp_dir,
                input_extensions=path_config.input_extensions
            )

        # Create settings instance
        settings = cls(
            color=color_config,
            paths=path_config,
            process=process_config
        )

        # Validate all configurations
        settings.validate()
        return settings

    def validate(self) -> None:
        """Validate all configuration settings."""
        validate_color_config(self.color)
        validate_path_config(self.paths)
        validate_process_config(self.process)

    def get_environment(self) -> dict[str, str]:
        """Get environment variables for subprocess execution."""
        env = os.environ.copy()

        # Remove any existing PYTHONPATH to avoid conflicts
        env.pop("PYTHONPATH", None)

        # Add color configuration
        env.update({
            "FORCE_COLOR": str(int(self.color.force_color)),
            "CLICOLOR": str(int(self.color.cli_color)),
            "CLICOLOR_FORCE": str(int(self.color.cli_color_force)),
            "NO_COLOR": str(int(self.color.no_color)),
            "COLORTERM": self.color.color_term,
        })
        if self.color.term:
            env["TERM"] = self.color.term

        return env

    def get_encode_environment(self, input_path: Path, output_path: Path) -> dict[str, str]:
        """Get environment variables for encode process."""
        env = self.get_environment()
        env.update({
            # Set all required paths in environment
            "SCRIPT_DIR": str(self.paths.script_dir.resolve()),
            "INPUT_DIR": str(input_path.parent.resolve()),
            "OUTPUT_DIR": str(output_path.parent.resolve()),
            "LOG_DIR": str(self.paths.log_dir.resolve()),
            "TEMP_DATA_DIR": str(self.paths.temp_data_dir.resolve()),
            "INPUT_FILE": str(input_path.resolve()),
            "OUTPUT_FILE": str(output_path.resolve()),
        })
        return env 