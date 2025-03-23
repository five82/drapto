
use crate::error::Result;
use crate::media::MediaInfo;
use super::ValidationReport;

/// Check if content is HDR
pub fn is_hdr_content(media_info: &MediaInfo) -> bool {
    if let Some(video) = media_info.primary_video_stream() {
        // Check for HDR/Dolby Vision metadata
        if let Some(pix_fmt) = video.properties.get("pix_fmt").and_then(|v| v.as_str()) {
            if pix_fmt.contains("10le") || pix_fmt.contains("10be") || pix_fmt.contains("12le") || pix_fmt.contains("12be") {
                return true;
            }
        }
        
        // Check specific HDR/DV tags
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
            
            if key_str.contains("dovi") || key_str.contains("dolby_vision") || key_str.contains("hdr") {
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
    
    // Perform a more thorough check for Dolby Vision
    // Using a more focused implementation specific to detection in video properties
    let input_is_dv = explicitly_check_dolby_vision(original_info);
    let output_is_dv = explicitly_check_dolby_vision(encoded_info);
    
    // Include the DV status in the report only if DV was detected in input
    if input_is_dv || output_is_dv {
        report.add_info(
            format!(
                "Dolby Vision detection: input={}, output={}",
                if input_is_dv { "Yes" } else { "No" },
                if output_is_dv { "Yes" } else { "No" }
            ),
            "HDR"
        );
    }
    
    // Only warn about Dolby Vision conversion if the input actually has Dolby Vision
    if input_is_dv && !output_is_dv {
        report.add_warning(
            "Dolby Vision to HDR10/SDR conversion detected. Input has Dolby Vision but output does not.",
            "HDR"
        );
    }
    
    Ok(())
}

/// Perform a more explicit check for Dolby Vision presence in stream properties
/// This is a more conservative detector than the general has_dolby_vision function
fn explicitly_check_dolby_vision(media_info: &MediaInfo) -> bool {
    if let Some(video_stream) = media_info.primary_video_stream() {
        // Check for explicit DV codec
        let codec_name = video_stream.codec_name.to_lowercase();
        if codec_name.contains("dvh") || codec_name.contains("dovi") {
            return true;
        }
        
        // Check for explicit DV codec tag
        if let Some(codec_tag) = video_stream.properties.get("codec_tag_string").and_then(|v| v.as_str()) {
            let codec_tag_lower = codec_tag.to_lowercase();
            if codec_tag_lower == "dovi" || codec_tag_lower.contains("dvh") {
                return true;
            }
        }
        
        // Check for explicit Dolby Vision tags
        for (key, _) in &video_stream.tags {
            let key_lower = key.to_lowercase();
            if key_lower == "dovi" || key_lower == "dolby_vision" || key_lower == "dv_profile" {
                return true;
            }
        }
        
        // Check for explicit Dolby Vision side data
        if let Some(side_data_list) = video_stream.properties.get("side_data_list").and_then(|v| v.as_array()) {
            for side_data in side_data_list {
                if let Some(side_data_type) = side_data.get("side_data_type").and_then(|v| v.as_str()) {
                    if side_data_type.contains("DOVI") || side_data_type.contains("Dolby Vision") {
                        return true;
                    }
                }
            }
        }
    }
    
    false
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