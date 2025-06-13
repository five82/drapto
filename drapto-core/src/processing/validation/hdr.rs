//! HDR/SDR validation

use ffprobe::Stream;

/// Validates HDR status between input and output
pub fn validate_hdr_status(
    stream: &Stream,
    expected_hdr: Option<bool>
) -> (bool, Option<bool>, Option<String>) {
    // Detect HDR status from color metadata
    let actual_hdr = detect_hdr_from_stream(stream);

    // Validate HDR status
    let (is_hdr_correct, hdr_message) = match (expected_hdr, actual_hdr) {
        (Some(expected), Some(actual)) => {
            if expected == actual {
                let status = if actual { "HDR" } else { "SDR" };
                (true, Some(format!("{} preserved", status)))
            } else {
                let expected_str = if expected { "HDR" } else { "SDR" };
                let actual_str = if actual { "HDR" } else { "SDR" };
                (false, Some(format!("Expected {}, found {}", expected_str, actual_str)))
            }
        }
        (None, Some(actual)) => {
            let status = if actual { "HDR" } else { "SDR" };
            (true, Some(format!("Output is {}", status)))
        }
        (Some(expected), None) => {
            let expected_str = if expected { "HDR" } else { "SDR" };
            (false, Some(format!("Expected {}, but could not detect HDR status", expected_str)))
        }
        (None, None) => {
            (false, Some("Could not detect HDR status".to_string()))
        }
    };

    (is_hdr_correct, actual_hdr, hdr_message)
}

/// Detect HDR status from video stream metadata
fn detect_hdr_from_stream(stream: &Stream) -> Option<bool> {
    // Use the same HDR detection logic as the rest of the codebase
    if let Some(color_space) = &stream.color_space {
        // Check against the HDR color spaces defined in config
        let is_hdr = crate::config::HDR_COLOR_SPACES.contains(&color_space.as_str());
        return Some(is_hdr);
    }
    
    // If we can't determine, return None
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hdr_detection() {
        // Test stream with HDR color space
        let mut stream = ffprobe::Stream::default();
        stream.color_space = Some("bt2020nc".to_string());
        assert_eq!(detect_hdr_from_stream(&stream), Some(true));
        
        stream.color_space = Some("bt2020c".to_string());
        assert_eq!(detect_hdr_from_stream(&stream), Some(true));
        
        // Test stream with SDR color space
        stream.color_space = Some("bt709".to_string());
        assert_eq!(detect_hdr_from_stream(&stream), Some(false));
        
        // Test stream with no color space info
        stream.color_space = None;
        assert_eq!(detect_hdr_from_stream(&stream), None);
        
        // Test unknown color space
        stream.color_space = Some("unknown".to_string());
        assert_eq!(detect_hdr_from_stream(&stream), Some(false));
    }

    #[test]
    fn test_hdr_validation() {
        // Test HDR preserved correctly
        let mut stream = ffprobe::Stream::default();
        stream.color_space = Some("bt2020nc".to_string());
        
        let (is_valid, actual_hdr, message) = validate_hdr_status(&stream, Some(true));
        assert!(is_valid);
        assert_eq!(actual_hdr, Some(true));
        assert!(message.unwrap().contains("HDR preserved"));

        // Test HDR mismatch (expected HDR, got SDR)
        stream.color_space = Some("bt709".to_string());
        let (is_valid, actual_hdr, message) = validate_hdr_status(&stream, Some(true));
        assert!(!is_valid);
        assert_eq!(actual_hdr, Some(false));
        assert!(message.unwrap().contains("Expected HDR, found SDR"));

        // Test SDR preserved correctly
        let (is_valid, actual_hdr, message) = validate_hdr_status(&stream, Some(false));
        assert!(is_valid);
        assert_eq!(actual_hdr, Some(false));
        assert!(message.unwrap().contains("SDR preserved"));
    }
}