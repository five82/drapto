// drapto-core/src/notifications.rs
//
// Module for handling ntfy notifications.

use crate::error::{CoreError, CoreResult};
use reqwest::blocking::Client;
use std::time::Duration;

const NTFY_TIMEOUT_SECS: u64 = 10;

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

/// Implementation of `Notifier` using reqwest to send to ntfy.sh.
pub struct NtfyNotifier {
    client: Client,
}

impl NtfyNotifier {
    /// Creates a new NtfyNotifier with a default reqwest client.
    pub fn new() -> CoreResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(NTFY_TIMEOUT_SECS))
            .build()
            .map_err(|e| CoreError::NotificationError(format!("Failed to build HTTP client: {}", e)))?;
        Ok(Self { client })
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
        // Basic validation of the topic URL
        if !topic_url.starts_with("http://") && !topic_url.starts_with("https://") {
            return Err(CoreError::NotificationError(format!(
                "Invalid ntfy topic URL scheme: {}",
                topic_url
            )));
        }

        let mut request_builder = self.client.post(topic_url).body(message.to_string());

        if let Some(t) = title {
            request_builder = request_builder.header("Title", t);
        }
        if let Some(p) = priority {
            request_builder = request_builder.header("Priority", p.to_string());
        }
        // Combine provided tags with the default 'drapto' tag
        let final_tags = match tags {
            Some(tg) => format!("{},drapto", tg),
            None => "drapto".to_string(),
        };
        request_builder = request_builder.header("Tags", final_tags);


        match request_builder.send() {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(())
                } else {
                    let status = response.status();
                    let error_body = response.text().unwrap_or_else(|_| "Could not read error body".to_string());
                     Err(CoreError::NotificationError(format!(
                        "ntfy server returned error {}: {}",
                        status, error_body
                    )))
                }
            }
            Err(e) => Err(CoreError::NotificationError(format!(
                "Failed to send ntfy request to {}: {}",
                topic_url, e
            ))),
        }
    }
}


/// (Deprecated) Sends a notification message to an ntfy topic URL. Use NtfyNotifier instead.
///
/// # Arguments
///
/// * `topic_url` - The full URL of the ntfy topic (e.g., "https://ntfy.sh/your_topic").
/// * `message` - The main body of the notification message.
/// * `title` - An optional title for the notification.
/// * `priority` - An optional priority level (1-5, 5 being highest).
/// * `tags` - Optional comma-separated string of tags (e.g., "warning,skull").
///
/// # Returns
///
/// * `Ok(())` if the notification was sent successfully (HTTP 2xx status).
/// * `Err(CoreError)` if there was an error building the client, sending the request,
///   or if the server returned a non-success status code.
pub fn send_ntfy(
    topic_url: &str,
    message: &str,
    title: Option<&str>,
    priority: Option<u8>,
    tags: Option<&str>,
) -> CoreResult<()> {
    // Basic validation of the topic URL
    if !topic_url.starts_with("http://") && !topic_url.starts_with("https://") {
        return Err(CoreError::NotificationError(format!(
            "Invalid ntfy topic URL scheme: {}",
            topic_url
        )));
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(NTFY_TIMEOUT_SECS))
        .build()
        .map_err(|e| CoreError::NotificationError(format!("Failed to build HTTP client: {}", e)))?;

    let mut request_builder = client.post(topic_url).body(message.to_string());

    if let Some(t) = title {
        request_builder = request_builder.header("Title", t);
    }
    if let Some(p) = priority {
        request_builder = request_builder.header("Priority", p.to_string());
    }
    if let Some(tg) = tags {
        request_builder = request_builder.header("Tags", tg);
    }

    // Add a default 'drapto' tag to all notifications for easier filtering.
    request_builder = request_builder.header("Tags", "drapto");


    match request_builder.send() {
        Ok(response) => {
            if response.status().is_success() {
                Ok(())
            } else {
                let status = response.status();
                let error_body = response.text().unwrap_or_else(|_| "Could not read error body".to_string());
                 Err(CoreError::NotificationError(format!(
                    "ntfy server returned error {}: {}",
                    status, error_body
                )))
            }
        }
        Err(e) => Err(CoreError::NotificationError(format!(
            "Failed to send ntfy request to {}: {}",
            topic_url, e
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*; // Import new items
    use crate::error::CoreError;

    // Note: These tests require a running ntfy server accessible at the specified URL
    // or a mock HTTP server. For simplicity, we'll just test the function structure here.
    // Real integration tests would be needed for full validation.

    // Basic test placeholder - does not actually send a request
    #[test]
    fn test_ntfy_notifier_structure() { // Renamed test
        // Test the NtfyNotifier implementation
        let notifier = NtfyNotifier::new().expect("Failed to create notifier");

        // Test sending (will likely fail without a server)
        let result = notifier.send("http://localhost:8080/test", "Test message", Some("Test Title"), Some(3), Some("test,tag"));
        // We expect an error because no server is running at localhost:8080 usually
        assert!(result.is_err());

        // Test invalid URL scheme
        let result_invalid_url = notifier.send("invalid-url", "Test", None, None, None);
        assert!(result_invalid_url.is_err());
        if let Err(CoreError::NotificationError(msg)) = result_invalid_url {
             assert!(msg.contains("Invalid ntfy topic URL scheme"));
        } else {
            panic!("Expected NotificationError for invalid URL scheme");
        }
    }

    // TODO: Add tests using MockNotifier once implemented
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

            // Combine tags as the real implementation does
            let final_tags = match tags {
                Some(tg) => format!("{},drapto", tg),
                None => "drapto".to_string(),
            };

            let notification = SentNotification {
                topic_url: topic_url.to_string(),
                message: message.to_string(),
                title: title.map(String::from),
                priority,
                tags: Some(final_tags), // Store combined tags
            };
            log::info!("MockNotifier::send: Success path. Capturing notification for topic: {}", topic_url); // Added log
            self.sent_notifications.borrow_mut().push(notification);
            log::info!("MockNotifier captured notification for topic: {}", topic_url);
            Ok(())
            // log::info!("MockNotifier::send: Success path - AFTER Ok(())"); // Should be reached
        }
    }
}