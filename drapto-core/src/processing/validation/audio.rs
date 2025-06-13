//! Audio codec validation

use ffprobe::FfProbe;

/// Validates that all audio streams use the Opus codec
pub fn validate_audio_codec(metadata: &FfProbe) -> (bool, Vec<String>, Option<String>) {
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

    let (is_audio_opus, audio_message) = if audio_streams.is_empty() {
        // No audio streams found - this is an error
        (false, Some("No audio streams found".to_string()))
    } else {
        let non_opus_codecs: Vec<&String> = audio_codecs
            .iter()
            .filter(|&codec| codec != "opus")
            .collect();

        if non_opus_codecs.is_empty() {
            let track_count = audio_streams.len();
            if track_count == 1 {
                (true, Some("Audio track is Opus".to_string()))
            } else {
                (true, Some(format!("All {} audio tracks are Opus", track_count)))
            }
        } else {
            let unique_codecs: std::collections::HashSet<&String> = non_opus_codecs.iter().copied().collect();
            let codec_list: Vec<&str> = unique_codecs.iter().map(|s| s.as_str()).collect();
            (false, Some(format!(
                "Expected Opus for all audio tracks, found: {}", 
                codec_list.join(", ")
            )))
        }
    };

    (is_audio_opus, audio_codecs, audio_message)
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
        // Test all Opus tracks
        let metadata = create_test_metadata(vec!["opus", "opus"]);
        let (is_valid, codecs, message) = validate_audio_codec(&metadata);
        assert!(is_valid);
        assert_eq!(codecs, vec!["opus", "opus"]);
        assert!(message.unwrap().contains("All 2 audio tracks are Opus"));

        // Test single Opus track
        let metadata = create_test_metadata(vec!["opus"]);
        let (is_valid, codecs, message) = validate_audio_codec(&metadata);
        assert!(is_valid);
        assert_eq!(codecs, vec!["opus"]);
        assert!(message.unwrap().contains("Audio track is Opus"));

        // Test mixed codecs
        let metadata = create_test_metadata(vec!["opus", "aac"]);
        let (is_valid, codecs, message) = validate_audio_codec(&metadata);
        assert!(!is_valid);
        assert_eq!(codecs, vec!["opus", "aac"]);
        assert!(message.unwrap().contains("Expected Opus for all audio tracks, found: aac"));

        // Test no audio streams
        let metadata = FfProbe {
            streams: vec![],
            format: Format::default(),
        };
        let (is_valid, codecs, message) = validate_audio_codec(&metadata);
        assert!(!is_valid);
        assert!(codecs.is_empty());
        assert!(message.unwrap().contains("No audio streams found"));
    }
}