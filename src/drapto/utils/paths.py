"""Path handling utilities for drapto."""
import os
from pathlib import Path
from typing import Optional


def ensure_directory(path: Path, create: bool = True) -> Path:
    """Ensure a directory exists and is writable.
    
    Args:
        path: The directory path to check/create
        create: Whether to create the directory if it doesn't exist
        
    Returns:
        The resolved path
        
    Raises:
        ValueError: If the path exists but is not a directory or not writable,
                  or if it doesn't exist and create=False
    """
    if not path.exists():
        if not create:
            raise ValueError(f"Directory does not exist: {path}")
        try:
            path.mkdir(parents=True, exist_ok=True)
        except (OSError, PermissionError) as e:
            raise ValueError(f"Cannot create directory '{path}': {e}")
    elif not path.is_dir():
        raise ValueError(f"Path exists but is not a directory: {path}")
    elif not os.access(path, os.W_OK):
        raise ValueError(f"Directory exists but is not writable: {path}")
    
    return path.resolve()


def is_under_directory(path: Path, parent: Path) -> bool:
    """Check if a path is under a parent directory.
    
    Args:
        path: The path to check
        parent: The parent directory path
        
    Returns:
        True if path is under parent, False otherwise
    """
    try:
        return path.resolve().is_relative_to(parent.resolve())
    except (ValueError, RuntimeError):
        return False


def normalize_path(path: Path | str) -> Path:
    """Normalize a path to an absolute Path object.
    
    Args:
        path: The path to normalize
        
    Returns:
        Normalized absolute Path object
    """
    if isinstance(path, str):
        path = Path(path)
    return path.resolve()


def get_relative_path(path: Path, base: Path) -> Path:
    """Get the relative path from a base directory.
    
    Args:
        path: The path to make relative
        base: The base directory
        
    Returns:
        Relative path from base to path
        
    Raises:
        ValueError: If path is not under base directory
    """
    path = normalize_path(path)
    base = normalize_path(base)
    
    if not is_under_directory(path, base):
        raise ValueError(f"Path '{path}' is not under base directory '{base}'")
    
    return path.relative_to(base)


def get_temp_path(base_dir: Path, prefix: str = "", suffix: str = "", create: bool = True) -> Path:
    """Get a unique temporary path under a base directory.
    
    Args:
        base_dir: The base directory for the temporary path
        prefix: Optional prefix for the temporary directory name
        suffix: Optional suffix for the temporary directory name
        create: Whether to create the base directory if it doesn't exist
        
    Returns:
        A unique path under the base directory
        
    Raises:
        ValueError: If base_dir is not a directory or not writable,
                  or if it doesn't exist and create=False
    """
    ensure_directory(base_dir, create)
    
    # Try to create a unique path
    for _ in range(100):  # Limit attempts to avoid infinite loop
        name = f"{prefix}{os.urandom(8).hex()}{suffix}"
        path = base_dir / name
        if not path.exists():
            return path
            
    raise ValueError(f"Could not create unique temporary path in {base_dir}") 