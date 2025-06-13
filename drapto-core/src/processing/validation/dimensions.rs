//! Video dimensions and crop validation

use ffprobe::Stream;

/// Validates the video dimensions against expected crop dimensions
pub fn validate_dimensions(
    stream: &Stream,
    expected_dimensions: Option<(u32, u32)>
) -> (bool, Option<(u32, u32)>, Option<String>) {
    // Get actual video dimensions
    let actual_width = stream.width;
    let actual_height = stream.height;
    let actual_dimensions = match (actual_width, actual_height) {
        (Some(w), Some(h)) => Some((w as u32, h as u32)),
        _ => None,
    };

    // Validate crop dimensions
    let (is_crop_correct, crop_message) = match (expected_dimensions, actual_dimensions) {
        (Some((expected_w, expected_h)), Some((actual_w, actual_h))) => {
            if expected_w == actual_w && expected_h == actual_h {
                (true, Some(format!("Crop applied correctly ({}x{})", actual_w, actual_h)))
            } else {
                (false, Some(format!(
                    "Expected {}x{}, found {}x{}", 
                    expected_w, expected_h, actual_w, actual_h
                )))
            }
        }
        (None, Some((actual_w, actual_h))) => {
            // No crop expected, check if video was cropped anyway
            (true, Some(format!("No crop expected, dimensions: {}x{}", actual_w, actual_h)))
        }
        (Some((expected_w, expected_h)), None) => {
            (false, Some(format!(
                "Expected dimensions {}x{}, but could not read actual dimensions", 
                expected_w, expected_h
            )))
        }
        (None, None) => {
            (false, Some("Could not read video dimensions".to_string()))
        }
    };

    (is_crop_correct, actual_dimensions, crop_message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimensions_validation() {
        // Test successful crop validation
        let mut stream = ffprobe::Stream::default();
        stream.width = Some(1856);
        stream.height = Some(1044);
        
        let (is_valid, actual_dims, message) = validate_dimensions(&stream, Some((1856, 1044)));
        assert!(is_valid);
        assert_eq!(actual_dims, Some((1856, 1044)));
        assert!(message.unwrap().contains("Crop applied correctly (1856x1044)"));

        // Test failed crop validation
        let (is_valid, actual_dims, message) = validate_dimensions(&stream, Some((1920, 1080)));
        assert!(!is_valid);
        assert_eq!(actual_dims, Some((1856, 1044)));
        assert!(message.unwrap().contains("Expected 1920x1080, found 1856x1044"));

        // Test no crop expected
        let (is_valid, actual_dims, message) = validate_dimensions(&stream, None);
        assert!(is_valid);
        assert_eq!(actual_dims, Some((1856, 1044)));
        assert!(message.unwrap().contains("No crop expected, dimensions: 1856x1044"));
    }
}