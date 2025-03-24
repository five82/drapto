//! Tests for quality validation and media compatibility
//!
//! These tests verify:
//! - Comprehensive media file validation for quality issues
//! - HDR/SDR compatibility and consistency checking
//! - Validation report formatting and message categorization
//! - Proper handling of different video color spaces and bit depths
//!
//! Some tests require actual media files and are conditionally executed
//! based on file availability.

use std::path::Path;
use drapto_core::validation::{ValidationReport, comprehensive_validation};

/// This test requires real media files to validate, so it's disabled by default
#[test]
#[ignore]
fn test_comprehensive_validation_with_files() {
    // Replace these paths with actual test files if running manually
    let test_file = Path::new("path/to/test/file.mkv");
    
    if !test_file.exists() {
        println!("Test file not found, skipping test");
        return;
    }
    
    match comprehensive_validation(test_file, None) {
        Ok(report) => {
            println!("Validation report:\n{}", report);
            
            // Get statistics
            println!("Errors: {}", report.errors().len());
            println!("Warnings: {}", report.warnings().len());
            println!("Info messages: {}", report.infos().len());
        },
        Err(e) => {
            panic!("Validation failed: {}", e);
        }
    }
}

#[test]
fn test_report_formatting() {
    let mut report = ValidationReport::new();
    
    // Add various messages
    report.add_info("Video codec is AV1", "Video Codec");
    report.add_info("Audio codec is Opus", "Audio Codec");
    report.add_warning("Low video bitrate detected", "Bitrate");
    report.add_error("A/V sync mismatch detected", "Sync");
    
    // Format report
    let formatted = report.format();
    
    // Check that the report contains all the messages
    assert!(formatted.contains("Video codec is AV1"));
    assert!(formatted.contains("Audio codec is Opus"));
    assert!(formatted.contains("Low video bitrate detected"));
    assert!(formatted.contains("A/V sync mismatch detected"));
    
    // Check summary line
    assert!(formatted.contains("1 error(s), 1 warning(s), 2 info message(s)"));
}

#[test]
fn test_hdr_consistency_validation() {
    use drapto_core::validation::quality::validate_hdr_consistency;
    use drapto_core::media::{MediaInfo, StreamInfo, StreamType, FormatInfo};
    use std::collections::HashMap;
    use serde_json::json;

    // Create a mock HDR input MediaInfo object
    let mut input_media = MediaInfo {
        format: Some(FormatInfo {
            format_name: "matroska".to_string(),
            format_long_name: Some("Matroska".to_string()),
            duration: Some(120.0),
            bit_rate: Some(5000000),
            size: Some(75000000),
            tags: HashMap::new(),
        }),
        streams: Vec::new(),
        chapters: Vec::new(),
    };
    
    // Add a mock HDR video stream
    let mut input_properties = HashMap::new();
    input_properties.insert("color_transfer".to_string(), json!("smpte2084"));
    input_properties.insert("color_primaries".to_string(), json!("bt2020"));
    input_properties.insert("color_space".to_string(), json!("bt2020nc"));
    input_properties.insert("pix_fmt".to_string(), json!("yuv420p10le"));
    
    let hdr_stream = StreamInfo {
        index: 0,
        codec_type: StreamType::Video,
        codec_name: "av1".to_string(),
        codec_long_name: Some("AV1 (AOMedia Video 1)".to_string()),
        tags: HashMap::new(),
        properties: input_properties,
    };
    
    input_media.streams.push(hdr_stream);
    
    // Test case 1: HDR input and HDR output (should pass)
    let output_media = input_media.clone();
    let mut report = ValidationReport::new();
    
    validate_hdr_consistency(&input_media, &output_media, &mut report).unwrap();
    assert!(report.passed);
    assert_eq!(report.errors().len(), 0);
    
    // Test case 2: HDR input but SDR output (should fail)
    let mut output_media_sdr = input_media.clone();
    let sdr_stream = &mut output_media_sdr.streams[0];
    sdr_stream.properties.insert("color_transfer".to_string(), json!("bt709"));
    sdr_stream.properties.insert("color_primaries".to_string(), json!("bt709"));
    sdr_stream.properties.insert("color_space".to_string(), json!("bt709"));
    sdr_stream.properties.insert("pix_fmt".to_string(), json!("yuv420p"));
    
    let mut report2 = ValidationReport::new();
    validate_hdr_consistency(&input_media, &output_media_sdr, &mut report2).unwrap();
    assert!(!report2.passed);
    assert_eq!(report2.errors().len(), 1);
    
    // Check report contents
    let formatted = report2.format();
    assert!(formatted.contains("HDR to SDR conversion detected"));
    
    // Test case 3: Use a normal SDR input and output
    let mut sdr_input_media = MediaInfo {
        format: Some(FormatInfo {
            format_name: "mp4".to_string(),
            format_long_name: Some("MP4 Format".to_string()),
            duration: Some(120.0),
            bit_rate: Some(5000000),
            size: Some(75000000),
            tags: HashMap::new(),
        }),
        streams: Vec::new(),
        chapters: Vec::new(),
    };
    
    // Add a normal SDR stream
    let mut sdr_properties = HashMap::new();
    sdr_properties.insert("color_transfer".to_string(), json!("bt709"));
    sdr_properties.insert("color_primaries".to_string(), json!("bt709"));
    sdr_properties.insert("color_space".to_string(), json!("bt709"));
    sdr_properties.insert("pix_fmt".to_string(), json!("yuv420p"));
    
    let sdr_stream = StreamInfo {
        index: 0,
        codec_type: StreamType::Video,
        codec_name: "h264".to_string(),
        codec_long_name: Some("H.264 / AVC / MPEG-4 AVC".to_string()),
        tags: HashMap::new(),
        properties: sdr_properties,
    };
    
    sdr_input_media.streams.push(sdr_stream);
    
    // Check validation with SDR input and SDR output
    let mut report3 = ValidationReport::new();
    validate_hdr_consistency(&sdr_input_media, &sdr_input_media.clone(), &mut report3).unwrap();
    
    // Should NOT have any warnings with SDR input and output
    assert_eq!(report3.warnings().len(), 0);
}