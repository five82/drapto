// ============================================================================
// drapto-cli/src/logging.rs
// ============================================================================
//
// LOGGING UTILITIES: Helper Functions for Logging
//
// This file contains utility functions related to logging in the Drapto CLI
// application. The main logging implementation uses the standard `log` crate
// with `env_logger` as the backend, configured in main.rs.
//
// KEY COMPONENTS:
// - Timestamp generation for log files and other time-based operations
// - Other logging-related utility functions
//
// USAGE:
// The application uses env_logger with the RUST_LOG environment variable:
// - RUST_LOG=info (default): Normal operation logs
// - RUST_LOG=debug: Detailed debugging information
// - RUST_LOG=trace: Very verbose debugging information
//
// AI-ASSISTANT-INFO: Logging utilities and helper functions

/// Returns the current local timestamp formatted as "YYYYMMDD_HHMMSS".
///
/// This function is used to generate unique timestamps for log files,
/// temporary directories, and other time-based operations.
///
/// # Returns
/// A string containing the formatted timestamp (e.g., "20240601_123045")
///
/// # Example
/// ```
/// use drapto_cli::logging::get_timestamp;
///
/// let log_filename = format!("drapto_log_{}.txt", get_timestamp());
/// // Result: "drapto_log_20240601_123045.txt"
/// ```
pub fn get_timestamp() -> String {
    chrono::Local::now().format("%Y%m%d_%H%M%S").to_string()
}

// Note: The previous custom log callback system has been replaced by
// standard `log` macros and `env_logger` initialization in `main.rs`.
// This provides better integration with the Rust ecosystem and
// standardized logging levels (error, warn, info, debug, trace).

use std::path::Path;
use anyhow::Result;

/// Strip ANSI escape codes from a string
fn strip_ansi_codes(s: &str) -> String {
    // Simple regex-free approach to strip ANSI codes
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Skip the escape sequence
            if chars.next() == Some('[') {
                // Skip until we find a letter (end of sequence)
                for next_ch in chars.by_ref() {
                    if next_ch.is_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }
    
    result
}

/// Setup logging for interactive mode that logs to both console and file
/// 
/// Logging is controlled by the RUST_LOG environment variable:
/// - Default: info level (normal output)
/// - With --verbose flag: debug level (detailed output)
/// - Can be overridden by setting RUST_LOG explicitly
pub fn setup_file_logging(log_path: &Path) -> Result<()> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Parse RUST_LOG environment variable to determine log level
    let log_level = std::env::var("RUST_LOG")
        .ok()
        .and_then(|s| s.parse::<log::LevelFilter>().ok())
        .unwrap_or_else(|| {
            // Check for drapto-specific level
            if let Ok(val) = std::env::var("RUST_LOG") {
                if val.starts_with("drapto=") {
                    let level_str = val.trim_start_matches("drapto=");
                    level_str.parse::<log::LevelFilter>().unwrap_or(log::LevelFilter::Info)
                } else {
                    log::LevelFilter::Info
                }
            } else {
                log::LevelFilter::Info
            }
        });
    
    // Console formatter - simple and clean
    let console_dispatch = fern::Dispatch::new()
        .format(|out, message, record| {
            let msg_str = format!("{}", message);
            
            // Check if this is ffmpeg output that already has [info] prefix
            if msg_str.starts_with("[info]") || msg_str.starts_with("Svt[info]:") {
                // Output as-is without additional formatting
                out.finish(format_args!("{}", message))
            } else if record.level() != log::Level::Info {
                out.finish(format_args!(
                    "[{}] {}",
                    record.level(),
                    message
                ))
            } else {
                out.finish(format_args!("{}", message))
            }
        })
        .level(log_level)
        .level_for("drapto", log_level)
        .level_for("drapto_cli", log_level)
        .level_for("drapto_core", log_level)
        // Filter out progress messages from console output
        .level_for("drapto::progress", log::LevelFilter::Off)
        .chain(std::io::stdout());
    
    // File formatter - strips ANSI codes for clean file output
    let file_dispatch = fern::Dispatch::new()
        .format(|out, message, record| {
            let msg_str = format!("{}", message);
            
            // Strip ANSI escape codes for clean file output
            let clean_str = strip_ansi_codes(&msg_str);
            
            // Format based on log level
            if clean_str.starts_with("[info]") || clean_str.starts_with("Svt[info]:") {
                out.finish(format_args!("{}", clean_str))
            } else if record.level() != log::Level::Info {
                out.finish(format_args!("[{}] {}", record.level(), clean_str))
            } else {
                out.finish(format_args!("{}", clean_str))
            }
        })
        .level(log_level)
        .level_for("drapto", log_level)
        .level_for("drapto_cli", log_level)
        .level_for("drapto_core", log_level)
        // Ensure progress messages are included in file output
        .level_for("drapto::progress", log_level)
        .chain(fern::log_file(log_path)?);
    
    // Combine both outputs
    fern::Dispatch::new()
        .level(log_level)
        .level_for("drapto", log_level)
        .level_for("drapto_cli", log_level)
        .level_for("drapto_core", log_level)
        .chain(console_dispatch)
        .chain(file_dispatch)
        .apply()?;
    
    Ok(())
}
