// drapto-core/src/notifications.rs
//
// Module for handling ntfy notifications.

use crate::error::{CoreError, CoreResult};
use reqwest::blocking::Client;
use std::time::Duration;

const NTFY_TIMEOUT_SECS: u64 = 10;

/// Sends a notification message to an ntfy topic URL.
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
    use crate::error::CoreError;

    // Note: These tests require a running ntfy server accessible at the specified URL
    // or a mock HTTP server. For simplicity, we'll just test the function structure here.
    // Real integration tests would be needed for full validation.

    // Basic test placeholder - does not actually send a request
    #[test]
    fn test_send_ntfy_structure() {
        // This test only checks if the function can be called without panicking
        // It doesn't verify the actual HTTP request sending.
        let result = super::send_ntfy("http://localhost:8080/test", "Test message", Some("Test Title"), Some(3), Some("test,tag"));
        // We expect an error because no server is running at localhost:8080 usually
        assert!(result.is_err());

        let result_invalid_url = super::send_ntfy("invalid-url", "Test", None, None, None);
        assert!(result_invalid_url.is_err());
        if let Err(CoreError::NotificationError(msg)) = result_invalid_url {
             assert!(msg.contains("Invalid ntfy topic URL scheme"));
        } else {
            panic!("Expected NotificationError for invalid URL scheme");
        }
    }
}