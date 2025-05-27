//! Notification system for sending encoding status updates.
//!
//! This module provides functionality for sending notifications about encoding
//! progress and status using the ntfy.sh service to deliver push notifications.
mod abstraction;
mod ntfy;

pub use abstraction::NotificationType;
pub use ntfy::NtfyNotificationSender;
