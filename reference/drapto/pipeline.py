"""High-level pipeline orchestration for video encoding

Responsibilities:
  - Coordinate the overall encoding pipeline flow
  - Import and expose processing functions
  - Maintain backward compatibility for existing code
"""

from .processing import process_file, process_directory

__all__ = ['process_file', 'process_directory']
