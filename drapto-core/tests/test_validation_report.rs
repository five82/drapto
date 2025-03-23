use drapto_core::validation::{ValidationReport, ValidationMessage, ValidationLevel};

#[test]
fn test_validation_report_creation() {
    // Create a new report
    let report = ValidationReport::new();
    
    // Check initial state
    assert!(report.passed);
    assert!(report.messages.is_empty());
    assert_eq!(report.errors().len(), 0);
    assert_eq!(report.warnings().len(), 0);
    assert_eq!(report.infos().len(), 0);
}

#[test]
fn test_add_info_message() {
    let mut report = ValidationReport::new();
    
    // Add an info message
    report.add_info("Test info message", "Test Category");
    
    // Check that report still passes
    assert!(report.passed);
    
    // Check message was added
    assert_eq!(report.messages.len(), 1);
    assert_eq!(report.infos().len(), 1);
    assert_eq!(report.warnings().len(), 0);
    assert_eq!(report.errors().len(), 0);
    
    // Check message content
    let msg = &report.messages[0];
    assert_eq!(msg.message, "Test info message");
    assert_eq!(msg.category, "Test Category");
    assert_eq!(msg.level, ValidationLevel::Info);
}

#[test]
fn test_add_warning_message() {
    let mut report = ValidationReport::new();
    
    // Add a warning message
    report.add_warning("Test warning message", "Test Category");
    
    // Check that report still passes (warnings don't fail validation)
    assert!(report.passed);
    
    // Check message was added
    assert_eq!(report.messages.len(), 1);
    assert_eq!(report.infos().len(), 0);
    assert_eq!(report.warnings().len(), 1);
    assert_eq!(report.errors().len(), 0);
    
    // Check message content
    let msg = &report.messages[0];
    assert_eq!(msg.message, "Test warning message");
    assert_eq!(msg.category, "Test Category");
    assert_eq!(msg.level, ValidationLevel::Warning);
}

#[test]
fn test_add_error_message() {
    let mut report = ValidationReport::new();
    
    // Add an error message
    report.add_error("Test error message", "Test Category");
    
    // Check that report now fails
    assert!(!report.passed);
    
    // Check message was added
    assert_eq!(report.messages.len(), 1);
    assert_eq!(report.infos().len(), 0);
    assert_eq!(report.warnings().len(), 0);
    assert_eq!(report.errors().len(), 1);
    
    // Check message content
    let msg = &report.messages[0];
    assert_eq!(msg.message, "Test error message");
    assert_eq!(msg.category, "Test Category");
    assert_eq!(msg.level, ValidationLevel::Error);
}

#[test]
fn test_format_function() {
    let mut report = ValidationReport::new();
    
    // Add messages of different types
    report.add_info("Test info message", "Category A");
    report.add_warning("Test warning message", "Category B");
    report.add_error("Test error message", "Category C");
    
    // Format report
    let formatted = report.format();
    
    // Check that important parts are included
    assert!(formatted.contains("VALIDATION REPORT SUMMARY - ✗ FAILED"));
    assert!(formatted.contains("Category: Category A"));
    assert!(formatted.contains("ℹ [INFO] Test info message"));
    assert!(formatted.contains("Category: Category B"));
    assert!(formatted.contains("⚠ [WARNING] Test warning message"));
    assert!(formatted.contains("Category: Category C"));
    assert!(formatted.contains("✗ [ERROR] Test error message"));
    assert!(formatted.contains("OVERALL SUMMARY: 1 error(s), 1 warning(s), 1 info message(s)"));
    assert!(formatted.contains("VALIDATION BY CATEGORY"));
}

#[test]
fn test_message_creation() {
    // Test direct message creation
    let info = ValidationMessage::info("Info message", "Test");
    let warning = ValidationMessage::warning("Warning message", "Test");
    let error = ValidationMessage::error("Error message", "Test");
    
    assert_eq!(info.level, ValidationLevel::Info);
    assert_eq!(warning.level, ValidationLevel::Warning);
    assert_eq!(error.level, ValidationLevel::Error);
    
    // Test message display
    let info_str = format!("{}", info);
    assert_eq!(info_str, "[INFO] Test: Info message");
}

#[test]
fn test_multiple_errors() {
    let mut report = ValidationReport::new();
    
    // Add multiple errors
    report.add_error("Error 1", "Test");
    report.add_error("Error 2", "Test");
    
    // Check that report fails
    assert!(!report.passed);
    
    // Check error count
    assert_eq!(report.errors().len(), 2);
}

#[test]
fn test_report_with_mixed_messages() {
    let mut report = ValidationReport::new();
    
    // Add mixed messages
    report.add_info("Info message", "Test");
    report.add_warning("Warning message", "Test");
    
    // Report should still pass with only info and warnings
    assert!(report.passed);
    
    // Add an error
    report.add_error("Error message", "Test");
    
    // Now report should fail
    assert!(!report.passed);
    
    // Check message counts
    assert_eq!(report.messages.len(), 3);
    assert_eq!(report.infos().len(), 1);
    assert_eq!(report.warnings().len(), 1);
    assert_eq!(report.errors().len(), 1);
}