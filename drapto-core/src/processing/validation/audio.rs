//! Audio codec validation

use ffprobe::FfProbe;

/// Validates audio codec expectations based on spatial audio detection
/// - For spatial audio (Atmos/DTS:X): Expects preservation (TrueHD/DTS)
/// - For non-spatial audio: Expects Opus transcoding
/// - For mixed content: Validates each stream according to its spatial status
pub fn validate_audio_codec(
    metadata: &FfProbe, 
    expected_track_count: Option<usize>,
    spatial_audio_streams: Option<&[bool]>
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

    // Validate codec based on spatial audio expectations
    let (is_audio_correct, mut audio_message) = if audio_streams.is_empty() {
        // No audio streams found - this is an error
        (false, Some("No audio streams found".to_string()))
    } else if let Some(spatial_flags) = spatial_audio_streams {
        // We have spatial audio information - validate accordingly
        validate_with_spatial_info(&audio_codecs, spatial_flags, actual_track_count)
    } else {
        // No spatial audio information - assume all should be Opus (legacy behavior)
        validate_without_spatial_info(&audio_codecs, actual_track_count)
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

    (is_audio_correct, is_track_count_correct, audio_codecs, audio_message)
}

/// Validates audio codecs when spatial audio information is available
fn validate_with_spatial_info(
    audio_codecs: &[String], 
    spatial_flags: &[bool], 
    _actual_track_count: usize
) -> (bool, Option<String>) {
    if audio_codecs.len() != spatial_flags.len() {
        return (false, Some("Mismatch between detected audio streams and spatial audio flags".to_string()));
    }

    let mut errors = Vec::new();
    let mut preserved_count = 0;
    let mut opus_count = 0;

    // Validate each stream according to its spatial status
    for (i, (codec, &is_spatial)) in audio_codecs.iter().zip(spatial_flags.iter()).enumerate() {
        if is_spatial {
            // Spatial audio should be preserved (TrueHD, DTS, etc.)
            if !is_preserved_codec(codec) {
                errors.push(format!("Stream {}: Expected preserved spatial audio (TrueHD/DTS), found {}", i, codec));
            } else {
                preserved_count += 1;
            }
        } else {
            // Non-spatial audio should be Opus
            if codec != "opus" {
                errors.push(format!("Stream {}: Expected Opus for non-spatial audio, found {}", i, codec));
            } else {
                opus_count += 1;
            }
        }
    }

    // Generate success or failure message
    if errors.is_empty() {
        let message = match (preserved_count, opus_count) {
            (0, opus) if opus > 0 => {
                if opus == 1 {
                    "Audio track is Opus".to_string()
                } else {
                    format!("All {} audio tracks are Opus", opus)
                }
            }
            (preserved, 0) if preserved > 0 => {
                if preserved == 1 {
                    "Spatial audio preserved".to_string()
                } else {
                    format!("All {} spatial audio tracks preserved", preserved)
                }
            }
            (preserved, opus) => {
                format!("{} spatial audio tracks preserved, {} Opus tracks", preserved, opus)
            }
        };
        (true, Some(message))
    } else {
        (false, Some(errors.join("; ")))
    }
}

/// Validates audio codecs without spatial audio information (legacy behavior)
fn validate_without_spatial_info(audio_codecs: &[String], actual_track_count: usize) -> (bool, Option<String>) {
    let non_opus_codecs: Vec<&String> = audio_codecs
        .iter()
        .filter(|&codec| codec != "opus")
        .collect();

    if non_opus_codecs.is_empty() {
        // All Opus - generate success message
        let codec_msg = if actual_track_count == 1 {
            "Audio track is Opus".to_string()
        } else {
            format!("All {} audio tracks are Opus", actual_track_count)
        };
        (true, Some(codec_msg))
    } else {
        // Some non-Opus codecs found
        let unique_codecs: std::collections::HashSet<&String> = non_opus_codecs.iter().copied().collect();
        let codec_list: Vec<&str> = unique_codecs.iter().map(|s| s.as_str()).collect();
        (false, Some(format!(
            "Expected Opus for all audio tracks, found: {}", 
            codec_list.join(", ")
        )))
    }
}

/// Checks if a codec is a preserved spatial audio codec
fn is_preserved_codec(codec: &str) -> bool {
    matches!(codec.to_lowercase().as_str(), "truehd" | "dts" | "dtshd")
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
        let (is_opus_valid, is_count_valid, codecs, message) = validate_audio_codec(&metadata, Some(2), None);
        assert!(is_opus_valid);
        assert!(is_count_valid);
        assert_eq!(codecs, vec!["opus", "opus"]);
        let msg = message.unwrap();
        assert!(msg.contains("All 2 audio tracks are Opus"));

        // Test single Opus track with correct count
        let metadata = create_test_metadata(vec!["opus"]);
        let (is_opus_valid, is_count_valid, codecs, message) = validate_audio_codec(&metadata, Some(1), None);
        assert!(is_opus_valid);
        assert!(is_count_valid);
        assert_eq!(codecs, vec!["opus"]);
        let msg = message.unwrap();
        assert!(msg.contains("Audio track is Opus"));

        // Test mixed codecs
        let metadata = create_test_metadata(vec!["opus", "aac"]);
        let (is_opus_valid, is_count_valid, codecs, message) = validate_audio_codec(&metadata, Some(2), None);
        assert!(!is_opus_valid);
        assert!(is_count_valid); // Count is correct
        assert_eq!(codecs, vec!["opus", "aac"]);
        assert!(message.unwrap().contains("Expected Opus for all audio tracks, found: aac"));

        // Test wrong track count
        let metadata = create_test_metadata(vec!["opus", "opus"]);
        let (is_opus_valid, is_count_valid, codecs, message) = validate_audio_codec(&metadata, Some(1), None);
        assert!(is_opus_valid); // Codecs are correct
        assert!(!is_count_valid); // Count is wrong
        assert_eq!(codecs, vec!["opus", "opus"]);
        assert!(message.unwrap().contains("Expected 1 audio tracks, found 2"));

        // Test no audio streams
        let metadata = FfProbe {
            streams: vec![],
            format: Format::default(),
        };
        let (is_opus_valid, is_count_valid, codecs, message) = validate_audio_codec(&metadata, Some(1), None);
        assert!(!is_opus_valid);
        assert!(!is_count_valid);
        assert!(codecs.is_empty());
        assert!(message.unwrap().contains("No audio streams found"));

        // Test with no expected count (should pass count validation)
        let metadata = create_test_metadata(vec!["opus"]);
        let (is_opus_valid, is_count_valid, codecs, message) = validate_audio_codec(&metadata, None, None);
        assert!(is_opus_valid);
        assert!(is_count_valid);
        assert_eq!(codecs, vec!["opus"]);
        assert!(message.unwrap().contains("Audio track is Opus"));
    }

    #[test]
    fn test_spatial_audio_validation() {
        // Test single spatial audio (TrueHD) - should pass
        let metadata = create_test_metadata(vec!["truehd"]);
        let spatial_flags = &[true];
        let (is_valid, is_count_valid, codecs, message) = validate_audio_codec(&metadata, Some(1), Some(spatial_flags));
        assert!(is_valid, "Spatial TrueHD should be valid");
        assert!(is_count_valid);
        assert_eq!(codecs, vec!["truehd"]);
        assert!(message.unwrap().contains("Spatial audio preserved"));

        // Test single non-spatial audio with Opus - should pass
        let metadata = create_test_metadata(vec!["opus"]);
        let spatial_flags = &[false];
        let (is_valid, is_count_valid, codecs, message) = validate_audio_codec(&metadata, Some(1), Some(spatial_flags));
        assert!(is_valid, "Non-spatial Opus should be valid");
        assert!(is_count_valid);
        assert_eq!(codecs, vec!["opus"]);
        assert!(message.unwrap().contains("Audio track is Opus"));

        // Test mixed streams (spatial TrueHD + non-spatial Opus) - should pass
        let metadata = create_test_metadata(vec!["truehd", "opus"]);
        let spatial_flags = &[true, false];
        let (is_valid, is_count_valid, codecs, message) = validate_audio_codec(&metadata, Some(2), Some(spatial_flags));
        assert!(is_valid, "Mixed spatial and non-spatial should be valid");
        assert!(is_count_valid);
        assert_eq!(codecs, vec!["truehd", "opus"]);
        assert!(message.unwrap().contains("1 spatial audio tracks preserved, 1 Opus tracks"));

        // Test spatial audio with wrong codec (should be TrueHD but found AAC) - should fail
        let metadata = create_test_metadata(vec!["aac"]);
        let spatial_flags = &[true];
        let (is_valid, is_count_valid, codecs, message) = validate_audio_codec(&metadata, Some(1), Some(spatial_flags));
        assert!(!is_valid, "Spatial audio with wrong codec should be invalid");
        assert!(is_count_valid);
        assert_eq!(codecs, vec!["aac"]);
        assert!(message.unwrap().contains("Expected preserved spatial audio (TrueHD/DTS), found aac"));

        // Test non-spatial audio with wrong codec (should be Opus but found AAC) - should fail
        let metadata = create_test_metadata(vec!["aac"]);
        let spatial_flags = &[false];
        let (is_valid, is_count_valid, codecs, message) = validate_audio_codec(&metadata, Some(1), Some(spatial_flags));
        assert!(!is_valid, "Non-spatial audio with wrong codec should be invalid");
        assert!(is_count_valid);
        assert_eq!(codecs, vec!["aac"]);
        assert!(message.unwrap().contains("Expected Opus for non-spatial audio, found aac"));

        // Test multiple spatial tracks - should pass
        let metadata = create_test_metadata(vec!["truehd", "dts"]);
        let spatial_flags = &[true, true];
        let (is_valid, is_count_valid, codecs, message) = validate_audio_codec(&metadata, Some(2), Some(spatial_flags));
        assert!(is_valid, "Multiple spatial tracks should be valid");
        assert!(is_count_valid);
        assert_eq!(codecs, vec!["truehd", "dts"]);
        assert!(message.unwrap().contains("All 2 spatial audio tracks preserved"));
    }
}