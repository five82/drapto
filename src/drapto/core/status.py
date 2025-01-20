"""Status streaming interface for drapto.

This module provides interfaces for real-time status updates
and progress tracking during media processing.
"""

from abc import ABC, abstractmethod
from dataclasses import dataclass
from datetime import datetime
from enum import Enum, auto
from typing import Dict, Optional, Any

class ProcessingStage(Enum):
    """Processing stages for status tracking."""
    INIT = auto()
    ANALYZING = auto()
    ENCODING = auto()
    MUXING = auto()
    FINALIZING = auto()
    COMPLETE = auto()
    ERROR = auto()

@dataclass
class ProcessingStatus:
    """Current processing status.
    
    Attributes:
        stage: Current processing stage
        progress: Progress percentage (0-100)
        message: Status message
        details: Additional status details
        timestamp: When status was updated
    """
    stage: ProcessingStage
    progress: float
    message: str
    details: Dict[str, Any]
    timestamp: datetime = None
    
    def __post_init__(self) -> None:
        """Initialize timestamp if not provided."""
        if self.timestamp is None:
            self.timestamp = datetime.now()
        if not 0 <= self.progress <= 100:
            raise ValueError("Progress must be between 0 and 100")

class StatusStream(ABC):
    """Interface for status streaming and progress updates."""
    
    @abstractmethod
    def update_progress(self, percent: float, message: str) -> None:
        """Update processing progress.
        
        Args:
            percent: Progress percentage (0-100)
            message: Progress message
            
        Raises:
            ValueError: If percent is not between 0 and 100
        """
        pass
    
    @abstractmethod
    def update_stage(self, stage: ProcessingStage, details: Dict[str, Any]) -> None:
        """Update processing stage.
        
        Args:
            stage: New processing stage
            details: Stage-specific details
        """
        pass
    
    @abstractmethod
    def error(self, error: str, details: Dict[str, Any]) -> None:
        """Report an error.
        
        Args:
            error: Error message
            details: Error details
        """
        pass
    
    @abstractmethod
    def get_status(self) -> ProcessingStatus:
        """Get current processing status.
        
        Returns:
            Current status object
        """
        pass

class ConsoleStatusStream(StatusStream):
    """Status stream implementation that writes to console."""
    
    def __init__(self) -> None:
        """Initialize console status stream."""
        self._current_status = ProcessingStatus(
            stage=ProcessingStage.INIT,
            progress=0.0,
            message="Initializing",
            details={},
            timestamp=datetime.now()
        )
    
    def update_progress(self, percent: float, message: str) -> None:
        """Update progress with console output."""
        if not 0 <= percent <= 100:
            raise ValueError("Progress must be between 0 and 100")
        
        self._current_status = ProcessingStatus(
            stage=self._current_status.stage,
            progress=percent,
            message=message,
            details=self._current_status.details
        )
        print(f"Progress: {percent:.1f}% - {message}")
    
    def update_stage(self, stage: ProcessingStage, details: Dict[str, Any]) -> None:
        """Update stage with console output."""
        self._current_status = ProcessingStatus(
            stage=stage,
            progress=self._current_status.progress,
            message=self._current_status.message,
            details=details
        )
        print(f"Stage: {stage.name}")
    
    def error(self, error: str, details: Dict[str, Any]) -> None:
        """Report error to console."""
        self._current_status = ProcessingStatus(
            stage=ProcessingStage.ERROR,
            progress=self._current_status.progress,
            message=error,
            details=details
        )
        print(f"Error: {error}")
    
    def get_status(self) -> ProcessingStatus:
        """Get current status."""
        return self._current_status 