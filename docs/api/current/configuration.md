# Configuration API

## Overview
The configuration system provides a type-safe, centralized way to manage all configuration settings. It handles environment variables, paths, process settings, and color output configuration.

## Classes

### Settings
The main configuration class that brings together all configuration components.

#### Methods
- `from_environment(script_dir: Optional[Path] = None, temp_dir: Optional[Path] = None) -> Settings`
  Creates settings from environment variables and optional overrides.
  
- `validate() -> None`
  Validates all configuration settings.
  
- `get_environment() -> dict[str, str]`
  Gets environment variables for subprocess execution.
  
- `get_encode_environment(input_path: Path, output_path: Path) -> dict[str, str]`
  Gets environment variables for encode process.

### ColorConfig
Controls terminal color output settings.

#### Attributes
- `force_color: bool` - Force color output (default: True)
- `cli_color: bool` - Enable CLI colors (default: True)
- `cli_color_force: bool` - Force CLI colors (default: True)
- `no_color: bool` - Disable all colors (default: False)
- `color_term: str` - Terminal color support (default: "truecolor")
- `term: Optional[str]` - Terminal type (default: auto-detected)

### PathConfig
Manages file and directory paths.

#### Attributes
- `script_dir: Path` - Location of encoding scripts
- `temp_dir: Path` - Base temporary directory
- `log_dir: Optional[Path]` - Log file directory (default: `temp_dir/logs`)
- `temp_data_dir: Optional[Path]` - Temporary data directory (default: `temp_dir/encode_data`)
- `input_extensions: tuple[str, ...]` - Supported input file extensions (default: `("mkv",)`)

### ProcessConfig
Controls process management settings.

#### Attributes
- `buffer_size: int` - Process output buffer size (default: 1)
- `process_timeout: float` - Process cleanup timeout in seconds (default: 2.0)
- `thread_timeout: float` - Thread cleanup timeout in seconds (default: 1.0)

## Environment Variables
The configuration system manages these environment variables:
- `SCRIPT_DIR`: Script directory path
- `FORCE_COLOR`: Force color output
- `CLICOLOR`: Enable CLI colors
- `CLICOLOR_FORCE`: Force CLI colors
- `NO_COLOR`: Disable all colors
- `COLORTERM`: Terminal color support
- `TERM`: Terminal type

## Examples

### Basic Usage
```python
from drapto.config import Settings

# Use default settings
settings = Settings.from_environment()

# Override specific paths
settings = Settings.from_environment(
    script_dir="/custom/script/dir",
    temp_dir="/custom/temp/dir"
)
```

### Custom Process Timeouts
```python
from drapto.config import Settings
from drapto.config.types import ProcessConfig

settings = Settings.from_environment()
settings.process = ProcessConfig(
    process_timeout=5.0,  # Longer process timeout
    thread_timeout=2.0    # Longer thread timeout
)
```

### Custom Input Extensions
```python
from drapto.config import Settings
from drapto.config.types import PathConfig

settings = Settings.from_environment()
settings.paths.input_extensions = ("mp4", "mov", "mkv")
```

### Disable Colors
```python
from drapto.config import Settings
from drapto.config.types import ColorConfig

settings = Settings.from_environment()
settings.color = ColorConfig(
    force_color=False,
    cli_color=False,
    cli_color_force=False,
    no_color=True
)
``` 