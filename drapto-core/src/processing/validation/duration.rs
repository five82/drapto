//! Video duration validation

use ffprobe::{FfProbe, Stream};

/// Validates that the video duration matches the expected duration
pub fn validate_duration(
    metadata: &FfProbe,
    video_stream: &Stream,
    expected_duration: Option<f64>
) -> (bool, Option<f64>, Option<String>) {
    // Get actual video duration - try multiple sources
    let actual_duration = get_duration_from_metadata(metadata, video_stream);

    // Validate duration (allow for small encoding differences - within 1 second tolerance)
    let duration_tolerance = 1.0; // seconds
    let (is_duration_correct, duration_message) = match (expected_duration, actual_duration) {
        (Some(expected), Some(actual)) => {
            let duration_diff = (expected - actual).abs();
            if duration_diff <= duration_tolerance {
                (true, Some(format!(
                    "Duration matches input ({:.1}s)", 
                    actual
                )))
            } else {
                (false, Some(format!(
                    "Expected {:.1}s, found {:.1}s (diff: {:.1}s)", 
                    expected, actual, duration_diff
                )))
            }
        }
        (None, Some(actual)) => {
            // No expected duration provided, just report what we found
            (true, Some(format!("Duration: {:.1}s", actual)))
        }
        (Some(expected), None) => {
            (false, Some(format!(
                "Expected duration {:.1}s, but could not read actual duration", 
                expected
            )))
        }
        (None, None) => {
            (false, Some("Could not read video duration".to_string()))
        }
    };

    (is_duration_correct, actual_duration, duration_message)
}

/// Extract duration from metadata using multiple methods
fn get_duration_from_metadata(metadata: &FfProbe, video_stream: &Stream) -> Option<f64> {
    // Method 1: Try video stream duration
    if let Some(duration_str) = &video_stream.duration {
        if let Ok(duration) = duration_str.parse::<f64>() {
            if duration > 0.0 {
                log::debug!("Duration from video stream: {}", duration);
                return Some(duration);
            }
        }
    }

    // Method 2: Try format duration
    if let Some(duration_str) = &metadata.format.duration {
        if let Ok(duration) = duration_str.parse::<f64>() {
            if duration > 0.0 {
                log::debug!("Duration from format: {}", duration);
                return Some(duration);
            }
        }
    }

    // Method 3: Calculate from video stream if we have frame count and frame rate
    if let (Some(nb_frames_str), r_frame_rate_str) = (&video_stream.nb_frames, &video_stream.r_frame_rate) {
        if let (Ok(nb_frames), Ok(frame_rate)) = (nb_frames_str.parse::<u64>(), parse_frame_rate(r_frame_rate_str)) {
            if frame_rate > 0.0 && nb_frames > 0 {
                let duration = nb_frames as f64 / frame_rate;
                log::debug!("Duration calculated from frames: {} frames / {} fps = {} seconds", nb_frames, frame_rate, duration);
                return Some(duration);
            }
        }
    }

    log::debug!("Could not determine duration from any source");
    None
}

/// Parse frame rate string (e.g., "30000/1001" or "30.0")
fn parse_frame_rate(frame_rate_str: &str) -> Result<f64, std::num::ParseFloatError> {
    if frame_rate_str.contains('/') {
        let parts: Vec<&str> = frame_rate_str.split('/').collect();
        if parts.len() == 2 {
            let numerator: f64 = parts[0].parse()?;
            let denominator: f64 = parts[1].parse()?;
            if denominator != 0.0 {
                return Ok(numerator / denominator);
            }
        }
    }
    frame_rate_str.parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffprobe::{FfProbe, Format};

    #[test]
    fn test_frame_rate_parsing() {
        assert_eq!(parse_frame_rate("30").unwrap(), 30.0);
        assert_eq!(parse_frame_rate("29.97").unwrap(), 29.97);
        assert_eq!(parse_frame_rate("30000/1001").unwrap(), 30000.0 / 1001.0);
        assert_eq!(parse_frame_rate("25/1").unwrap(), 25.0);
        assert!(parse_frame_rate("invalid").is_err());
        assert!(parse_frame_rate("30/0").is_err()); // Division by zero should be handled by parse
    }

    #[test]
    fn test_duration_validation() {
        // Test successful duration validation
        let mut stream = ffprobe::Stream::default();
        stream.duration = Some("6155.0".to_string());
        
        let metadata = FfProbe {
            streams: vec![stream.clone()],
            format: Format::default(),
        };

        let (is_valid, actual_duration, message) = validate_duration(&metadata, &stream, Some(6155.0));
        assert!(is_valid);
        assert_eq!(actual_duration, Some(6155.0));
        assert!(message.unwrap().contains("Duration matches input (6155.0s)"));

        // Test failed duration validation
        let (is_valid, actual_duration, message) = validate_duration(&metadata, &stream, Some(6200.0));
        assert!(!is_valid);
        assert_eq!(actual_duration, Some(6155.0));
        assert!(message.unwrap().contains("Expected 6200.0s, found 6155.0s"));
    }
}