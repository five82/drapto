"""
Command-line interface for drapto video encoding pipeline
"""
import argparse
import logging
import sys
from pathlib import Path

from rich.logging import RichHandler

from . import __version__
from .formatting import print_header, print_error, print_info, print_success
from .pipeline import process_directory, process_file
from .utils import check_dependencies

def setup_logging():
    """Configure logging with rich output"""
    logging.basicConfig(
        level=logging.DEBUG,
        format="%(message)s",
        datefmt="[%X]",
        handlers=[RichHandler(rich_tracebacks=True)]
    )
    # Set scene detection logger to DEBUG explicitly
    logging.getLogger("drapto.video.scene_detection").setLevel(logging.DEBUG)

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
        "input",
        type=Path,
        help="Input file or directory"
    )
    parser.add_argument(
        "output",
        type=Path,
        help="Output file (if input is file) or directory (if input is directory)"
    )
    return parser.parse_args()

def main():
    """Main entry point"""
    setup_logging()
    args = parse_args()
    
    log = logging.getLogger("drapto")
    print_header(f"Starting drapto video encoder v{__version__}")
    print_info("Processing input...")
    
    # Check dependencies
    if not check_dependencies():
        log.error("Missing required dependencies")
        return 1
        
    # Process input
    try:
        if args.input.is_file():
            # Determine if the output should be treated as a directory
            # Even if args.output doesn't exist, if it has no file extension we assume it's a directory.
            if args.output.exists():
                if args.output.is_dir():
                    out_file = args.output / args.input.name
                else:
                    out_file = args.output
            else:
                if args.output.suffix == "":
                    args.output.mkdir(parents=True, exist_ok=True)
                    out_file = args.output / args.input.name
                else:
                    out_file = args.output

            if process_file(args.input, out_file):
                print_success(f"Successfully encoded {args.input.name}")
                return 0
        elif args.input.is_dir():
            if not args.output.suffix:
                # Directory mode: pass both input and output directories to process_directory
                if process_directory(args.input, args.output):
                    log.info("Successfully processed directory %s", args.input)
                    return 0
            else:
                log.error("Output must be a directory when input is a directory")
                return 1
        else:
            log.error("Input %s does not exist", args.input)
            return 1
    except KeyboardInterrupt:
        log.warning("Encoding interrupted by user")
        return 130
    except Exception as e:
        log.exception("Encoding failed: %s", e)
        return 1
        
    log.error("Encoding failed")
    return 1

if __name__ == "__main__":
    sys.exit(main())
