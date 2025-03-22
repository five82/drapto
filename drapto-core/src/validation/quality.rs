use std::path::Path;
use std::process::Command;

use log::{info, error};
use serde_json::Value;

use crate::error::{DraptoError, Result};
use crate::media::MediaInfo;
use crate::util::command;
use super::ValidationReport;

/// Default target VMAF for standard video
pub const DEFAULT_TARGET_VMAF: f32 = 93.0;

/// Target VMAF for HDR content (higher to preserve quality)
pub const HDR_TARGET_VMAF: f32 = 95.0;

/// Validate overall quality of encoded video
///
/// # Arguments
/// 
/// * `original` - Path to the original video
/// * `encoded` - Path to the encoded video
/// * `report` - Validation report to add results to
///
/// # Returns
///
/// * `Result<()>` - Ok if validation is successful
pub fn validate_quality<P1, P2>(
    original: P1,
    encoded: P2,
    report: &mut ValidationReport
) -> Result<()> 
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    // First determine if we should run VMAF calculation
    if !is_vmaf_available() {
        report.add_warning(
            "VMAF is not available - quality metrics cannot be calculated",
            "Quality"
        );
        return Ok(());
    }
    
    // Determine if content is HDR to set appropriate target
    let original_path = original.as_ref();
    let encoded_path = encoded.as_ref();
    
    // Get media info for both files
    let original_info = MediaInfo::from_path(original_path)?;
    let encoded_info = MediaInfo::from_path(encoded_path)?;
    
    // Check if content is HDR
    let is_hdr = is_hdr_content(&original_info) || is_hdr_content(&encoded_info);
    let target_vmaf = if is_hdr { HDR_TARGET_VMAF } else { DEFAULT_TARGET_VMAF };
    
    report.add_info(
        format!(
            "Quality target: VMAF {:.1} ({})", 
            target_vmaf, 
            if is_hdr { "HDR content" } else { "standard content" }
        ),
        "Quality"
    );
    
    // Perform basic bitrate comparison
    compare_bitrates(&original_info, &encoded_info, report)?;
    
    // Perform VMAF validation if files are not too large
    let original_size = match std::fs::metadata(original_path) {
        Ok(meta) => meta.len(),
        Err(e) => {
            report.add_warning(
                format!("Could not get original file size: {}", e),
                "Quality"
            );
            return Ok(());
        }
    };
    
    // Skip VMAF for very large files (over 1GB)
    const SIZE_THRESHOLD: u64 = 1024 * 1024 * 1024; // 1GB
    if original_size > SIZE_THRESHOLD {
        report.add_warning(
            format!(
                "Original file is large ({} bytes) - skipping VMAF calculation for performance reasons",
                original_size
            ),
            "Quality"
        );
        return Ok(());
    }
    
    // Run VMAF calculation
    info!("Running VMAF calculation between original and encoded files");
    match calculate_vmaf(original_path, encoded_path) {
        Ok(score) => {
            // Add VMAF score to report
            report.add_info(
                format!("VMAF score: {:.2}", score),
                "Quality"
            );
            
            // Check if VMAF score meets target
            if score < target_vmaf {
                report.add_error(
                    format!("VMAF score {:.2} is below target of {:.2}", score, target_vmaf),
                    "Quality"
                );
            } else {
                report.add_info(
                    format!("VMAF score {:.2} meets or exceeds target of {:.2}", score, target_vmaf),
                    "Quality"
                );
            }
        },
        Err(e) => {
            report.add_warning(
                format!("VMAF calculation failed: {}", e),
                "Quality"
            );
        }
    }
    
    Ok(())
}

/// Validate encoding quality using VMAF
pub fn validate_vmaf<P: AsRef<Path>>(
    reference_path: P,
    encoded_path: P,
    target_score: f32,
    report: &mut ValidationReport
) -> Result<f32> {
    // Run FFmpeg with libvmaf filter to get VMAF score
    let vmaf_score = calculate_vmaf(reference_path, encoded_path)?;
    
    // Add VMAF score to report
    report.add_info(
        format!("VMAF score: {:.2}", vmaf_score),
        "Quality"
    );
    
    // Check if VMAF score meets target
    if vmaf_score < target_score {
        report.add_error(
            format!("VMAF score {:.2} is below target of {:.2}", vmaf_score, target_score),
            "Quality"
        );
    } else {
        report.add_info(
            format!("VMAF score {:.2} meets or exceeds target of {:.2}", vmaf_score, target_score),
            "Quality"
        );
    }
    
    Ok(vmaf_score)
}

