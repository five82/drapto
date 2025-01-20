"""Path monitoring utilities for drapto."""
import os
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional

from ..utils.paths import normalize_path


@dataclass
class PathUsage:
    """Track usage statistics for a path."""
    path: Path
    access_count: int = 0
    last_access: float = 0.0
    total_size: int = 0
    is_error: bool = False
    error_message: Optional[str] = None

    def update_access(self) -> None:
        """Update access statistics."""
        self.access_count += 1
        self.last_access = time.time()
        if self.path.exists():
            try:
                if self.path.is_file():
                    self.total_size = self.path.stat().st_size
                elif self.path.is_dir():
                    self.total_size = sum(f.stat().st_size for f in self.path.rglob('*') if f.is_file())
            except (OSError, PermissionError) as e:
                self.is_error = True
                self.error_message = str(e)

    def mark_error(self, message: str) -> None:
        """Mark path as having an error."""
        self.is_error = True
        self.error_message = message
        self.last_access = time.time()


class PathMonitor:
    """Monitor path usage and track issues."""
    def __init__(self) -> None:
        """Initialize path monitor."""
        self._paths: Dict[Path, PathUsage] = {}

    def track_path(self, path: Path) -> None:
        """Start tracking a path.
        
        Args:
            path: The path to track
        """
        path = normalize_path(path)
        if path not in self._paths:
            self._paths[path] = PathUsage(path)

    def record_access(self, path: Path) -> None:
        """Record an access to a path.
        
        Args:
            path: The path that was accessed
        """
        path = normalize_path(path)
        if path not in self._paths:
            self.track_path(path)
        self._paths[path].update_access()

    def record_error(self, path: Path, message: str) -> None:
        """Record an error with a path.
        
        Args:
            path: The path that had an error
            message: The error message
        """
        path = normalize_path(path)
        if path not in self._paths:
            self.track_path(path)
        self._paths[path].mark_error(message)

    def get_usage(self, path: Path) -> Optional[PathUsage]:
        """Get usage statistics for a path.
        
        Args:
            path: The path to get statistics for
            
        Returns:
            PathUsage object if path is tracked, None otherwise
        """
        path = normalize_path(path)
        return self._paths.get(path)

    def get_all_usage(self) -> List[PathUsage]:
        """Get usage statistics for all tracked paths.
        
        Returns:
            List of PathUsage objects
        """
        return list(self._paths.values())

    def get_errors(self) -> List[PathUsage]:
        """Get all paths with errors.
        
        Returns:
            List of PathUsage objects for paths with errors
        """
        return [usage for usage in self._paths.values() if usage.is_error]

    def clear(self) -> None:
        """Clear all tracked paths."""
        self._paths.clear()


# Global path monitor instance
path_monitor = PathMonitor() 