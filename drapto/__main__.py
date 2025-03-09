"""
Command-line interface for drapto video encoding pipeline
"""
import argparse
import logging
import sys
from pathlib import Path

from rich.logging import RichHandler

from . import __version__
from .config import LOG_DIR
from .utils import get_timestamp
from .formatting import print_header, print_error, print_info, print_success
from .pipeline import process_directory, process_file
from .utils import check_dependencies

def setup_logging(log_level: str = None):
    """Configure logging with rich output using the specified logging level"""
    from drapto.config import LOG_LEVEL
    # Use the provided log_level or fallback to the one in config.py
    level = log_level if log_level is not None else LOG_LEVEL
    # Convert the level (a string) to its numeric value using logging._nameToLevel
    numeric_level = logging._nameToLevel.get(level.upper(), logging.INFO)

    # Configure both console and file handlers
    handlers = [RichHandler(rich_tracebacks=True, show_path=False)]
    
    # Add file handler with timestamp-based filename
    log_file = LOG_DIR / f"drapto_{get_timestamp()}.log"
    file_handler = logging.FileHandler(log_file)
    file_handler.setFormatter(logging.Formatter(
        '%(asctime)s - %(name)s - %(levelname)s - %(message)s'
    ))
    handlers.append(file_handler)
    
    logging.basicConfig(
        level=numeric_level,
        format="%(message)s",
        datefmt="[%X]",
        handlers=handlers
    )
    
    # Log the start of a new session
    log = logging.getLogger("drapto")
    log.info("Started new logging session")
    log.info("Log file: %s", log_file)

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
        "--log-level",
        dest="log_level",
        choices=["DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"],
        default=None,
        help="Set logging level (default from config: %(default)s)"
    )
    parser.add_argument(
        "--disable-crop",
        dest="disable_crop",
        action="store_true",
        help="Disable automatic crop detection"
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
    args = parse_args()
    setup_logging(args.log_level)
    
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
