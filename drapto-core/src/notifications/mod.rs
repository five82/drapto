// ============================================================================
// drapto-core/src/notifications/mod.rs
// ============================================================================
//
// NOTIFICATIONS: Sending Progress and Status Updates
//
// This module provides functionality for sending notifications about encoding
// progress and status. It uses the ntfy.sh service to deliver push notifications
// to users about encoding start, completion, and errors.
//
// KEY COMPONENTS:
// - NotificationType: Enum defining different types of notifications
// - NtfyNotificationSender: Implementation using the ntfy.sh service
//
// DESIGN PHILOSOPHY:
// The notification system follows a minimalist approach, focusing on the core
// functionality of sending notifications without unnecessary abstraction layers.
// The implementation uses the ntfy crate to send notifications to the ntfy.sh service.
//
// AI-ASSISTANT-INFO: Notification system for sending encoding status updates

// ---- Module declarations ----
mod abstraction;
mod ntfy;

// ---- Re-exports ----
pub use abstraction::NotificationType;
pub use ntfy::NtfyNotificationSender;
