//! Event helpers shared by the processing workflow.

use crate::events::{Event, EventDispatcher};

/// Helper function to emit events if event dispatcher is available
pub fn emit_event(event_dispatcher: Option<&EventDispatcher>, event: Event) {
    if let Some(dispatcher) = event_dispatcher {
        dispatcher.emit(event);
    }
}
