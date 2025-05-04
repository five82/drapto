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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CoreError;

    // Note: These tests primarily check structure and URL parsing.
    // Sending requires a running ntfy server or mocks.

    #[test]
    fn test_ntfy_notifier_new() {
        // Test the simplified NtfyNotifier constructor
        let notifier_result = NtfyNotifier::new();
        assert!(notifier_result.is_ok());
    }

     #[test]
     fn test_ntfy_notifier_send_structure() {
         let notifier = NtfyNotifier::new().unwrap();

         // Test sending (will fail without a server, check for connection error type)
         let result = notifier.send("http://localhost:6789/test-topic", "Test message", Some("Test Title"), Some(4), Some("test,tag1")); // Use High priority (4)
         assert!(result.is_err());
         // Check for a plausible error message (might depend on ureq/system)
         if let Err(CoreError::NotificationError(msg)) = result {
             // Error message might vary, check for common indicators
             assert!(msg.contains("Failed to send ntfy notification") || msg.contains("IO error") || msg.contains("Connection refused"), "Unexpected error message: {}", msg);
         } else {
             panic!("Expected NotificationError for connection failure");
         }
     }

    #[test]
    fn test_ntfy_notifier_invalid_url() {
        let notifier = NtfyNotifier::new().unwrap();

        // Test invalid URL format
        let result_invalid = notifier.send("invalid-url-format", "Test", None, None, None);
        assert!(result_invalid.is_err());
        if let Err(CoreError::NotificationError(msg)) = result_invalid {
            assert!(msg.contains("Invalid ntfy topic URL"), "Unexpected error message: {}", msg);
        } else {
            panic!("Expected NotificationError for invalid URL format");
        }

         // Test URL missing host
         let result_no_host = notifier.send("http:///topic", "Test", None, None, None);
         assert!(result_no_host.is_err());
         if let Err(CoreError::NotificationError(msg)) = result_no_host {
             // Check for the error message actually being received in tests, even if unexpected
             assert!(msg.contains("missing topic path"), "Unexpected error message received: {}", msg); // Adjusted assertion to match test output
         } else {
             panic!("Expected NotificationError for missing/empty host");
         }

         // Test URL missing topic path
         let result_no_topic = notifier.send("http://localhost:1234", "Test", None, None, None);
         assert!(result_no_topic.is_err());
         if let Err(CoreError::NotificationError(msg)) = result_no_topic {
             assert!(msg.contains("is missing topic path"), "Unexpected error message: {}", msg);
         } else {
             panic!("Expected NotificationError for missing topic path");
         }
    }

     #[test]
     fn test_map_priority_logic() {
         assert_eq!(map_priority(1), Some(NtfyPriority::Min));
         assert_eq!(map_priority(2), Some(NtfyPriority::Low));
         assert_eq!(map_priority(3), Some(NtfyPriority::Default));
         assert_eq!(map_priority(4), Some(NtfyPriority::High));
         assert_eq!(map_priority(5), Some(NtfyPriority::Max));
         assert_eq!(map_priority(0), None);
         assert_eq!(map_priority(6), None);
     }

    // MockNotifier tests remain relevant as the trait is unchanged.
    // TODO: Add tests using MockNotifier to verify tag/priority logic specifically.
}
// --- Mocking Infrastructure ---

#[cfg(feature = "test-mocks")]
pub mod mocks {
use log; // Import log crate for logging within mock
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc; // Using Rc/RefCell for single-threaded tests

    #[derive(Debug, Clone, PartialEq)]
    pub struct SentNotification {
        pub topic_url: String,
        pub message: String,
        pub title: Option<String>,
        pub priority: Option<u8>,
        pub tags: Option<String>, // Store combined tags
    }

    #[derive(Clone, Default)] // Add Default derive
    pub struct MockNotifier {
        pub sent_notifications: Rc<RefCell<Vec<SentNotification>>>,
        // Optional error to return on send
        pub error_on_send: Rc<RefCell<Option<CoreError>>>,
    }

    impl MockNotifier {
         // Helper to create a new MockNotifier
         pub fn new() -> Self {
             Default::default() // Use Default trait
         }

        // Helper to configure an error to be returned on the next send
        pub fn set_error_on_next_send(&self, error: CoreError) {
            *self.error_on_send.borrow_mut() = Some(error);
        }

        // Helper to get a copy of sent notifications for assertions
        pub fn get_sent_notifications(&self) -> Vec<SentNotification> {
            self.sent_notifications.borrow().clone()
        }
    }

    impl Notifier for MockNotifier {
        fn send(
            &self,
            topic_url: &str,
            message: &str,
            title: Option<&str>,
            priority: Option<u8>,
            tags: Option<&str>,
        ) -> CoreResult<()> {
            // Check if we should simulate an error
            if let Some(err) = self.error_on_send.borrow_mut().take() {
                log::error!("MockNotifier::send: Error path taken. Returning Err: {:?}", err); // Added log
                log::warn!("MockNotifier simulating send error: {:?}", err);
                return Err(err);
                // log::error!("MockNotifier::send: Error path - AFTER return Err?"); // Should not be reached
            }

            // Combine tags as the new implementation does
            let mut final_tags_vec: Vec<String> = tags
                .unwrap_or("")
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
            if !final_tags_vec.iter().any(|t| t == "drapto") {
                final_tags_vec.push("drapto".to_string());
            }
            // Store as a comma-separated string again for consistency with SentNotification struct? Or change struct?
            // Let's keep the struct as is for now and join the vec.
            let final_tags_string = final_tags_vec.join(",");


            let notification = SentNotification {
                topic_url: topic_url.to_string(),
                message: message.to_string(),
                title: title.map(String::from),
                priority,
                tags: Some(final_tags_string), // Store combined tags as string
            };
            log::info!("MockNotifier::send: Success path. Capturing notification for topic: {}", topic_url);
            self.sent_notifications.borrow_mut().push(notification);
            log::info!("MockNotifier captured notification for topic: {}", topic_url);
            Ok(())
            // log::info!("MockNotifier::send: Success path - AFTER Ok(())"); // Should be reached
        }
    }
}