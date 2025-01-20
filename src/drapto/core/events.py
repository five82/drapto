"""Event system for drapto.

This module provides an event-driven communication system
for coordinating between different drapto components.
"""

from dataclasses import dataclass
from datetime import datetime
from enum import Enum, auto
from typing import Any, Callable, Dict, List, Optional, Set

class EventType(Enum):
    """Core event types."""
    FILE_DISCOVERED = auto()
    ANALYSIS_STARTED = auto()
    ANALYSIS_COMPLETE = auto()
    ENCODING_STARTED = auto()
    SEGMENT_COMPLETE = auto()
    ENCODING_COMPLETE = auto()
    ERROR_OCCURRED = auto()
    PROGRESS_UPDATE = auto()

@dataclass
class Event:
    """Event data container.
    
    Attributes:
        type: Type of event
        timestamp: When the event occurred
        data: Event-specific data
        source: Component that generated the event
    """
    type: EventType
    timestamp: datetime
    data: Dict[str, Any]
    source: str

class EventEmitter:
    """Base event system for component communication."""
    
    def __init__(self) -> None:
        """Initialize event emitter."""
        self._handlers: Dict[EventType, Set[Callable[[Event], None]]] = {}
        self._error_handlers: Set[Callable[[Exception], None]] = set()
    
    def on(self, event_type: EventType, handler: Callable[[Event], None]) -> None:
        """Register an event handler.
        
        Args:
            event_type: Type of event to handle
            handler: Callback function for the event
        """
        if event_type not in self._handlers:
            self._handlers[event_type] = set()
        self._handlers[event_type].add(handler)
    
    def off(self, event_type: EventType, handler: Callable[[Event], None]) -> None:
        """Remove an event handler.
        
        Args:
            event_type: Type of event to remove handler from
            handler: Handler to remove
        """
        if event_type in self._handlers:
            self._handlers[event_type].discard(handler)
            if not self._handlers[event_type]:
                del self._handlers[event_type]
    
    def on_error(self, handler: Callable[[Exception], None]) -> None:
        """Register an error handler.
        
        Args:
            handler: Error handling callback
        """
        self._error_handlers.add(handler)
    
    def off_error(self, handler: Callable[[Exception], None]) -> None:
        """Remove an error handler.
        
        Args:
            handler: Error handler to remove
        """
        self._error_handlers.discard(handler)
    
    def emit(self, event_type: EventType, data: Dict[str, Any], source: str) -> None:
        """Emit an event to registered handlers.
        
        Args:
            event_type: Type of event to emit
            data: Event data
            source: Component emitting the event
        """
        event = Event(
            type=event_type,
            timestamp=datetime.now(),
            data=data,
            source=source
        )
        
        if event_type in self._handlers:
            for handler in self._handlers[event_type]:
                try:
                    handler(event)
                except Exception as e:
                    self._handle_error(e)
    
    def _handle_error(self, error: Exception) -> None:
        """Handle errors in event handlers.
        
        Args:
            error: Exception that occurred
        """
        if self._error_handlers:
            for handler in self._error_handlers:
                try:
                    handler(error)
                except Exception:
                    pass  # Prevent error handler loops 