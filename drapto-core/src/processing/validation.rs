//! Post-encode validation functions
//!
//! This module provides functions to validate the output of video encoding operations.
//! It verifies that the encoded video meets the expected specifications.

use crate::error::{CoreError, CoreResult};
use ffprobe::ffprobe;
use std::path::Path;

/// Validation results for an encoded video file
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the video codec is AV1
    pub is_av1: bool,
    /// Whether the video is 10-bit
    pub is_10_bit: bool,
    /// Whether crop was applied correctly (if crop was expected)
    pub is_crop_correct: bool,
    /// Whether duration matches the input duration
    pub is_duration_correct: bool,
    /// The actual codec found
    pub codec_name: Option<String>,
    /// The actual pixel format found
    pub pixel_format: Option<String>,
    /// The actual bit depth found
    pub bit_depth: Option<u32>,
    /// The actual video dimensions found
    pub actual_dimensions: Option<(u32, u32)>,
    /// The expected dimensions after crop
    pub expected_dimensions: Option<(u32, u32)>,
    /// Crop validation message
    pub crop_message: Option<String>,
    /// The actual duration found (in seconds)
    pub actual_duration: Option<f64>,
    /// The expected duration (in seconds)
    pub expected_duration: Option<f64>,
    /// Duration validation message
    pub duration_message: Option<String>,
}

impl ValidationResult {
    /// Returns true if all validations pass
    pub fn is_valid(&self) -> bool {
        self.is_av1 && self.is_10_bit && self.is_crop_correct && self.is_duration_correct
    }

    /// Returns a list of validation failures
    pub fn get_failures(&self) -> Vec<String> {
        let mut failures = Vec::new();
        
        if !self.is_av1 {
            let codec = self.codec_name.as_deref().unwrap_or("unknown");
            failures.push(format!("Expected AV1 codec, found: {}", codec));
        }
        
        if !self.is_10_bit {
            let pix_fmt = self.pixel_format.as_deref().unwrap_or("unknown");
            let bit_depth = self.bit_depth.map_or("unknown".to_string(), |d| d.to_string());
            failures.push(format!("Expected 10-bit depth, found: {} bit (pixel format: {})", bit_depth, pix_fmt));
        }
        
        if !self.is_crop_correct {
            if let Some(msg) = &self.crop_message {
                failures.push(format!("Crop validation failed: {}", msg));
            } else {
                failures.push("Crop validation failed".to_string());
            }
        }
        
        if !self.is_duration_correct {
            if let Some(msg) = &self.duration_message {
                failures.push(format!("Duration validation failed: {}", msg));
            } else {
                failures.push("Duration validation failed".to_string());
            }
        }
        
        failures
    }

    /// Returns individual validation step results
    pub fn get_validation_steps(&self) -> Vec<(String, bool, String)> {
        let mut steps = Vec::new();
        
        // AV1 codec validation
        let codec_name = self.codec_name.as_deref().unwrap_or("unknown");
        let codec_result = if self.is_av1 {
            format!("AV1 codec ({})", codec_name)
        } else {
            format!("Expected AV1, found: {}", codec_name)
        };
        steps.push(("Video codec".to_string(), self.is_av1, codec_result));
        
        // 10-bit depth validation
        let bit_depth_result = if self.is_10_bit {
            format!("{}-bit depth", self.bit_depth.unwrap_or(10))
        } else {
            let depth = self.bit_depth.map_or("unknown".to_string(), |d| d.to_string());
            format!("Expected 10-bit, found: {}-bit", depth)
        };
        steps.push(("Bit depth".to_string(), self.is_10_bit, bit_depth_result));
        
        // Crop validation
        let crop_result = if self.is_crop_correct {
            self.crop_message.as_ref()
                .map(|msg| msg.clone())
                .unwrap_or_else(|| "Crop applied correctly".to_string())
        } else {
            self.crop_message.as_ref()
                .map(|msg| msg.clone())
                .unwrap_or_else(|| "Crop validation failed".to_string())
        };
        steps.push(("Crop detection".to_string(), self.is_crop_correct, crop_result));
        
        // Duration validation
        let duration_result = if self.is_duration_correct {
            self.duration_message.as_ref()
                .map(|msg| msg.clone())
                .unwrap_or_else(|| "Duration matches input".to_string())
        } else {
            self.duration_message.as_ref()
                .map(|msg| msg.clone())
                .unwrap_or_else(|| "Duration validation failed".to_string())
        };
        steps.push(("Video duration".to_string(), self.is_duration_correct, duration_result));
        
        steps
    }
}

