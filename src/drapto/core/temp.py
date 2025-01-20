"""Temporary file management for drapto.

This module provides interfaces and implementations for managing
temporary files and directories during media processing.
"""

import atexit
import shutil
from contextlib import contextmanager
from dataclasses import dataclass
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, Generator, List, Optional, Set

from drapto.core.exceptions import TemporaryFileError

@dataclass
class TempFileInfo:
    """Information about a temporary file.
    
    Attributes:
        path: Path to temporary file
        created: When file was created
        category: File category (e.g., 'segments', 'encoded')
        keep: Whether to keep file after processing
        metadata: Additional file metadata
    """
    path: Path
    created: datetime
    category: str
    keep: bool = False
    metadata: Dict[str, str] = None
    
    def __post_init__(self) -> None:
        """Initialize metadata if not provided."""
        if self.metadata is None:
            self.metadata = {}

class TempManager:
    """Manages temporary files and directories.
    
    Directory structure:
    TEMP_DIR/
    ├── logs/           # Processing logs
    ├── encode_data/    # Encoding state and metadata
    ├── segments/       # Video segments for chunked encoding
    ├── encoded/        # Encoded segments
    └── working/        # Temporary processing files
    """
    
    def __init__(self, base_dir: Path) -> None:
        """Initialize temporary file manager.
        
        Args:
            base_dir: Base directory for temporary files
            
        Raises:
            TemporaryFileError: If base directory cannot be created/accessed
        """
        self.base_dir = base_dir
        self._tracked_files: Dict[Path, TempFileInfo] = {}
        self._required_dirs = {
            'logs',
            'encode_data',
            'segments',
            'encoded',
            'working'
        }
        
        try:
            self._init_dirs()
        except Exception as e:
            raise TemporaryFileError(f"Failed to initialize temp directories: {e}")
        
        # Register cleanup on exit
        atexit.register(self.cleanup)
    
    def _init_dirs(self) -> None:
        """Initialize required directory structure."""
        self.base_dir.mkdir(parents=True, exist_ok=True)
        
        for dir_name in self._required_dirs:
            dir_path = self.base_dir / dir_name
            dir_path.mkdir(exist_ok=True)
    
    def get_path(self, category: str, name: str) -> Path:
        """Get path for a temporary file.
        
        Args:
            category: File category (must be one of required_dirs)
            name: File name
            
        Returns:
            Path object for the temporary file
            
        Raises:
            ValueError: If category is invalid
        """
        if category not in self._required_dirs:
            raise ValueError(f"Invalid category: {category}")
        
        return self.base_dir / category / name
    
    def track_file(self, path: Path, category: str, keep: bool = False) -> None:
        """Start tracking a temporary file.
        
        Args:
            path: Path to file
            category: File category
            keep: Whether to keep file after processing
        """
        self._tracked_files[path] = TempFileInfo(
            path=path,
            created=datetime.now(),
            category=category,
            keep=keep
        )
    
    def cleanup(self, max_age: Optional[timedelta] = None) -> None:
        """Clean up temporary files.
        
        Args:
            max_age: Maximum age for files to keep
        """
        now = datetime.now()
        
        for path, info in list(self._tracked_files.items()):
            if info.keep:
                continue
                
            if max_age and (now - info.created) <= max_age:
                continue
                
            try:
                if path.exists():
                    if path.is_file():
                        path.unlink()
                    elif path.is_dir():
                        shutil.rmtree(path)
                    del self._tracked_files[path]
            except Exception as e:
                print(f"Failed to remove temporary file {path}: {e}")
    
    @contextmanager
    def temp_file(self, category: str, name: str, keep: bool = False) -> Generator[Path, None, None]:
        """Context manager for temporary file.
        
        Args:
            category: File category
            name: File name
            keep: Whether to keep file after context exit
            
        Yields:
            Path to temporary file
        """
        path = self.get_path(category, name)
        self.track_file(path, category, keep)
        
        try:
            yield path
        finally:
            if not keep and path.exists():
                try:
                    if path.is_file():
                        path.unlink()
                    elif path.is_dir():
                        shutil.rmtree(path)
                    del self._tracked_files[path]
                except Exception as e:
                    print(f"Failed to remove temporary file {path}: {e}")
    
    def __del__(self) -> None:
        """Cleanup on deletion."""
        self.cleanup() 