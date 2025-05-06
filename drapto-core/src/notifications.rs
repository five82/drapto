// drapto-core/src/notifications.rs
//
// Module for handling ntfy notifications.

use crate::error::{CoreError, CoreResult};
use ntfy::DispatcherBuilder; // Use blocking dispatcher builder
use ntfy::payload::{Payload, Priority as NtfyPriority}; // Renamed Priority to avoid conflict if needed elsewhere
use ntfy::error::Error as NtfyError;
use url::Url; // For parsing the topic URL

// Removed NTFY_TIMEOUT_SECS as timeout is handled by ntfy/ureq internally or via its builder if needed

/// Trait for sending notifications.
pub trait Notifier {
    /// Sends a notification.
    fn send(
        &self,
        topic_url: &str,
        message: &str,
        title: Option<&str>,
        priority: Option<u8>,
        tags: Option<&str>,
    ) -> CoreResult<()>;
}

/// Implementation of `Notifier` using the `ntfy` crate (blocking).
#[derive(Debug, Default)] // Added derive for convenience
pub struct NtfyNotifier; // Simplified struct, dispatcher created dynamically

impl NtfyNotifier {
    /// Creates a new NtfyNotifier.
    pub fn new() -> CoreResult<Self> {
        // No complex client setup needed here
        Ok(Self)
    }
}

// Helper function to map u8 priority to ntfy::Priority
fn map_priority(p: u8) -> Option<NtfyPriority> {
    match p {
        1 => Some(NtfyPriority::Min),
        2 => Some(NtfyPriority::Low),
        3 => Some(NtfyPriority::Default),
        4 => Some(NtfyPriority::High),
        5 => Some(NtfyPriority::Max),
        _ => None, // Ignore invalid priorities or treat as default? Let's ignore for now.
    }
}


impl Notifier for NtfyNotifier {
    fn send(
        &self,
        topic_url: &str,
        message: &str,
        title: Option<&str>,
        priority: Option<u8>,
        tags: Option<&str>,
    ) -> CoreResult<()> {
        // 1. Parse the full topic URL
        let parsed_url = Url::parse(topic_url)
            .map_err(|e| CoreError::NotificationError(format!("Invalid ntfy topic URL '{}': {}", topic_url, e)))?;

        // 2. Extract base URL and topic path, validating host presence (must not be None or empty)
        let host = match parsed_url.host_str() {
            Some(h) if !h.is_empty() => h, // Host is present and non-empty
            _ => return Err(CoreError::NotificationError(format!("URL '{}' must have a non-empty host", topic_url))), // Host is None or empty, return error
        };

        let base_url = format!("{}://{}", parsed_url.scheme(), host);

        let topic = parsed_url.path().trim_start_matches('/'); // ntfy crate expects topic without leading /
        if topic.is_empty() {
             // This should only be reached if the host was valid but the path is empty/root
             return Err(CoreError::NotificationError(format!("URL '{}' is missing topic path", topic_url)));
        }

        // 3. Build the dispatcher
        // TODO: Consider adding proxy/auth support here if needed later, potentially via config
        let dispatcher = DispatcherBuilder::new(&base_url) // Use DispatcherBuilder::new
             // .credentials(Auth::None) // Explicitly none for now
             // .proxy(None) // Explicitly none for now
            .build_blocking() // Use blocking build method
            .map_err(|e: NtfyError| CoreError::NotificationError(format!("Failed to build ntfy dispatcher for {}: {}", base_url, e)))?;

        // 4. Build the payload using chained builder methods
        let mut payload_builder = Payload::new(topic).message(message); // Start chain

        if let Some(t) = title {
            payload_builder = payload_builder.title(t); // Continue chain
        }

        if let Some(p_val) = priority {
            if let Some(ntfy_p) = map_priority(p_val) {
                payload_builder = payload_builder.priority(ntfy_p); // Continue chain
            } else {
                // Optional: Log a warning about invalid priority?
                log::warn!("Invalid ntfy priority value provided: {}", p_val);
            }
        }

        // Handle tags: Combine input tags with "drapto"
        let mut final_tags: Vec<String> = tags
            .unwrap_or("") // Handle None case
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();

        if !final_tags.iter().any(|t| t == "drapto") {
             final_tags.push("drapto".to_string());
        }
        // Only add tags if there are any to add
        if !final_tags.is_empty() {
             payload_builder = payload_builder.tags(final_tags); // Final chain step
        }

        // The final payload is the result of the chain
        let final_payload = payload_builder;

        // 5. Send the notification
        dispatcher.send(&final_payload) // Send the final payload
            .map_err(|e: NtfyError| CoreError::NotificationError(format!("Failed to send ntfy notification to {}: {}", topic_url, e)))
    }
}

// Removed deprecated send_ntfy function
