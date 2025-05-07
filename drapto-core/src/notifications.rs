// ============================================================================
// drapto-core/src/notifications.rs
// ============================================================================
//
// NOTIFICATIONS: Sending Progress and Status Updates
//
// This module provides functionality for sending notifications about encoding
// progress and status. It uses the ntfy.sh service to deliver push notifications
// to users about encoding start, completion, and errors.
//
// KEY COMPONENTS:
// - Notifier: Trait defining the notification interface
// - NtfyNotifier: Implementation using the ntfy.sh service
//
// DESIGN PHILOSOPHY:
// The notification system follows the dependency injection pattern through the
// Notifier trait, allowing for different notification backends or mock implementations
// for testing. The default implementation uses the ntfy crate to send notifications
// to the ntfy.sh service.
//
// AI-ASSISTANT-INFO: Notification system for sending encoding status updates

// ---- Internal crate imports ----
use crate::error::{CoreError, CoreResult};

// ---- External crate imports ----
use ntfy::DispatcherBuilder;
use ntfy::payload::{Payload, Priority as NtfyPriority};
use ntfy::error::Error as NtfyError;
use url::Url;

// ---- Standard library imports ----
// None needed for this module

// ============================================================================
// NOTIFICATION INTERFACE
// ============================================================================

/// Trait for sending notifications about encoding progress and status.
///
/// This trait defines the interface for sending notifications from the drapto-core
/// library. It allows for different notification backends to be used, following
/// the dependency injection pattern.
///
/// # Examples
///
/// ```rust,no_run
/// use drapto_core::notifications::Notifier;
/// use drapto_core::CoreResult;
///
/// struct MockNotifier;
///
/// impl Notifier for MockNotifier {
///     fn send(
///         &self,
///         topic_url: &str,
///         message: &str,
///         title: Option<&str>,
///         priority: Option<u8>,
///         tags: Option<&str>,
///     ) -> CoreResult<()> {
///         println!("MOCK NOTIFICATION: {} - {}", title.unwrap_or("No title"), message);
///         Ok(())
///     }
/// }
///
/// // Use the mock notifier in tests
/// let notifier = MockNotifier;
/// notifier.send(
///     "https://ntfy.sh/test",
///     "Encoding complete!",
///     Some("Drapto"),
///     Some(3),
///     Some("success"),
/// ).unwrap();
/// ```
pub trait Notifier {
    /// Sends a notification to the specified topic.
    ///
    /// # Arguments
    ///
    /// * `topic_url` - The full URL of the notification topic (e.g., "https://ntfy.sh/your_topic")
    /// * `message` - The main content of the notification
    /// * `title` - Optional title for the notification
    /// * `priority` - Optional priority level (1-5, where 1 is lowest and 5 is highest)
    /// * `tags` - Optional comma-separated list of tags (e.g., "success,complete")
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the notification was sent successfully
    /// * `Err(CoreError)` - If an error occurred sending the notification
    fn send(
        &self,
        topic_url: &str,
        message: &str,
        title: Option<&str>,
        priority: Option<u8>,
        tags: Option<&str>,
    ) -> CoreResult<()>;
}

// ============================================================================
// NTFY IMPLEMENTATION
// ============================================================================

/// Implementation of `Notifier` using the ntfy.sh service via the `ntfy` crate.
///
/// This struct provides a concrete implementation of the Notifier trait that
/// sends notifications to the ntfy.sh service. It uses the blocking version of
/// the ntfy crate to send notifications synchronously.
///
/// # Examples
///
/// ```rust,no_run
/// use drapto_core::notifications::{Notifier, NtfyNotifier};
///
/// // Create a new notifier
/// let notifier = NtfyNotifier::new().unwrap();
///
/// // Send a notification
/// notifier.send(
///     "https://ntfy.sh/your_topic",
///     "Encoding complete!",
///     Some("Drapto"),
///     Some(3),
///     Some("success"),
/// ).unwrap();
/// ```
#[derive(Debug, Default)]
pub struct NtfyNotifier;

impl NtfyNotifier {
    /// Creates a new NtfyNotifier instance.
    ///
    /// This function creates a new NtfyNotifier with default settings.
    /// The actual connection to the ntfy.sh service is established when
    /// the `send` method is called.
    ///
    /// # Returns
    ///
    /// * `Ok(NtfyNotifier)` - A new notifier instance
    /// * `Err(CoreError)` - This currently always returns Ok but may return
    ///   errors in the future if initialization becomes more complex
    pub fn new() -> CoreResult<Self> {
        // No complex client setup needed here
        Ok(Self)
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

// ============================================================================
// NOTIFIER IMPLEMENTATION
// ============================================================================

impl Notifier for NtfyNotifier {
    fn send(
        &self,
        topic_url: &str,
        message: &str,
        title: Option<&str>,
        priority: Option<u8>,
        tags: Option<&str>,
    ) -> CoreResult<()> {
        // STEP 1: Parse the full topic URL
        let parsed_url = Url::parse(topic_url)
            .map_err(|e| CoreError::NotificationError(format!(
                "Invalid ntfy topic URL '{}': {}",
                topic_url, e
            )))?;

        // STEP 2: Extract and validate base URL and topic path
        // Ensure the host is present and non-empty
        let host = match parsed_url.host_str() {
            Some(h) if !h.is_empty() => h,
            _ => return Err(CoreError::NotificationError(format!(
                "URL '{}' must have a non-empty host",
                topic_url
            ))),
        };

        // Construct the base URL (scheme + host)
        let base_url = format!("{}://{}", parsed_url.scheme(), host);

        // Extract the topic from the path (removing leading slash)
        let topic = parsed_url.path().trim_start_matches('/');

        // Ensure the topic is not empty
        if topic.is_empty() {
             return Err(CoreError::NotificationError(format!(
                 "URL '{}' is missing topic path",
                 topic_url
             )));
        }

        // STEP 3: Build the ntfy dispatcher
        let dispatcher = DispatcherBuilder::new(&base_url)
            // Future enhancements could add proxy/auth support here
            .build_blocking()
            .map_err(|e: NtfyError| CoreError::NotificationError(format!(
                "Failed to build ntfy dispatcher for {}: {}",
                base_url, e
            )))?;

        // STEP 4: Build the notification payload
        // Start with the required fields (topic and message)
        let mut payload_builder = Payload::new(topic).message(message);

        // Add optional title if provided
        if let Some(t) = title {
            payload_builder = payload_builder.title(t);
        }

        // Add optional priority if provided and valid
        if let Some(p_val) = priority {
            if let Some(ntfy_p) = map_priority(p_val) {
                payload_builder = payload_builder.priority(ntfy_p);
            } else {
                // Log a warning for invalid priority values
                log::warn!("Invalid ntfy priority value provided: {}", p_val);
            }
        }

        // Process tags: combine input tags with "drapto"
        let mut final_tags: Vec<String> = tags
            .unwrap_or("")
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();

        // Always include the "drapto" tag for identification
        if !final_tags.iter().any(|t| t == "drapto") {
             final_tags.push("drapto".to_string());
        }

        // Add tags to the payload if there are any
        if !final_tags.is_empty() {
             payload_builder = payload_builder.tags(final_tags);
        }

        // Finalize the payload
        let final_payload = payload_builder;

        // STEP 5: Send the notification
        dispatcher.send(&final_payload)
            .map_err(|e: NtfyError| CoreError::NotificationError(format!(
                "Failed to send ntfy notification to {}: {}",
                topic_url, e
            )))
    }
}

// Removed deprecated send_ntfy function
