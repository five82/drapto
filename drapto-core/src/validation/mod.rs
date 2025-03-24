//! Media validation module
//!
//! Responsibilities:
//! - Validate encoding output against quality criteria
//! - Check for A/V sync issues in encoded media
//! - Verify video quality, resolution, and color space
//! - Validate audio tracks and subtitles
//! - Generate comprehensive validation reports
//!
//! This module provides a framework for validating various aspects of
//! media files including video, audio, subtitles, and synchronization.

use std::path::Path;
use log::{info, error, warn};

use crate::error::{DraptoError, Result};
use crate::media::MediaInfo;
use crate::logging;

pub mod video;
pub mod audio;
pub mod sync;
pub mod report;
pub mod subtitles;
pub mod quality;

// Re-export from report module
pub use report::{ValidationMessage, ValidationLevel, ValidationReport};

/// Validate a media file
pub fn validate_media<P: AsRef<Path>>(
    path: P,
    config: Option<&crate::config::Config>
) -> Result<ValidationReport> {
    let path_ref = path.as_ref();
    logging::log_section("MEDIA VALIDATION");
    info!("Validating media file: {}", path_ref.display());
    
    let mut report = ValidationReport::new();
    let media_info = MediaInfo::from_path(path_ref)?;
    
    // Run various validations with section headers
    logging::log_subsection("AUDIO VALIDATION");
    audio::validate_audio(&media_info, &mut report);
    
    logging::log_subsection("VIDEO VALIDATION");
    video::validate_video(&media_info, &mut report);
    
    logging::log_subsection("SUBTITLES VALIDATION");
    subtitles::validate_subtitles(&media_info, &mut report);
    
    logging::log_subsection("A/V SYNC VALIDATION");
    sync::validate_sync(&media_info, &mut report);
    
    // Validate codecs with config
    logging::log_subsection("CODEC VALIDATION");
    validate_codecs(&media_info, &mut report, config);
    
    // Log overall result
    logging::log_subsection("VALIDATION SUMMARY");
    if !report.passed {
        error!("Validation failed: {} error(s), {} warning(s)", 
               report.errors().len(), report.warnings().len());
        return Err(DraptoError::Validation(
            format!("Media validation failed: {} error(s)", report.errors().len())
        ));
    }
    
    info!("Validation passed: {} warning(s)", report.warnings().len());
    Ok(report)
}

/// Run comprehensive validation on a media file
/// 
/// This performs all available validations and returns a detailed report
pub fn comprehensive_validation<P: AsRef<Path>>(
    path: P,
    config: Option<&crate::config::Config>
) -> Result<ValidationReport> {
    let path_ref = path.as_ref();
    let mut report = ValidationReport::new();
    
    logging::log_section("COMPREHENSIVE VALIDATION");
    info!("Running comprehensive validation on: {}", path_ref.display());
    
    // Get media info
    let media_info = MediaInfo::from_path(path_ref)?;
    
    // Basic validation categories with subsections
    logging::log_subsection("AUDIO VALIDATION");
    audio::validate_audio(&media_info, &mut report);
    
    logging::log_subsection("VIDEO VALIDATION");
    video::validate_video(&media_info, &mut report);
    
    logging::log_subsection("SUBTITLES VALIDATION");
    subtitles::validate_subtitles(&media_info, &mut report);
    
    logging::log_subsection("A/V SYNC VALIDATION");
    sync::validate_sync(&media_info, &mut report);
    
    // Validate codecs with config
    logging::log_subsection("CODEC VALIDATION");
    validate_codecs(&media_info, &mut report, config);
    
    // Additional validation for format detection
    logging::log_subsection("FORMAT DETECTION");
    if let Some(video) = media_info.primary_video_stream() {
        // Check HDR
        if quality::is_hdr_content(&media_info) {
            report.add_info(
                "HDR content detected",
                "Format Detection"
            );
            
            // Check for appropriate pixel format
            if let Some(pix_fmt) = video.properties.get("pix_fmt").and_then(|v| v.as_str()) {
                if !pix_fmt.contains("10") && !pix_fmt.contains("12") {
                    report.add_error(
                        format!("HDR content with insufficient bit depth: {}", pix_fmt),
                        "Format Detection"
                    );
                }
            }
        }
        
        // Check color properties
        logging::log_subsection("COLOR PROPERTIES");
        if let Some(color_space) = video.properties.get("color_space").and_then(|v| v.as_str()) {
            report.add_info(
                format!("Color space: {}", color_space),
                "Color Properties"
            );
        }
        
        if let Some(color_transfer) = video.properties.get("color_transfer").and_then(|v| v.as_str()) {
            report.add_info(
                format!("Color transfer: {}", color_transfer),
                "Color Properties"
            );
        }
        
        if let Some(color_primaries) = video.properties.get("color_primaries").and_then(|v| v.as_str()) {
            report.add_info(
                format!("Color primaries: {}", color_primaries),
                "Color Properties"
            );
        }
    }
    
    // Check for container issues
    logging::log_subsection("CONTAINER VALIDATION");
    if let Some(format) = &media_info.format {
        report.add_info(
            format!("Container: {} ({})", 
                    format.format_name, 
                    format.format_long_name.as_deref().unwrap_or("unknown")),
            "Container"
        );
        
        // Verify common container properties
        match format.format_name.as_str() {
            "matroska" | "webm" => {
                report.add_info(
                    "Using Matroska container",
                    "Container"
                );
            },
            "mp4" | "mov" => {
                report.add_info(
                    "Using MP4/MOV container",
                    "Container"
                );
                
                // Check if AV1 is in MP4 (should ideally be in MKV)
                if media_info.video_streams().iter()
                             .any(|s| s.codec_name.contains("av1")) {
                    report.add_warning(
                        "AV1 codec in MP4 container - consider using MKV for better compatibility",
                        "Container"
                    );
                }
            },
            _ => {
                report.add_info(
                    format!("Using {} container", format.format_name),
                    "Container"
                );
            }
        }
    }
    
    // Set overall pass/fail status
    logging::log_subsection("VALIDATION SUMMARY");
    if report.errors().is_empty() {
        report.passed = true;
        info!("Comprehensive validation passed: {} warnings", report.warnings().len());
    } else {
        report.passed = false;
        error!("Comprehensive validation failed: {} errors, {} warnings", 
               report.errors().len(), report.warnings().len());
    }
    
    Ok(report)
}

