// ============================================================================
// drapto-core/src/notifications/ntfy.rs
// ============================================================================
//
// NTFY IMPLEMENTATION: Notification Implementation Using ntfy.sh
//
// This module provides functionality for sending notifications using the ntfy.sh
// service. It allows for sending notifications to ntfy.sh topics, which can be
// received on various devices.
//
// KEY COMPONENTS:
// - NtfyNotificationSender: Sends notifications to ntfy.sh
//
// DESIGN PHILOSOPHY:
// This module follows a minimalist approach, providing a direct implementation
// for sending notifications to ntfy.sh without unnecessary abstraction layers.
//
// AI-ASSISTANT-INFO: ntfy.sh implementation for sending notifications

// ---- Internal crate imports ----
use crate::error::{CoreError, CoreResult};
use crate::notifications::abstraction::NotificationType;

// ---- External crate imports ----
use ntfy::DispatcherBuilder;
use ntfy::payload::{Payload, Priority as NtfyPriority};

use url::Url;

// ---- Standard library imports ----
use log;

// ============================================================================
// NTFY NOTIFICATION SENDER
// ============================================================================

/// Sends notifications to the ntfy.sh service.
///
/// This struct provides functionality for sending notifications to the ntfy.sh
/// service. It uses the blocking version of the ntfy crate to send notifications
/// synchronously.
///
/// # Examples
///
/// ```rust,no_run
/// use drapto_core::notifications::{NotificationType, NtfyNotificationSender};
/// use std::path::PathBuf;
/// use std::time::Duration;
///
/// // Create a new notification sender
/// let sender = NtfyNotificationSender::new("https://ntfy.sh/your_topic").unwrap();
///
/// // Send a notification
/// let notification = NotificationType::EncodeComplete {
///     input_path: PathBuf::from("/path/to/input.mkv"),
///     output_path: PathBuf::from("/path/to/output.mkv"),
///     input_size: 1000000,
///     output_size: 500000,
///     duration: Duration::from_secs(300),
///     hostname: "my-computer".to_string(),
/// };
///
/// sender.send_notification(&notification).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct NtfyNotificationSender {
    /// The topic URL to send notifications to
    topic_url: String,
}

impl NtfyNotificationSender {
    /// Creates a new NtfyNotificationSender instance.
    ///
    /// # Arguments
    ///
    /// * `topic_url` - The full URL of the notification topic (e.g., "https://ntfy.sh/your_topic")
    ///
    /// # Returns
    ///
    /// * `Ok(NtfyNotificationSender)` - A new notification sender instance
    /// * `Err(CoreError)` - If the topic URL is invalid
    pub fn new(topic_url: &str) -> CoreResult<Self> {
        // Validate the topic URL
        let parsed_url = Url::parse(topic_url)
            .map_err(|e| CoreError::NotificationError(format!(
                "Invalid ntfy topic URL '{}': {}",
                topic_url, e
            )))?;

        // Ensure the host is present and non-empty
        let _host = match parsed_url.host_str() {
            Some(h) if !h.is_empty() => h,
            _ => return Err(CoreError::NotificationError(format!(
                "URL '{}' must have a non-empty host",
                topic_url
            ))),
        };

        // Extract the topic from the path (removing leading slash)
        let topic = parsed_url.path().trim_start_matches('/');

        // Ensure the topic is not empty
        if topic.is_empty() {
             return Err(CoreError::NotificationError(format!(
                 "URL '{}' is missing topic path",
                 topic_url
             )));
        }

        Ok(Self {
            topic_url: topic_url.to_string(),
        })
    }

    /// Sends a notification to the ntfy.sh service.
    ///
    /// # Arguments
    ///
    /// * `notification` - The notification to send
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the notification was sent successfully
    /// * `Err(String)` - If an error occurred while sending the notification
    pub fn send_notification(&self, notification: &NotificationType) -> Result<(), String> {
        // Parse the URL (already validated in new())
        let parsed_url = Url::parse(&self.topic_url)
            .map_err(|e| format!("Invalid ntfy topic URL '{}': {}", self.topic_url, e))?;

        // Extract the base URL and topic
        let host = parsed_url.host_str().unwrap_or("");
        let base_url = format!("{}://{}", parsed_url.scheme(), host);
        let topic = parsed_url.path().trim_start_matches('/');

        // Build the ntfy dispatcher
        let dispatcher = match DispatcherBuilder::new(&base_url).build_blocking() {
            Ok(d) => d,
            Err(e) => return Err(format!("Failed to build ntfy dispatcher for {}: {}", base_url, e)),
        };

        // Build the notification payload
        // Start with the required fields (topic and message)
        let mut payload_builder = Payload::new(topic)
            .message(&notification.get_message())
            .title(&notification.get_title());

        // Add priority
        let priority = match map_priority(notification.get_priority()) {
            Some(p) => p,
            None => {
                log::warn!("Invalid ntfy priority value provided: {}", notification.get_priority());
                NtfyPriority::Default
            }
        };
        payload_builder = payload_builder.priority(priority);

        // Add tags
        let mut tags = vec!["drapto".to_string()];
        match notification {
            NotificationType::EncodeStart { .. } => tags.push("start".to_string()),
            NotificationType::EncodeComplete { .. } => tags.push("complete".to_string()),
            NotificationType::EncodeError { .. } => tags.push("error".to_string()),
            NotificationType::Custom { .. } => {}
        }
        payload_builder = payload_builder.tags(tags);

        // Finalize the payload
        let final_payload = payload_builder;

        // Send the notification
        match dispatcher.send(&final_payload) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to send ntfy notification to {}: {}", self.topic_url, e)),
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Maps a numeric priority value to the corresponding ntfy Priority enum.
///
/// This function converts a simple numeric priority (1-5) to the corresponding
/// ntfy::Priority enum value. This makes it easier for consumers of the library
/// to specify priorities without needing to know the specific enum values.
///
/// # Arguments
///
/// * `p` - Numeric priority value (1-5)
///
/// # Returns
///
/// * `Some(NtfyPriority)` - The corresponding ntfy Priority enum value
/// * `None` - If the input priority is outside the valid range
///
/// # Priority Mapping
///
/// * 1 -> Min (lowest priority)
/// * 2 -> Low
/// * 3 -> Default (normal priority)
/// * 4 -> High
/// * 5 -> Max (highest priority)
fn map_priority(p: u8) -> Option<NtfyPriority> {
    match p {
        1 => Some(NtfyPriority::Min),     // Lowest priority
        2 => Some(NtfyPriority::Low),     // Low priority
        3 => Some(NtfyPriority::Default), // Normal priority
        4 => Some(NtfyPriority::High),    // High priority
        5 => Some(NtfyPriority::Max),     // Highest priority
        _ => None,                        // Invalid priority value
    }
}

