use std::path::Path;
use drapto_core::validation::{ValidationReport, comprehensive_validation};
use drapto_core::validation::quality;

#[test]
fn test_is_vmaf_available() {
    // This test just checks that the function runs without crashing
    let available = quality::is_vmaf_available();
    println!("VMAF available: {}", available);
}

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
    
    match comprehensive_validation(test_file) {
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