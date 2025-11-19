//! Main validation orchestration

use crate::error::{CoreError, CoreResult};
use ffprobe::ffprobe;
use std::path::Path;

use super::audio;
use super::dimensions;
use super::duration;
use super::hdr;
use super::result::ValidationResult;
use super::video;

/// Validates that the output video file has AV1 codec, 10-bit depth, correct crop dimensions, matching duration, HDR status, appropriate audio codecs, and preserved sync
pub fn validate_output_video(
    input_path: &Path,
    output_path: &Path,
    expected_dimensions: Option<(u32, u32)>,
    expected_duration: Option<f64>,
    expected_hdr: Option<bool>,
    expected_audio_track_count: Option<usize>,
    spatial_audio_streams: Option<&[bool]>, // Legacy parameter; spatial audio preservation removed
) -> CoreResult<ValidationResult> {
    log::debug!("Validating output video: {}", output_path.display());

    let metadata = ffprobe(output_path).map_err(|e| {
        CoreError::FfprobeParse(format!(
            "Failed to probe output file {}: {:?}",
            output_path.display(),
            e
        ))
    })?;

    let video_stream = metadata
        .streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("video"))
        .ok_or_else(|| {
            CoreError::VideoInfoError(format!(
                "No video stream found in output file: {}",
                output_path.display()
            ))
        })?;

    // Validate video codec and bit depth
    let (is_av1, is_10_bit, codec_name, pixel_format, bit_depth) =
        video::validate_video_codec_and_depth(video_stream);

    // Validate dimensions/crop
    let (is_crop_correct, actual_dimensions, crop_message) =
        dimensions::validate_dimensions(video_stream, expected_dimensions);

    // Validate duration
    let (is_duration_correct, actual_duration, duration_message) =
        duration::validate_duration(&metadata, video_stream, expected_duration);

    // Validate HDR status using MediaInfo
    let (is_hdr_correct, actual_hdr, hdr_message) =
        hdr::validate_hdr_status_with_path(output_path, expected_hdr);

    // Validate audio codec and track count
    let (is_audio_opus, is_audio_track_count_correct, audio_codecs, audio_message) =
        audio::validate_audio_codec(&metadata, expected_audio_track_count, spatial_audio_streams);

    // Validate audio/video sync preservation
    let (is_sync_preserved, sync_drift_ms, sync_message) =
        duration::validate_sync_preservation(input_path, output_path);

    let result = ValidationResult {
        is_av1,
        is_10_bit,
        is_crop_correct,
        is_duration_correct,
        is_hdr_correct,
        is_audio_opus,
        is_audio_track_count_correct,
        is_sync_preserved,
        codec_name,
        pixel_format,
        bit_depth,
        actual_dimensions,
        expected_dimensions,
        crop_message,
        actual_duration,
        expected_duration,
        duration_message,
        expected_hdr,
        actual_hdr,
        hdr_message,
        audio_codecs,
        audio_message,
        sync_drift_ms,
        sync_message: Some(sync_message),
    };

    log::debug!("Validation result: {:?}", result);

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_validate_output_video_error_handling() {
        // Test with non-existent file
        let non_existent_path = PathBuf::from("/non/existent/file.mkv");
        let input_path = PathBuf::from("/non/existent/input.mkv");
        let result = validate_output_video(
            &input_path,
            &non_existent_path,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(result.is_err());

        // Should be an FfprobeParse error
        match result.unwrap_err() {
            CoreError::FfprobeParse(_) => (),
            other => panic!("Expected FfprobeParse error, got: {:?}", other),
        }
    }
}
