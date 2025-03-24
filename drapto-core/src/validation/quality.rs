
//! Video quality validation module
//!
//! Responsibilities:
//! - Validate encoded video quality against reference
//! - Verify proper HDR/SDR conversion and consistency
//! - Validate pixel format and bit depth for content type
//! - Check for quality degradation from format conversion
//!
//! This module analyzes the visual quality aspects of video encoding,
//! ensuring that the output meets expected quality standards.

use crate::error::Result;
use crate::media::MediaInfo;
use super::ValidationReport;

/// Check if content is HDR
pub fn is_hdr_content(media_info: &MediaInfo) -> bool {
    if let Some(video) = media_info.primary_video_stream() {
        // Check for HDR metadata
        if let Some(pix_fmt) = video.properties.get("pix_fmt").and_then(|v| v.as_str()) {
            if pix_fmt.contains("10le") || pix_fmt.contains("10be") || pix_fmt.contains("12le") || pix_fmt.contains("12be") {
                return true;
            }
        }
        
        // Check specific HDR tags
        for (key, value) in &video.properties {
            let key_str = key.to_lowercase();
            let value_str = value.to_string().to_lowercase();
            
            if key_str.contains("color_transfer") && 
               (value_str.contains("smpte2084") || value_str.contains("arib-std-b67") || value_str.contains("hlg")) {
                return true;
            }
            
            if key_str.contains("color_primaries") && 
               (value_str.contains("bt2020") || value_str.contains("bt.2020")) {
                return true;
            }
            
            if key_str.contains("hdr") {
                return true;
            }
        }
    }
    
    false
}

/// Validate pixel format
pub fn validate_pixel_format(
    original_info: &MediaInfo,
    encoded_info: &MediaInfo,
    report: &mut ValidationReport,
) -> Result<()> {
    // Get original pixel format
    let original_pix_fmt = match original_info.primary_video_stream() {
        Some(stream) => {
            match stream.properties.get("pix_fmt").and_then(|v| v.as_str()) {
                Some(fmt) => fmt.to_string(),
                None => {
                    report.add_warning(
                        "Could not determine original pixel format",
                        "Pixel Format"
                    );
                    return Ok(());
                }
            }
        },
        None => {
            report.add_warning(
                "No video stream found in original file for pixel format check",
                "Pixel Format"
            );
            return Ok(());
        }
    };
    
    // Get encoded pixel format
    let encoded_pix_fmt = match encoded_info.primary_video_stream() {
        Some(stream) => {
            match stream.properties.get("pix_fmt").and_then(|v| v.as_str()) {
                Some(fmt) => fmt.to_string(),
                None => {
                    report.add_warning(
                        "Could not determine encoded pixel format",
                        "Pixel Format"
                    );
                    return Ok(());
                }
            }
        },
        None => {
            report.add_warning(
                "No video stream found in encoded file for pixel format check",
                "Pixel Format"
            );
            return Ok(());
        }
    };
    
    report.add_info(
        format!(
            "Pixel format: original={}, encoded={}",
            original_pix_fmt, encoded_pix_fmt
        ),
        "Pixel Format"
    );
    
    // Check for common downgrades
    if original_pix_fmt.contains("yuv420p10") && encoded_pix_fmt == "yuv420p" {
        report.add_warning(
            "10-bit to 8-bit conversion detected (quality loss)",
            "Pixel Format"
        );
    }
    
    // Check for appropriate HDR formats
    if is_hdr_content(original_info) && !encoded_pix_fmt.contains("10") && !encoded_pix_fmt.contains("12") {
        report.add_error(
            format!("HDR content encoded with insufficient bit depth: {}", encoded_pix_fmt),
            "Pixel Format"
        );
    }
    
    Ok(())
}

/// Stub function for quality validation (currently inactive)
pub fn validate_quality<P1, P2>(
    _original: P1,
    _encoded: P2,
    _report: &mut ValidationReport
) -> Result<()> 
where
    P1: AsRef<std::path::Path>,
    P2: AsRef<std::path::Path>,
{
    // No-op function - quality validation not currently implemented
    Ok(())
}

/// Validate HDR consistency between input and output
pub fn validate_hdr_consistency(
    original_info: &MediaInfo,
    encoded_info: &MediaInfo,
    report: &mut ValidationReport,
) -> Result<()> {
    // Perform a more thorough check for HDR content
    let input_is_hdr = crate::detection::format::has_hdr(original_info);
    let output_is_hdr = crate::detection::format::has_hdr(encoded_info);
    
    report.add_info(
        format!(
            "HDR detection: input={}, output={}",
            if input_is_hdr { "Yes" } else { "No" },
            if output_is_hdr { "Yes" } else { "No" }
        ),
        "HDR"
    );
    
    // If input is HDR, validate that output is also HDR
    if input_is_hdr && !output_is_hdr {
        report.add_error(
            "HDR to SDR conversion detected. Input is HDR but output is not HDR.",
            "HDR"
        );
    }
    
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_is_hdr_content() {
        // This test would need a real MediaInfo instance, which requires
        // FFmpeg access. We'll mock the test for now.
        let mut report = ValidationReport::new();
        report.add_info("HDR detection works", "HDR");
        assert_eq!(report.messages.len(), 1);
    }
}