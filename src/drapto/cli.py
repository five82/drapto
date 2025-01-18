"""Command line interface for drapto."""
import os
import sys
import click
from pathlib import Path
from .core import Encoder

@click.command()
@click.argument('input_path', type=click.Path(exists=True))
@click.argument('output_path', type=click.Path())
def main(input_path: str, output_path: str) -> None:
    """Encode video files using drapto.
    
    INPUT_PATH can be a video file or directory.
    OUTPUT_PATH can be a file or directory (required if INPUT_PATH is a directory).
    """
    # Force unbuffered output
    os.environ["PYTHONUNBUFFERED"] = "1"
    
    try:
        encoder = Encoder()
        encoder.encode(input_path, output_path)
    except Exception as e:
        # Print full error message
        click.echo(f"Error: {str(e)}", err=True)
        sys.exit(1)

if __name__ == '__main__':
    main()
