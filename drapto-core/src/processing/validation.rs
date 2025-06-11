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
    /// The actual codec found
    pub codec_name: Option<String>,
    /// The actual pixel format found
    pub pixel_format: Option<String>,
    /// The actual bit depth found
    pub bit_depth: Option<u32>,
}

impl ValidationResult {
    /// Returns true if all validations pass
    pub fn is_valid(&self) -> bool {
        self.is_av1 && self.is_10_bit
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
        
        steps
    }
}

/// Validates that the output video file has AV1 codec and 10-bit depth
pub fn validate_output_video(output_path: &Path) -> CoreResult<ValidationResult> {
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

    let result = ValidationResult {
        is_av1,
        is_10_bit,
        codec_name,
        pixel_format,
        bit_depth,
    };

    log::debug!("Validation result: {:?}", result);
    
    Ok(result)
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
            codec_name: Some("h264".to_string()),
            pixel_format: Some("yuv420p10le".to_string()),
            bit_depth: Some(10),
        };
        
        let failures = result.get_failures();
        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("Expected AV1 codec"));
        assert!(failures[0].contains("h264"));
    }
}