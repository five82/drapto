//! Post-encode validation functions
//!
//! This module provides functions to validate the output of video encoding operations.
//! It verifies that the encoded video meets the expected specifications.
//!
//! # Organization
//!
//! The validation system is split into focused modules:
//! - `result`: Validation result structure and display logic
//! - `video`: Video codec and bit depth validation
//! - `audio`: Audio codec validation  
//! - `dimensions`: Video dimensions and crop validation
//! - `duration`: Video duration validation
//! - `hdr`: HDR/SDR status validation
//! - `validate`: Main validation orchestration

pub use self::result::ValidationResult;
pub use self::validate::validate_output_video;

mod result;
mod validate;
mod video;
mod audio;
mod dimensions;
mod duration;
mod hdr;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_integration() {
        // Test that all modules are properly integrated
        // This is a basic smoke test to ensure modules compile together
        
        let result = ValidationResult {
            is_av1: true,
            is_10_bit: true,
            is_crop_correct: true,
            is_duration_correct: true,
            is_hdr_correct: true,
            is_audio_opus: true,
            is_audio_track_count_correct: true,
            is_sync_preserved: true,
            codec_name: Some("av01".to_string()),
            pixel_format: Some("yuv420p10le".to_string()),
            bit_depth: Some(10),
            actual_dimensions: Some((1920, 1080)),
            expected_dimensions: Some((1920, 1080)),
            crop_message: Some("No crop expected, dimensions: 1920x1080".to_string()),
            actual_duration: Some(6155.0),
            expected_duration: Some(6155.0),
            duration_message: Some("Duration matches input (6155.0s)".to_string()),
            expected_hdr: Some(false),
            actual_hdr: Some(false),
            hdr_message: Some("SDR preserved".to_string()),
            audio_codecs: vec!["opus".to_string()],
            audio_message: Some("Audio track is Opus".to_string()),
            sync_drift_ms: Some(25.0),
            sync_message: Some("Audio/video sync preserved (drift: 25.0ms)".to_string()),
        };
        
        assert!(result.is_valid());
        assert_eq!(result.get_validation_steps().len(), 7);
        assert!(result.get_failures().is_empty());
    }
}