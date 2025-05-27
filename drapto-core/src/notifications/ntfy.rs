//! Notification implementation using the ntfy.sh service.
//!
//! This module provides functionality for sending notifications to ntfy.sh topics,
//! which can be received on various devices.
use crate::error::{CoreError, CoreResult};
use crate::notifications::NotificationType;

use ntfy::DispatcherBuilder;
use ntfy::payload::{Payload, Priority as NtfyPriority};

use log;


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
    /// Creates a new `NtfyNotificationSender` instance.
    ///
    /// # Arguments
    ///
    /// * `topic_url` - The full URL of the notification topic (e.g., "<https://ntfy.sh/your_topic>")
    ///
    /// # Returns
    ///
    /// * `Ok(NtfyNotificationSender)` - A new notification sender instance
    /// * `Err(CoreError)` - If the topic URL is invalid
    pub fn new(topic_url: &str) -> CoreResult<Self> {
        // Must start with https:// and have a topic path
        if !topic_url.starts_with("https://") {
            return Err(CoreError::NotificationError(format!(
                "Invalid ntfy topic URL '{topic_url}': must start with https://"
            )));
        }

        let after_scheme = &topic_url[8..];
        let host_end = after_scheme.find('/').unwrap_or(after_scheme.len());
        let host = &after_scheme[..host_end];

        if host.is_empty() {
            return Err(CoreError::NotificationError(format!(
                "URL '{topic_url}' must have a non-empty host"
            )));
        }

        let topic = if host_end < after_scheme.len() {
            &after_scheme[host_end + 1..]
        } else {
            ""
        };

        if topic.is_empty() {
            return Err(CoreError::NotificationError(format!(
                "URL '{topic_url}' is missing topic path"
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
    /// * `Err(CoreError)` - If an error occurred while sending the notification
    pub fn send_notification(&self, notification: &NotificationType) -> CoreResult<()> {
        // Extract base URL and topic from validated URL
        let after_scheme = &self.topic_url[8..];
        let host_end = after_scheme.find('/').unwrap_or(after_scheme.len());
        let host = &after_scheme[..host_end];
        let base_url = format!("https://{host}");
        let topic = if host_end < after_scheme.len() {
            &after_scheme[host_end + 1..]
        } else {
            ""
        };

        let dispatcher = DispatcherBuilder::new(&base_url)
            .build_blocking()
            .map_err(|e| {
                CoreError::NotificationError(format!(
                    "Failed to build ntfy dispatcher for {base_url}: {e}"
                ))
            })?;

        // Build notification payload
        let mut payload_builder = Payload::new(topic)
            .message(notification.get_message())
            .title(notification.get_title());

        let priority = if let Some(p) = map_priority(notification.get_priority()) { p } else {
            log::warn!(
                "Invalid ntfy priority value provided: {}",
                notification.get_priority()
            );
            NtfyPriority::Default
        };
        payload_builder = payload_builder.priority(priority);

        let mut tags = vec!["drapto".to_string()];
        match notification {
            NotificationType::EncodeStart { .. } => tags.push("start".to_string()),
            NotificationType::EncodeComplete { .. } => tags.push("complete".to_string()),
            NotificationType::EncodeError { .. } => tags.push("error".to_string()),
            NotificationType::Custom { .. } => {}
        }
        payload_builder = payload_builder.tags(tags);

        let final_payload = payload_builder;

        dispatcher.send(&final_payload).map_err(|e| {
            CoreError::NotificationError(format!(
                "Failed to send ntfy notification to {}: {}",
                self.topic_url, e
            ))
        })?;
        Ok(())
    }
}


/// Maps a numeric priority value to the corresponding ntfy Priority enum.
///
/// This function converts a simple numeric priority (1-5) to the corresponding
/// `ntfy::Priority` enum value. This makes it easier for consumers of the library
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
        1 => Some(NtfyPriority::Min),
        2 => Some(NtfyPriority::Low),
        3 => Some(NtfyPriority::Default),
        4 => Some(NtfyPriority::High),
        5 => Some(NtfyPriority::Max),
        _ => None,
    }
}
