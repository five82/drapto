"""Centralized logging configuration for drapto

This module handles:
- Setting up Rich-based console logging with proper formatting
- Configuring file-based logging with rotation
- Managing log levels and output destinations
- Providing a LogTracker utility for capturing logs during operations
- Coordinating logging across all pipeline components

The logging configuration ensures consistent formatting and proper
log aggregation throughout the encoding pipeline.
"""

import logging
from rich.logging import RichHandler
from typing import Optional
from pathlib import Path
from .config import LOG_DIR

def configure_logging(log_level: str = "INFO", file_logging: bool = True) -> None:
    """Central logging configuration for all modules"""
    logger = logging.getLogger("drapto")
    logger.setLevel(log_level)
    
    # Remove existing handlers
    for handler in logger.handlers[:]:
        logger.removeHandler(handler)
        
    # Rich console handler
    console_handler = RichHandler(show_path=False, rich_tracebacks=True)
    console_handler.setFormatter(logging.Formatter("%(message)s"))
    logger.addHandler(console_handler)
    
    if file_logging:
        # Simple file handler for main log
        log_file = LOG_DIR / "drapto.log"
        file_handler = logging.FileHandler(log_file)
        file_formatter = logging.Formatter(
            "%(asctime)s - %(name)s - %(levelname)s - %(message)s"
        )
        file_handler.setFormatter(file_formatter)
        logger.addHandler(file_handler)
    
    # Capture warnings
    logging.captureWarnings(True)

class LogTracker:
    """Track log messages during operations"""
    def __init__(self):
        self.messages = []
        
    def capture(self, msg: str, level: str):
        self.messages.append((level, msg))
        
    def get_errors(self) -> list:
        return [msg for level, msg in self.messages if level in ("ERROR", "CRITICAL")]