/// Validate A/V synchronization
pub fn validate_av_sync<P: AsRef<Path>>(
    path: P,
    _config: Option<&crate::config::Config>
) -> Result<ValidationReport> {
    let path_ref = path.as_ref();
    logging::log_section("A/V SYNC VALIDATION");
    info!("Validating A/V sync in: {}", path_ref.display());
    
    let mut report = ValidationReport::new();
    let media_info = MediaInfo::from_path(path_ref)?;
    
    // Use the sync module to check AV synchronization
    sync::validate_sync(&media_info, &mut report);
    
    // Log summary
    logging::log_subsection("SYNC VALIDATION SUMMARY");
    if !report.passed {
        error!("A/V sync validation failed: {} error(s)", report.errors().len());
        return Err(DraptoError::Validation(
            format!("A/V sync validation failed: {} error(s)", report.errors().len())
        ));
    }
    
    info!("A/V sync validation passed");
    Ok(report)
}

/// Validate output file by comparing with input
/// 
/// This performs a comprehensive validation comparing:
/// 1. Video and audio quality
/// 2. A/V synchronization
/// 3. Duration matching
/// 4. Content completeness
/// 5. Codec compliance
///
/// # Arguments
///
/// * `input_file` - Original input file path
/// * `output_file` - Encoded output file path
/// * `config` - Optional configuration for validation settings
///
/// # Returns
///
/// * `Result<ValidationReport>` - Validation report 
pub fn validate_output<P1, P2>(
    input_file: P1, 
    output_file: P2, 
    config: Option<&crate::config::Config>
) -> Result<ValidationReport>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let input_path = input_file.as_ref();
    let output_path = output_file.as_ref();
    
    logging::log_section("OUTPUT VALIDATION");
    info!("Validating output: {}", output_path.display());
    
    // Ensure output file exists and has content
    if !output_path.exists() {
        return Err(DraptoError::Validation(
            format!("Output file not found: {}", output_path.display())
        ));
    }
    
    let output_size = match std::fs::metadata(output_path) {
        Ok(metadata) => metadata.len(),
        Err(e) => {
            return Err(DraptoError::Validation(
                format!("Failed to get output file metadata: {}", e)
            ));
        }
    };
    
    if output_size == 0 {
        return Err(DraptoError::Validation(
            format!("Output file is empty: {}", output_path.display())
        ));
    }
    
    // Get media information
    let input_info = MediaInfo::from_path(input_path)?;
    let output_info = MediaInfo::from_path(output_path)?;
    
    let mut report = ValidationReport::new();
    
    // Add basic file information
    report.add_info(
        format!("Input: {} ({} bytes)", 
                input_path.file_name().unwrap_or_default().to_string_lossy(),
                std::fs::metadata(input_path)?.len()),
        "File"
    );
    
    report.add_info(
        format!("Output: {} ({} bytes)", 
                output_path.file_name().unwrap_or_default().to_string_lossy(),
                output_size),
        "File"
    );
    
    // Run each validation component with clear section headers
    
    // 1. Duration validation
    logging::log_subsection("DURATION VALIDATION");
    validate_duration(&input_info, &output_info, &mut report)?;
    
    // 2. Video validation
    logging::log_subsection("VIDEO VALIDATION");
    video::validate_video(&output_info, &mut report);
    
    // 3. Audio validation
    logging::log_subsection("AUDIO VALIDATION");
    audio::validate_audio(&output_info, &mut report);
    
    // 4. A/V sync validation
    logging::log_subsection("A/V SYNC VALIDATION");
    sync::validate_sync(&output_info, &mut report);
    
    // 5. Codec compliance
    logging::log_subsection("CODEC VALIDATION");
    validate_codecs(&output_info, &mut report, config);
    
    // 6. Pixel format validation
    logging::log_subsection("PIXEL FORMAT VALIDATION");
    quality::validate_pixel_format(&input_info, &output_info, &mut report)?;
    
    // 6.1 HDR consistency validation
    logging::log_subsection("HDR VALIDATION");
    quality::validate_hdr_consistency(&input_info, &output_info, &mut report)?;
    
    // 7. Subtitles validation
    logging::log_subsection("SUBTITLES VALIDATION");
    subtitles::validate_subtitles(&output_info, &mut report);
    
    // 8. Compare subtitles between input and output
    logging::log_subsection("SUBTITLES COMPARISON");
    subtitles::compare_subtitles(input_path, output_path, &mut report)?;
    
    // Quality validation is disabled but function is kept to maintain API compatibility
    quality::validate_quality(input_path, output_path, &mut report)?;
    
    // Set overall pass/fail status
    logging::log_subsection("VALIDATION SUMMARY");
    if report.errors().is_empty() {
        report.passed = true;
        info!("Validation passed: {} warnings", report.warnings().len());
    } else {
        report.passed = false;
        error!("Validation failed: {} errors, {} warnings", 
              report.errors().len(), report.warnings().len());
        
        // Don't fail on warnings only
        if report.errors().is_empty() {
            warn!("Validation warnings, but continuing");
            report.passed = true;
        }
    }
    
    Ok(report)
}

