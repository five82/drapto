//! Audio codec validation

use ffprobe::FfProbe;

/// Validates that all audio streams use the Opus codec and match expected track count
pub fn validate_audio_codec(
    metadata: &FfProbe, 
    expected_track_count: Option<usize>
) -> (bool, bool, Vec<String>, Option<String>) {
    // Find all audio streams
    let audio_streams: Vec<&ffprobe::Stream> = metadata
        .streams
        .iter()
        .filter(|s| s.codec_type.as_deref() == Some("audio"))
        .collect();

    let audio_codecs: Vec<String> = audio_streams
        .iter()
        .filter_map(|s| s.codec_name.clone())
        .collect();

    let actual_track_count = audio_streams.len();

    // Validate track count
    let is_track_count_correct = match expected_track_count {
        Some(expected) => actual_track_count == expected,
        None => true, // If no expectation, consider it correct
    };

    // Validate codec and generate message
    let (is_audio_opus, mut audio_message) = if audio_streams.is_empty() {
        // No audio streams found - this is an error
        (false, Some("No audio streams found".to_string()))
    } else {
        let non_opus_codecs: Vec<&String> = audio_codecs
            .iter()
            .filter(|&codec| codec != "opus")
            .collect();

        if non_opus_codecs.is_empty() {
            // Generate base message about codec
            let codec_msg = if actual_track_count == 1 {
                "Audio track is Opus".to_string()
            } else {
                format!("All {} audio tracks are Opus", actual_track_count)
            };
            
            // Add track count information if we have an expectation
            let full_msg = match expected_track_count {
                Some(expected) if actual_track_count == expected => {
                    format!("{} (track count correct: {})", codec_msg, actual_track_count)
                }
                Some(_) => codec_msg, // Track count mismatch will be handled below
                None => codec_msg, // No expectation, just report codec
            };
            
            (true, Some(full_msg))
        } else {
            let unique_codecs: std::collections::HashSet<&String> = non_opus_codecs.iter().copied().collect();
            let codec_list: Vec<&str> = unique_codecs.iter().map(|s| s.as_str()).collect();
            (false, Some(format!(
                "Expected Opus for all audio tracks, found: {}", 
                codec_list.join(", ")
            )))
        }
    };

    // Update message if track count is wrong
    if !is_track_count_correct {
        if let Some(expected) = expected_track_count {
            let track_count_msg = format!(
                "Expected {} audio tracks, found {}", 
                expected, actual_track_count
            );
            
            match audio_message {
                Some(existing_msg) => {
                    audio_message = Some(format!("{}; {}", existing_msg, track_count_msg));
                }
                None => {
                    audio_message = Some(track_count_msg);
                }
            }
        }
    }

    (is_audio_opus, is_track_count_correct, audio_codecs, audio_message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffprobe::{FfProbe, Stream, Format};

    fn create_test_metadata(audio_codecs: Vec<&str>) -> FfProbe {
        let streams: Vec<Stream> = audio_codecs
            .into_iter()
            .map(|codec| {
                let mut stream = Stream::default();
                stream.codec_type = Some("audio".to_string());
                stream.codec_name = Some(codec.to_string());
                stream
            })
            .collect();

        FfProbe {
            streams,
            format: Format::default(),
        }
    }

    #[test]
    fn test_audio_codec_validation() {
        // Test all Opus tracks with correct count
        let metadata = create_test_metadata(vec!["opus", "opus"]);
        let (is_opus_valid, is_count_valid, codecs, message) = validate_audio_codec(&metadata, Some(2));
        assert!(is_opus_valid);
        assert!(is_count_valid);
        assert_eq!(codecs, vec!["opus", "opus"]);
        let msg = message.unwrap();
        assert!(msg.contains("All 2 audio tracks are Opus"));
        assert!(msg.contains("track count correct: 2"));

        // Test single Opus track with correct count
        let metadata = create_test_metadata(vec!["opus"]);
        let (is_opus_valid, is_count_valid, codecs, message) = validate_audio_codec(&metadata, Some(1));
        assert!(is_opus_valid);
        assert!(is_count_valid);
        assert_eq!(codecs, vec!["opus"]);
        let msg = message.unwrap();
        assert!(msg.contains("Audio track is Opus"));
        assert!(msg.contains("track count correct: 1"));

        // Test mixed codecs
        let metadata = create_test_metadata(vec!["opus", "aac"]);
        let (is_opus_valid, is_count_valid, codecs, message) = validate_audio_codec(&metadata, Some(2));
        assert!(!is_opus_valid);
        assert!(is_count_valid); // Count is correct
        assert_eq!(codecs, vec!["opus", "aac"]);
        assert!(message.unwrap().contains("Expected Opus for all audio tracks, found: aac"));

        // Test wrong track count
        let metadata = create_test_metadata(vec!["opus", "opus"]);
        let (is_opus_valid, is_count_valid, codecs, message) = validate_audio_codec(&metadata, Some(1));
        assert!(is_opus_valid); // Codecs are correct
        assert!(!is_count_valid); // Count is wrong
        assert_eq!(codecs, vec!["opus", "opus"]);
        assert!(message.unwrap().contains("Expected 1 audio tracks, found 2"));

        // Test no audio streams
        let metadata = FfProbe {
            streams: vec![],
            format: Format::default(),
        };
        let (is_opus_valid, is_count_valid, codecs, message) = validate_audio_codec(&metadata, Some(1));
        assert!(!is_opus_valid);
        assert!(!is_count_valid);
        assert!(codecs.is_empty());
        assert!(message.unwrap().contains("No audio streams found"));

        // Test with no expected count (should pass count validation)
        let metadata = create_test_metadata(vec!["opus"]);
        let (is_opus_valid, is_count_valid, codecs, message) = validate_audio_codec(&metadata, None);
        assert!(is_opus_valid);
        assert!(is_count_valid);
        assert_eq!(codecs, vec!["opus"]);
        assert!(message.unwrap().contains("Audio track is Opus"));
    }
}