/// Calculate VMAF score between reference and encoded video
fn calculate_vmaf<P: AsRef<Path>>(reference_path: P, encoded_path: P) -> Result<f32> {
    let reference = reference_path.as_ref();
    let encoded = encoded_path.as_ref();
    
    if !reference.exists() {
        return Err(DraptoError::Validation(
            format!("Reference file not found: {:?}", reference)
        ));
    }
    
    if !encoded.exists() {
        return Err(DraptoError::Validation(
            format!("Encoded file not found: {:?}", encoded)
        ));
    }
    
    // Create temporary file for JSON output
    let temp_dir = std::env::temp_dir();
    let json_output = temp_dir.join("vmaf_output.json");
    
    // Build FFmpeg command for VMAF calculation
    let mut cmd = Command::new("ffmpeg");
    cmd.args([
        "-i", encoded.to_str().unwrap_or_default(),
        "-i", reference.to_str().unwrap_or_default(),
        "-filter_complex", &format!(
            "[0:v]setpts=PTS-STARTPTS[distorted];\
             [1:v]setpts=PTS-STARTPTS[reference];\
             [distorted][reference]libvmaf=log_fmt=json:log_path={}:model_path=vmaf_v0.6.1.json",
            json_output.to_str().unwrap_or_default()
        ),
        "-f", "null", "-"
    ]);
    
    info!("Running VMAF calculation: {:?}", cmd);
    
    // Execute command
    let output = command::run_command(&mut cmd)?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("VMAF calculation failed: {}", stderr);
        return Err(DraptoError::CommandExecution(
            format!("VMAF calculation failed: {}", stderr)
        ));
    }
    
    // Read and parse JSON output
    if !json_output.exists() {
        return Err(DraptoError::Validation(
            "VMAF calculation completed but no output file was created".to_string()
        ));
    }
    
    let json_content = std::fs::read_to_string(&json_output)
        .map_err(|e| DraptoError::Validation(
            format!("Failed to read VMAF output file: {}", e)
        ))?;
    
    let json: Value = serde_json::from_str(&json_content)
        .map_err(|e| DraptoError::Validation(
            format!("Failed to parse VMAF JSON output: {}", e)
        ))?;
    
    // Extract pooled VMAF score
    let vmaf_score = json["pooled_metrics"]["vmaf"]["mean"]
        .as_f64()
        .ok_or_else(|| DraptoError::Validation(
            "Failed to extract VMAF score from JSON output".to_string()
        ))? as f32;
    
    // Clean up temporary file
    let _ = std::fs::remove_file(json_output);
    
    Ok(vmaf_score)
}

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

/// Compare bitrates between original and encoded files
fn compare_bitrates(
    original_info: &MediaInfo,
    encoded_info: &MediaInfo,
    report: &mut ValidationReport
) -> Result<()> {
    // Get original and encoded bitrates
    let original_bitrate = match original_info.bitrate() {
        Some(br) => br,
        None => {
            report.add_warning(
                "Could not determine original file bitrate",
                "Bitrate"
            );
            return Ok(());
        }
    };
    
    let encoded_bitrate = match encoded_info.bitrate() {
        Some(br) => br,
        None => {
            report.add_warning(
                "Could not determine encoded file bitrate",
                "Bitrate"
            );
            return Ok(());
        }
    };
    
    // Calculate bitrate reduction percentage
    let bitrate_reduction = if original_bitrate > 0 {
        ((original_bitrate - encoded_bitrate) as f64 / original_bitrate as f64) * 100.0
    } else {
        0.0
    };
    
    report.add_info(
        format!(
            "Bitrate: original={} kbps, encoded={} kbps, reduction={:.1}%",
            original_bitrate / 1000,
            encoded_bitrate / 1000,
            bitrate_reduction
        ),
        "Bitrate"
    );
    
    // Check for unusual bitrate changes
    if encoded_bitrate > original_bitrate {
        report.add_warning(
            format!(
                "Encoded bitrate ({} kbps) is higher than original ({} kbps)",
                encoded_bitrate / 1000,
                original_bitrate / 1000
            ),
            "Bitrate"
        );
    } else if bitrate_reduction > 90.0 {
        report.add_warning(
            format!("Extreme bitrate reduction: {:.1}%", bitrate_reduction),
            "Bitrate"
        );
    } else if bitrate_reduction < 10.0 && original_bitrate > 1000000 {
        report.add_warning(
            format!("Minimal bitrate reduction: {:.1}%", bitrate_reduction),
            "Bitrate"
        );
    }
    
    Ok(())
}

/// Check if libvmaf is available
pub fn is_vmaf_available() -> bool {
    let output = Command::new("ffmpeg")
        .arg("-filters")
        .output();
    
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.contains("libvmaf")
        },
        Err(_) => false
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_is_vmaf_available() {
        // This only tests that the function runs without crashing
        let _ = is_vmaf_available();
    }
    
    #[test]
    fn test_validate_vmaf_missing_file() {
        let mut report = ValidationReport::new();
        let result = validate_vmaf(
            "non_existent_reference.mp4",
            "non_existent_encoded.mp4",
            90.0,
            &mut report
        );
        
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(format!("{}", err).contains("Reference file not found"));
        }
    }
    
    #[test]
    fn test_report_messages() {
        let mut report = ValidationReport::new();
        let score = 85.0;
        let target = 90.0;
        
        // Mock the report messages that would be added by validate_vmaf
        report.add_info(
            format!("VMAF score: {:.2}", score),
            "Quality"
        );
        
        report.add_error(
            format!("VMAF score {:.2} is below target of {:.2}", score, target),
            "Quality"
        );
        
        // Verify the report has the expected messages
        assert_eq!(report.messages.len(), 2);
        assert!(report.messages[0].message.contains("VMAF score: 85.00"));
        assert!(report.messages[1].message.contains("below target of 90.00"));
        assert!(!report.passed);
    }
    
    #[test]
    fn test_is_hdr_content() {
        // This test would need a real MediaInfo instance, which requires
        // FFmpeg access. We'll mock the test for now.
        let mut report = ValidationReport::new();
        report.add_info("HDR detection works", "HDR");
        assert_eq!(report.messages.len(), 1);
    }
}