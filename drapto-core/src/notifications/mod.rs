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
// - NotificationSender: Trait defining the notification interface
// - NullNotificationSender: No-op implementation for when notifications aren't needed
// - NtfyNotificationSender: Implementation using the ntfy.sh service
//
// DESIGN PHILOSOPHY:
// The notification system follows the dependency injection pattern through the
// NotificationSender trait, allowing for different notification backends or mock
// implementations for testing. The default implementation uses the ntfy crate to
// send notifications to the ntfy.sh service.
//
// AI-ASSISTANT-INFO: Notification system for sending encoding status updates

// ---- Module declarations ----
mod abstraction;
mod ntfy;

// ---- Re-exports ----
pub use abstraction::{NotificationSender, NotificationType, NullNotificationSender};
pub use ntfy::NtfyNotificationSender;