/// Validate matching duration between input and output
fn validate_duration(
    input_info: &MediaInfo,
    output_info: &MediaInfo,
    report: &mut ValidationReport
) -> Result<()> {
    let input_duration = input_info.duration()
        .ok_or_else(|| DraptoError::Validation("Could not determine input duration".to_string()))?;
    
    let output_duration = output_info.duration()
        .ok_or_else(|| DraptoError::Validation("Could not determine output duration".to_string()))?;
    
    // Allow a tolerance of 1% of duration or at least 0.5 seconds
    let tolerance = (input_duration * 0.01).max(0.5);
    let duration_diff = (input_duration - output_duration).abs();
    
    report.add_info(
        format!("Input duration: {:.2}s, Output duration: {:.2}s", 
                input_duration, output_duration),
        "Duration"
    );
    
    if duration_diff > tolerance {
        report.add_error(
            format!("Duration mismatch: input={:.2}s vs output={:.2}s (diff={:.2}s, tolerance={:.2}s)",
                   input_duration, output_duration, duration_diff, tolerance),
            "Duration"
        );
    } else {
        report.add_info(
            format!("Duration match within tolerance ({:.2}s)", tolerance),
            "Duration"
        );
    }
    
    Ok(())
}

/// Validate codec compliance
fn validate_codecs(
    output_info: &MediaInfo, 
    report: &mut ValidationReport,
    _config: Option<&crate::config::Config>,
) {
    // Check video codec is AV1
    if let Some(video_stream) = output_info.primary_video_stream() {
        let codec = &video_stream.codec_name;
        
        if codec != "av1" {
            report.add_error(
                format!("Incorrect video codec: {} (expected AV1)", codec),
                "Codec"
            );
        } else {
            report.add_info("Video codec: AV1", "Codec");
        }
    } else {
        report.add_error("No video stream found", "Codec");
    }
    
    // Check audio codec is Opus
    for (i, audio) in output_info.audio_streams().iter().enumerate() {
        let codec = &audio.codec_name;
        
        if codec != "opus" {
            // All audio tracks should be encoded to Opus, this is always an error
            // since our encoder only produces Opus audio
            report.add_error(
                format!("Audio track {} has non-opus codec: {} (all audio must be encoded to opus)", i, codec),
                "Codec"
            );
        } else {
            report.add_info(format!("Audio track {}: Opus", i), "Codec");
            
            // Check for opus-specific parameters
            if let Some(application) = audio.properties.get("application").and_then(|v| v.as_str()) {
                if application != "audio" {
                    report.add_warning(
                        format!("Audio track {} has application={} (expected 'audio')", i, application),
                        "Codec"
                    );
                }
            }
        }
    }
}