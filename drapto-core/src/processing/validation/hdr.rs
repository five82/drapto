//! HDR/SDR validation

use crate::external::{detect_hdr_from_mediainfo, get_mediainfo_data, is_mediainfo_available};
use std::path::Path;

/// Validates HDR status between input and output using MediaInfo
pub fn validate_hdr_status_with_path(
    output_path: &Path,
    expected_hdr: Option<bool>,
) -> (bool, Option<bool>, Option<String>) {
    validate_hdr_status_with_availability_check(output_path, expected_hdr, is_mediainfo_available())
}

/// Internal HDR validation function that accepts MediaInfo availability as parameter
/// This allows for easier testing without depending on actual system MediaInfo installation
fn validate_hdr_status_with_availability_check(
    output_path: &Path,
    expected_hdr: Option<bool>,
    mediainfo_available: bool,
) -> (bool, Option<bool>, Option<String>) {
    // Check if MediaInfo is available first
    if !mediainfo_available {
        log::warn!("MediaInfo not available for HDR validation");
        return (
            true,
            None,
            Some("MediaInfo not installed - HDR validation skipped".to_string()),
        );
    }

    // Use MediaInfo for HDR detection
    let actual_hdr = match get_mediainfo_data(output_path) {
        Ok(media_info) => {
            let hdr_info = detect_hdr_from_mediainfo(&media_info);
            Some(hdr_info.is_hdr)
        }
        Err(e) => {
            log::warn!("Failed to get MediaInfo for HDR validation: {}", e);
            None
        }
    };

    validate_hdr_result(expected_hdr, actual_hdr)
}

/// Common HDR validation logic
fn validate_hdr_result(
    expected_hdr: Option<bool>,
    actual_hdr: Option<bool>,
) -> (bool, Option<bool>, Option<String>) {
    // Validate HDR status
    let (is_hdr_correct, hdr_message) = match (expected_hdr, actual_hdr) {
        (Some(expected), Some(actual)) => {
            if expected == actual {
                let status = if actual { "HDR" } else { "SDR" };
                (true, Some(format!("{} preserved", status)))
            } else {
                let expected_str = if expected { "HDR" } else { "SDR" };
                let actual_str = if actual { "HDR" } else { "SDR" };
                (
                    false,
                    Some(format!("Expected {}, found {}", expected_str, actual_str)),
                )
            }
        }
        (None, Some(actual)) => {
            let status = if actual { "HDR" } else { "SDR" };
            (true, Some(format!("Output is {}", status)))
        }
        (Some(expected), None) => {
            let expected_str = if expected { "HDR" } else { "SDR" };
            (
                false,
                Some(format!(
                    "Expected {}, but could not detect HDR status",
                    expected_str
                )),
            )
        }
        (None, None) => (false, Some("Could not detect HDR status".to_string())),
    };

    (is_hdr_correct, actual_hdr, hdr_message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hdr_validation_result() {
        // Test HDR preserved correctly
        let (is_valid, actual_hdr, message) = validate_hdr_result(Some(true), Some(true));
        assert!(is_valid);
        assert_eq!(actual_hdr, Some(true));
        assert!(message.unwrap().contains("HDR preserved"));

        // Test SDR preserved correctly
        let (is_valid, actual_hdr, message) = validate_hdr_result(Some(false), Some(false));
        assert!(is_valid);
        assert_eq!(actual_hdr, Some(false));
        assert!(message.unwrap().contains("SDR preserved"));

        // Test HDR mismatch (expected HDR, got SDR)
        let (is_valid, actual_hdr, message) = validate_hdr_result(Some(true), Some(false));
        assert!(!is_valid);
        assert_eq!(actual_hdr, Some(false));
        assert!(message.unwrap().contains("Expected HDR, found SDR"));

        // Test SDR mismatch (expected SDR, got HDR)
        let (is_valid, actual_hdr, message) = validate_hdr_result(Some(false), Some(true));
        assert!(!is_valid);
        assert_eq!(actual_hdr, Some(true));
        assert!(message.unwrap().contains("Expected SDR, found HDR"));

        // Test when expected is None but actual is detected
        let (is_valid, actual_hdr, message) = validate_hdr_result(None, Some(true));
        assert!(is_valid);
        assert_eq!(actual_hdr, Some(true));
        assert!(message.unwrap().contains("Output is HDR"));

        // Test when expected is set but actual couldn't be detected
        let (is_valid, actual_hdr, message) = validate_hdr_result(Some(true), None);
        assert!(!is_valid);
        assert_eq!(actual_hdr, None);
        assert!(
            message
                .unwrap()
                .contains("Expected HDR, but could not detect HDR status")
        );

        // Test when neither expected nor actual are available
        let (is_valid, actual_hdr, message) = validate_hdr_result(None, None);
        assert!(!is_valid);
        assert_eq!(actual_hdr, None);
        assert!(message.unwrap().contains("Could not detect HDR status"));
    }

    #[test]
    fn test_mediainfo_not_available() {
        // Test the MediaInfo availability check with mocked unavailability
        use std::path::PathBuf;

        let dummy_path = PathBuf::from("/nonexistent/file.mkv");
        // Use the internal function with mediainfo_available = false
        let (is_valid, actual_hdr, message) =
            validate_hdr_status_with_availability_check(&dummy_path, Some(false), false);

        // When MediaInfo is not available, validation should pass with a skip message
        assert!(is_valid);
        assert_eq!(actual_hdr, None);
        assert!(
            message
                .unwrap()
                .contains("MediaInfo not installed - HDR validation skipped")
        );
    }

    #[test]
    fn test_mediainfo_available_but_file_not_found() {
        // Test behavior when MediaInfo is available but the file doesn't exist
        use std::path::PathBuf;

        let dummy_path = PathBuf::from("/nonexistent/file.mkv");
        // Use the internal function with mediainfo_available = true
        let (is_valid, actual_hdr, message) =
            validate_hdr_status_with_availability_check(&dummy_path, Some(false), true);

        // When MediaInfo is available but file access fails, validation should fail
        assert!(!is_valid);
        assert_eq!(actual_hdr, None);
        assert!(
            message
                .unwrap()
                .contains("Expected SDR, but could not detect HDR status")
        );
    }
}