/// Validates that the output video file has AV1 codec, 10-bit depth, correct crop dimensions, and matching duration
pub fn validate_output_video(
    output_path: &Path, 
    expected_dimensions: Option<(u32, u32)>,
    expected_duration: Option<f64>
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

    let codec_name = video_stream.codec_name.clone();
    let pixel_format = video_stream.pix_fmt.clone();
    
    // Check if codec is AV1
    let is_av1 = codec_name
        .as_deref()
        .map(|c| c.eq_ignore_ascii_case("av01") || c.eq_ignore_ascii_case("av1"))
        .unwrap_or(false);

    // Check bit depth - try multiple methods
    let bit_depth = get_bit_depth_from_stream(video_stream);
    let is_10_bit = bit_depth.map_or(false, |depth| depth == 10);

    // Get actual video dimensions
    let actual_width = video_stream.width;
    let actual_height = video_stream.height;
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

    // Get actual video duration - try multiple sources
    let actual_duration = get_duration_from_metadata(&metadata, video_stream);

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

    let result = ValidationResult {
        is_av1,
        is_10_bit,
        is_crop_correct,
        is_duration_correct,
        codec_name,
        pixel_format,
        bit_depth,
        actual_dimensions,
        expected_dimensions,
        crop_message,
        actual_duration,
        expected_duration,
        duration_message,
    };

    log::debug!("Validation result: {:?}", result);
    
    Ok(result)
}

