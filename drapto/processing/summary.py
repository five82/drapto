"""Processing summary and logging utilities

Responsibilities:
- Set up logging for encode sessions
- Build and format encoding summaries
- Track execution time and file statistics
"""

import logging
import time
from pathlib import Path
from typing import Optional, Dict, Tuple

from ..config import LOG_DIR
from ..utils import get_timestamp, format_size, get_file_size
from ..formatting import (
    print_header, print_check, print_success,
    print_separator
)

def setup_encode_logging(input_file: Path) -> tuple[logging.FileHandler, Path]:
    """Setup logging for an encode session."""
    timestamp = get_timestamp()
    log_file = LOG_DIR / f"{input_file.stem}_{timestamp}.log"
    
    file_handler = logging.FileHandler(log_file)
    file_handler.setFormatter(logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s'))
    logging.root.addHandler(file_handler)
    
    return file_handler, log_file

def build_encode_summary(input_file: Path, output_file: Path, start_time: float) -> dict:
    """Build the encoding summary dictionary."""
    input_size = get_file_size(input_file)
    output_size = get_file_size(output_file)
    reduction = ((input_size - output_size) / input_size) * 100
    
    end_time = time.time()
    elapsed = end_time - start_time
    hours = int(elapsed // 3600)
    minutes = int((elapsed % 3600) // 60)
    seconds = int(elapsed % 60)
    finished_time = time.strftime("%a %b %d %H:%M:%S %Z %Y", time.localtime(end_time))
    
    print_header("Encoding Summary")
    print_success(f"Input size:  {format_size(input_size)}")
    print_success(f"Output size: {format_size(output_size)}")
    print_success(f"Reduction:   {reduction:.2f}%")
    print_check(f"Completed: {input_file.name}")
    print_check(f"Encoding time: {hours:02d}h {minutes:02d}m {seconds:02d}s")
    print_check(f"Finished encode at {finished_time}")
    print_separator()
    
    return {
        "output_file": output_file,
        "filename": input_file.name,
        "input_size": input_size,
        "output_size": output_size,
        "reduction": reduction,
        "encoding_time": elapsed
    }
