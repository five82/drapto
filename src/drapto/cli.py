"""Command line interface for drapto."""
import os
import sys
from pathlib import Path
import click

from .config.settings import Settings
from .config.types import PathConfig
from .core import Encoder
from .utils.paths import normalize_path


@click.command()
@click.argument('input_path', type=click.Path(exists=True))
@click.argument('output_path', type=click.Path())
@click.option('--script-dir', type=click.Path(exists=True), help='Directory containing encoding scripts')
@click.option('--temp-dir', type=click.Path(), help='Base directory for temporary files')
def main(input_path: str, output_path: str, script_dir: str | None = None, temp_dir: str | None = None) -> None:
    """Encode video files using drapto.
    
    INPUT_PATH can be a video file or directory.
    OUTPUT_PATH can be a file or directory (required if INPUT_PATH is a directory).
    """
    # Force unbuffered output
    os.environ["PYTHONUNBUFFERED"] = "1"
    
    try:
        # Initialize settings with default configuration
        settings = Settings.from_environment()
        
        # Override paths if provided
        if script_dir:
            settings.paths.script_dir = normalize_path(script_dir)
        if temp_dir:
            settings.paths.temp_dir = normalize_path(temp_dir)
        
        # Create encoder with settings
        encoder = Encoder(settings)
        encoder.encode(Path(input_path), Path(output_path))
    except Exception as e:
        # Print full error message
        click.echo(f"Error: {str(e)}", err=True)
        sys.exit(1)


if __name__ == '__main__':
    main()