/// Extract duration from metadata using multiple methods
fn get_duration_from_metadata(metadata: &ffprobe::FfProbe, video_stream: &ffprobe::Stream) -> Option<f64> {
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

/// Extract bit depth from video stream using multiple methods
fn get_bit_depth_from_stream(stream: &ffprobe::Stream) -> Option<u32> {
    // Method 1: Check bits_per_raw_sample field
    if let Some(bits_str) = &stream.bits_per_raw_sample {
        if let Ok(bits) = bits_str.parse::<u32>() {
            if bits > 0 {
                return Some(bits);
            }
        }
    }

    // Method 2: Infer from pixel format
    if let Some(pix_fmt) = &stream.pix_fmt {
        return infer_bit_depth_from_pixel_format(pix_fmt);
    }

    // Method 3: Check profile for additional hints
    if let Some(profile) = &stream.profile {
        if profile.contains("10") {
            return Some(10);
        }
    }

    None
}

/// Infer bit depth from pixel format string
fn infer_bit_depth_from_pixel_format(pix_fmt: &str) -> Option<u32> {
    match pix_fmt {
        // 10-bit formats
        s if s.contains("10le") || s.contains("10be") => Some(10),
        s if s.contains("p010") || s.contains("p016") => Some(10),
        s if s.contains("yuv420p10") || s.contains("yuv422p10") || s.contains("yuv444p10") => Some(10),
        
        // 12-bit formats
        s if s.contains("12le") || s.contains("12be") => Some(12),
        s if s.contains("yuv420p12") || s.contains("yuv422p12") || s.contains("yuv444p12") => Some(12),
        
        // 8-bit formats (default)
        s if s.contains("yuv420p") || s.contains("yuv422p") || s.contains("yuv444p") => Some(8),
        s if s.contains("nv12") || s.contains("nv21") => Some(8),
        
        // If we can't determine, return None
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_format_bit_depth_inference() {
        assert_eq!(infer_bit_depth_from_pixel_format("yuv420p10le"), Some(10));
        assert_eq!(infer_bit_depth_from_pixel_format("yuv420p"), Some(8));
        assert_eq!(infer_bit_depth_from_pixel_format("yuv422p12le"), Some(12));
        assert_eq!(infer_bit_depth_from_pixel_format("p010le"), Some(10));
        assert_eq!(infer_bit_depth_from_pixel_format("unknown_format"), None);
    }

    #[test]
    fn test_validation_result_failures() {
        let result = ValidationResult {
            is_av1: false,
            is_10_bit: true,
            is_crop_correct: true,
            is_duration_correct: true,
            codec_name: Some("h264".to_string()),
            pixel_format: Some("yuv420p10le".to_string()),
            bit_depth: Some(10),
            actual_dimensions: Some((1920, 1080)),
            expected_dimensions: Some((1920, 1080)),
            crop_message: Some("No crop expected, dimensions: 1920x1080".to_string()),
            actual_duration: Some(6155.0),
            expected_duration: Some(6155.0),
            duration_message: Some("Duration matches input (6155.0s)".to_string()),
        };
        
        let failures = result.get_failures();
        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("Expected AV1 codec"));
        assert!(failures[0].contains("h264"));
    }

    #[test]
    fn test_crop_validation_steps() {
        // Test successful crop validation
        let result = ValidationResult {
            is_av1: true,
            is_10_bit: true,
            is_crop_correct: true,
            is_duration_correct: true,
            codec_name: Some("av01".to_string()),
            pixel_format: Some("yuv420p10le".to_string()),
            bit_depth: Some(10),
            actual_dimensions: Some((1856, 1044)),
            expected_dimensions: Some((1856, 1044)),
            crop_message: Some("Crop applied correctly (1856x1044)".to_string()),
            actual_duration: Some(6155.0),
            expected_duration: Some(6155.0),
            duration_message: Some("Duration matches input (6155.0s)".to_string()),
        };
        
        let steps = result.get_validation_steps();
        assert_eq!(steps.len(), 4);
        assert_eq!(steps[2].0, "Crop detection");
        assert_eq!(steps[2].1, true);
        assert!(steps[2].2.contains("Crop applied correctly"));

        // Test failed crop validation
        let result_failed = ValidationResult {
            is_av1: true,
            is_10_bit: true,
            is_crop_correct: false,
            is_duration_correct: true,
            codec_name: Some("av01".to_string()),
            pixel_format: Some("yuv420p10le".to_string()),
            bit_depth: Some(10),
            actual_dimensions: Some((1920, 1080)),
            expected_dimensions: Some((1856, 1044)),
            crop_message: Some("Expected 1856x1044, found 1920x1080".to_string()),
            actual_duration: Some(6155.0),
            expected_duration: Some(6155.0),
            duration_message: Some("Duration matches input (6155.0s)".to_string()),
        };
        
        let steps_failed = result_failed.get_validation_steps();
        assert_eq!(steps_failed[2].1, false);
        assert!(steps_failed[2].2.contains("Expected 1856x1044, found 1920x1080"));
    }

    #[test]
    fn test_duration_validation_steps() {
        // Test successful duration validation
        let result = ValidationResult {
            is_av1: true,
            is_10_bit: true,
            is_crop_correct: true,
            is_duration_correct: true,
            codec_name: Some("av01".to_string()),
            pixel_format: Some("yuv420p10le".to_string()),
            bit_depth: Some(10),
            actual_dimensions: Some((1920, 1080)),
            expected_dimensions: Some((1920, 1080)),
            crop_message: Some("No crop expected, dimensions: 1920x1080".to_string()),
            actual_duration: Some(6155.0),
            expected_duration: Some(6155.0),
            duration_message: Some("Duration matches input (6155.0s)".to_string()),
        };
        
        let steps = result.get_validation_steps();
        assert_eq!(steps.len(), 4);
        assert_eq!(steps[3].0, "Video duration");
        assert_eq!(steps[3].1, true);
        assert!(steps[3].2.contains("Duration matches input"));

        // Test failed duration validation
        let result_failed = ValidationResult {
            is_av1: true,
            is_10_bit: true,
            is_crop_correct: true,
            is_duration_correct: false,
            codec_name: Some("av01".to_string()),
            pixel_format: Some("yuv420p10le".to_string()),
            bit_depth: Some(10),
            actual_dimensions: Some((1920, 1080)),
            expected_dimensions: Some((1920, 1080)),
            crop_message: Some("No crop expected, dimensions: 1920x1080".to_string()),
            actual_duration: Some(6150.0),
            expected_duration: Some(6155.0),
            duration_message: Some("Expected 6155.0s, found 6150.0s (diff: 5.0s)".to_string()),
        };
        
        let steps_failed = result_failed.get_validation_steps();
        assert_eq!(steps_failed[3].1, false);
        assert!(steps_failed[3].2.contains("Expected 6155.0s, found 6150.0s"));
    }

    #[test]
    fn test_frame_rate_parsing() {
        assert_eq!(parse_frame_rate("30").unwrap(), 30.0);
        assert_eq!(parse_frame_rate("29.97").unwrap(), 29.97);
        assert_eq!(parse_frame_rate("30000/1001").unwrap(), 30000.0 / 1001.0);
        assert_eq!(parse_frame_rate("25/1").unwrap(), 25.0);
        assert!(parse_frame_rate("invalid").is_err());
        assert!(parse_frame_rate("30/0").is_err()); // Division by zero should be handled by parse
    }
}