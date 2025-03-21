use std::fmt::Display;
use std::io::{self, Write};
use drapto_core::validation::{ValidationReport, ValidationLevel, ValidationMessage};

/// Print a heading
pub fn print_heading(text: &str) {
    println!("\n=== {} ===", text);
}

/// Print a section heading (smaller than main heading)
pub fn print_section(text: &str) {
    println!("\n--- {} ---", text);
}

/// Print an info line with label and value
pub fn print_info<T: Display>(label: &str, value: T) {
    println!("{}: {}", label, value);
}

/// Print a message with a specific color based on severity
pub fn print_message(message: &ValidationMessage) {
    match message.level {
        ValidationLevel::Info => println!("ℹ️  {}: {}", message.category, message.message),
        ValidationLevel::Warning => println!("⚠️  {}: {}", message.category, message.message),
        ValidationLevel::Error => println!("❌ {}: {}", message.category, message.message),
    }
}

/// Print a validation report with formatting
pub fn print_validation_report(report: &ValidationReport) {
    print_heading(if report.passed { "Validation PASSED" } else { "Validation FAILED" });
    
    let errors = report.errors();
    let warnings = report.warnings();
    let infos = report.infos();
    
    // Group by category
    let mut categories = std::collections::HashMap::new();
    for msg in &report.messages {
        categories
            .entry(msg.category.clone())
            .or_insert_with(Vec::new)
            .push(msg);
    }
    
    // Print by category
    for (category, messages) in categories {
        print_section(&category);
        
        // Sort by severity
        let mut sorted_messages = messages.clone();
        sorted_messages.sort_by_key(|m| match m.level {
            ValidationLevel::Error => 0,
            ValidationLevel::Warning => 1,
            ValidationLevel::Info => 2,
        });
        
        for msg in sorted_messages {
            print_message(msg);
        }
    }
    
    println!("\nSummary: {} error(s), {} warning(s), {} info(s)",
        errors.len(), warnings.len(), infos.len());
}

/// Print a progress message to stderr
pub fn print_progress(message: &str) -> io::Result<()> {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    write!(handle, "\r{}", message)?;
    handle.flush()
}

/// Print an error message
pub fn print_error(message: &str) {
    eprintln!("Error: {}", message);
}

/// Print a success message
pub fn print_success(message: &str) {
    println!("✅ {}", message);
}

/// Print a warning message
pub fn print_warning(message: &str) {
    println!("⚠️  Warning: {}", message);
}