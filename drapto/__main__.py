"""
Command-line interface for drapto video encoding pipeline
"""
import argparse
import logging
import sys
from pathlib import Path

from rich.logging import RichHandler

from . import __version__

def setup_logging():
    """Configure logging with rich output"""
    logging.basicConfig(
        level=logging.INFO,
        format="%(message)s",
        datefmt="[%X]",
        handlers=[RichHandler(rich_tracebacks=True)]
    )

def parse_args():
    """Parse command line arguments"""
    parser = argparse.ArgumentParser(
        description="Video encoding pipeline using AV1"
    )
    parser.add_argument(
        "--version",
        action="version",
        version=f"%(prog)s {__version__}"
    )
    parser.add_argument(
        "-i", "--input-dir",
        type=Path,
        help="Input directory containing video files"
    )
    parser.add_argument(
        "-o", "--output-dir", 
        type=Path,
        help="Output directory for encoded files"
    )
    return parser.parse_args()

def main():
    """Main entry point"""
    setup_logging()
    args = parse_args()
    
    log = logging.getLogger("drapto")
    log.info("Starting drapto video encoder v%s", __version__)
    
    # TODO: Implement pipeline stages
    
    return 0

if __name__ == "__main__":
    sys.exit(main())
