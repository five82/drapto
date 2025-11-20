//! Event and notification helpers shared by the processing workflow.

use crate::events::{Event, EventDispatcher};
use crate::notifications::NotificationSender;
use log::warn;

/// Helper function to safely send notifications with consistent error handling.
pub fn send_notification_safe(
    sender: Option<&dyn NotificationSender>,
    message: &str,
    context: &str,
) {
    if let Some(sender) = sender {
        if let Err(e) = sender.send(message) {
            warn!("Failed to send {} notification: {e}", context);
        }
    }
}

/// Helper function to send individual validation failure notifications
pub fn send_validation_failure_notifications(
    sender: Option<&dyn NotificationSender>,
    filename: &str,
    validation_steps: &[(String, bool, String)],
) {
    let failures: Vec<&(String, bool, String)> = validation_steps
        .iter()
        .filter(|(_, passed, _)| !passed)
        .collect();

    if failures.is_empty() {
        return;
    }

    // Send individual notifications for each failure
    for (step_name, _, message) in failures.iter() {
        let notification_msg = format!(
            "{}: {} validation failed - {}",
            filename, step_name, message
        );
        send_notification_safe(sender, &notification_msg, "validation_failure");
    }

    // Send summary notification if multiple failures
    if failures.len() > 1 {
        let summary_msg = format!(
            "{}: {} validation checks failed (encoding completed)",
            filename,
            failures.len()
        );
        send_notification_safe(sender, &summary_msg, "validation_summary");
    }
}

/// Helper function to emit events if event dispatcher is available
pub fn emit_event(event_dispatcher: Option<&EventDispatcher>, event: Event) {
    if let Some(dispatcher) = event_dispatcher {
        dispatcher.emit(event);
    }
}
