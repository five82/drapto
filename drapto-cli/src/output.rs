use std::fmt::Display;
use std::io::{self, Write};
use drapto_core::validation::{ValidationReport, ValidationLevel, ValidationMessage};
use colored::*;
use std::time::Duration;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

/// Print a heading with colored styling and clear separation
pub fn print_heading(text: &str) {
    let heading = format!(" {} ", text).bold().bright_white();
    let line = "=".repeat(50).bright_blue();
    
    println!("\n{}", line);
    println!("{}", heading);
    println!("{}\n", line);
}

/// Print a section heading (smaller than main heading) with colored styling
pub fn print_section(text: &str) {
    let section = format!(" {} ", text).bold().white();
    let line = "-".repeat(40).blue();
    
    println!("\n{}", line);
    println!("{}", section);
    println!("{}", line);
}

/// Print a separator line
pub fn print_separator() {
    println!("\n{}", "-".repeat(50).bright_blue());
}

/// Print an info line with label and value, with the label colored
pub fn print_info<T: Display>(label: &str, value: T) {
    println!("{}: {}", label.bright_cyan(), value);
}

/// Print a message with a specific color based on severity
pub fn print_message(message: &ValidationMessage) {
    match message.level {
        ValidationLevel::Info => println!("ℹ️  {}: {}", 
            message.category.bright_cyan().bold(), 
            message.message),
        ValidationLevel::Warning => println!("⚠️  {}: {}", 
            message.category.yellow().bold(), 
            message.message),
        ValidationLevel::Error => println!("❌ {}: {}", 
            message.category.bright_red().bold(), 
            message.message),
    }
}

/// Print a validation report with enhanced formatting and color
pub fn print_validation_report(report: &ValidationReport) {
    // Use the appropriate title with status indicators
    let title_status = if report.passed {
        if !report.warnings().is_empty() {
            "⚠️ Validation PASSED WITH WARNINGS".yellow().bold()
        } else {
            "✅ Validation PASSED".bright_green().bold() 
        }
    } else { 
        "❌ Validation FAILED".bright_red().bold() 
    };
    
    print_heading(&title_status.to_string());
    
    let errors = report.errors();
    let warnings = report.warnings();
    let infos = report.infos();
    
    // Count errors and warnings by category
    let mut category_stats = std::collections::HashMap::new();
    for msg in &report.messages {
        let entry = category_stats.entry(msg.category.clone()).or_insert((0, 0));
        match msg.level {
            ValidationLevel::Error => entry.0 += 1,
            ValidationLevel::Warning => entry.1 += 1,
            _ => {}
        }
    }
    
    // Print validation summary by category first
    print_section("Validation by Category");
    for (category, (errors, warnings)) in &category_stats {
        let status = if *errors > 0 {
            "❌".bright_red()
        } else if *warnings > 0 {
            "⚠️".yellow()
        } else {
            "✅".bright_green()
        };
        
        let status_text = if *errors > 0 {
            "FAILED".bright_red().bold()
        } else if *warnings > 0 {
            "PASSED WITH WARNINGS".yellow().bold()
        } else {
            "PASSED".bright_green().bold()
        };
        
        println!("  {} {} {}: {} {}, {} {}", 
            status,
            status_text,
            category.bright_cyan().bold(),
            errors.to_string().bright_red().bold(),
            "error(s)".bright_red(),
            warnings.to_string().yellow().bold(),
            "warning(s)".yellow());
    }
    
    // Group by category for detailed reporting
    let mut categories = std::collections::HashMap::new();
    for msg in &report.messages {
        categories
            .entry(msg.category.clone())
            .or_insert_with(Vec::new)
            .push(msg);
    }
    
    // Print detailed messages by category
    print_section("Detailed Results");
    for (category, messages) in categories {
        print_info(&category, "");
        
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
        
        println!("");
    }
    
    // Print summary with colors
    print_section("Validation Summary");
    println!("  {} {}", 
        errors.len().to_string().bold().bright_red(), 
        "error(s)".bright_red());
    println!("  {} {}", 
        warnings.len().to_string().bold().yellow(), 
        "warning(s)".yellow());
    println!("  {} {}", 
        infos.len().to_string().bold().bright_cyan(), 
        "info message(s)".bright_cyan());
    
    // Print overall status
    if report.passed {
        if !warnings.is_empty() {
            println!("\n⚠️ {}", "VALIDATION PASSED WITH WARNINGS - Output may have minor issues".yellow().bold());
        } else {
            println!("\n✅ {}", "VALIDATION PASSED - Output meets all quality criteria".bright_green().bold());
        }
    } else {
        println!("\n❌ {}", "VALIDATION FAILED - Output has quality issues that should be addressed".bright_red().bold());
    }
}

/// Print a progress message to stderr
pub fn print_progress(message: &str) -> io::Result<()> {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    write!(handle, "\r{}", message.cyan())?;
    handle.flush()
}

/// Create a progress bar for showing encoding progress
#[allow(dead_code)]
pub fn create_progress_bar(total: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} {msg} [{bar:40.cyan/blue}] {percent}% ({eta})")
        .unwrap()
        .progress_chars("█▓▒░ "));
    
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Create a multi-progress bar for tracking multiple concurrent tasks
#[allow(dead_code)]
pub fn create_multi_progress() -> MultiProgress {
    MultiProgress::new()
}

/// Print an error message with red styling
pub fn print_error(message: &str) {
    eprintln!("{} {}", "Error:".bold().bright_red(), message);
}

/// Print a success message with green styling and a checkmark
pub fn print_success(message: &str) {
    println!("{} {}", "✅".green(), message);
}

/// Print a warning message with yellow styling
pub fn print_warning(message: &str) {
    println!("{} {}", "⚠️".yellow(), message.yellow());
}

/// Print a status update with appropriate coloring based on status type
#[allow(dead_code)]
pub fn print_status(status_type: &str, message: &str) {
    match status_type.to_lowercase().as_str() {
        "start" | "starting" | "begin" => {
            println!("{} {} {}", "🚀".cyan(), status_type.cyan().bold(), message);
        }
        "processing" | "running" => {
            println!("{} {} {}", "🔄".blue(), status_type.blue().bold(), message);
        }
        "complete" | "completed" | "success" => {
            println!("{} {} {}", "✅".green(), status_type.green().bold(), message);
        }
        "warning" => {
            println!("{} {} {}", "⚠️".yellow(), status_type.yellow().bold(), message);
        }
        "error" | "failed" | "failure" => {
            println!("{} {} {}", "❌".bright_red(), status_type.bright_red().bold(), message);
        }
        _ => {
            println!("{} {} {}", "•".bright_white(), status_type.white().bold(), message);
        }
    }
}

/// Print configuration info in a formatted, easy-to-read block
#[allow(dead_code)]
pub fn print_config_block(title: &str, config_items: &[(&str, String)]) {
    print_section(&format!("{} Configuration", title));
    
    let max_key_length = config_items.iter()
        .map(|(key, _)| key.len())
        .max()
        .unwrap_or(0);
    
    for (key, value) in config_items {
        let padding = " ".repeat(max_key_length - key.len() + 2);
        println!("  {}{}{} {}", 
            key.bright_green().bold(), 
            padding, 
            ":".bright_white(),
            value);
    }
    
    println!();
}