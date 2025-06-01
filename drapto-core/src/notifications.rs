//! Simplified notification system for sending encoding status updates via ntfy.sh.
//!
//! This module provides a direct implementation for sending notifications about encoding
//! progress and status using the ntfy.sh service.

use crate::error::{CoreError, CoreResult};
use ntfy::DispatcherBuilder;
use ntfy::payload::{Payload, Priority as NtfyPriority};

/// A simple notification structure for sending messages via ntfy.sh.
#[derive(Debug, Clone)]
pub struct Notification {
    pub title: String,
    pub message: String,
    pub priority: u8,  // 1-5, where 5 is highest
    pub tags: Vec<String>,
}

impl Notification {
    /// Creates a new notification with default priority (3) and "drapto" tag.
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            priority: 3,
            tags: vec!["drapto".to_string()],
        }
    }

    /// Sets the priority level (1-5).
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Adds a tag to the notification.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Replaces all tags with the provided tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

/// Sends notifications to the ntfy.sh service.
///
/// # Examples
///
/// ```rust,no_run
/// use drapto_core::notifications::{Notification, NtfyNotificationSender};
///
/// let sender = NtfyNotificationSender::new("https://ntfy.sh/your_topic").unwrap();
///
/// let notification = Notification::new("Encoding Complete", "Video processed successfully")
///     .with_priority(4)
///     .with_tag("complete");
///
/// sender.send(&notification).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct NtfyNotificationSender {
    topic_url: String,
}

impl NtfyNotificationSender {
    /// Creates notification sender. URL must be https:// with non-empty host and topic.
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
    pub fn send(&self, notification: &Notification) -> CoreResult<()> {
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
            .message(&notification.message)
            .title(&notification.title);

        // Map numeric priority to ntfy priority enum
        let priority = match notification.priority {
            1 => NtfyPriority::Min,
            2 => NtfyPriority::Low,
            3 => NtfyPriority::Default,
            4 => NtfyPriority::High,
            5 => NtfyPriority::Max,
            _ => {
                crate::progress_reporting::warning(&format!("Invalid ntfy priority value: {}, using default", notification.priority));
                NtfyPriority::Default
            }
        };
        payload_builder = payload_builder.priority(priority);

        if !notification.tags.is_empty() {
            payload_builder = payload_builder.tags(notification.tags.clone());
        }

        dispatcher.send(&payload_builder).map_err(|e| {
            CoreError::NotificationError(format!(
                "Failed to send ntfy notification to {}: {}",
                self.topic_url, e
            ))
        })?;

        Ok(())
    }

}