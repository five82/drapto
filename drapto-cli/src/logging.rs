//! Logging utilities and helper functions.
//!
//! This module provides logging setup for both console and file output,
//! with support for different log levels controlled by the RUST_LOG environment variable.

/// Returns the current local timestamp formatted as "`YYYYMMDD_HHMMSS`".
///
/// This function is used to generate unique timestamps for log files,
/// temporary directories, and other time-based operations.
///
/// # Returns
/// A string containing the formatted timestamp (e.g., "`20240601_123045`")
///
/// # Example
/// ```
/// use drapto_cli::logging::get_timestamp;
///
/// let log_filename = format!("drapto_log_{}.txt", get_timestamp());
/// // Result: "drapto_log_20240601_123045.txt"
/// ```
#[must_use] pub fn get_timestamp() -> String {
    chrono::Local::now().format("%Y%m%d_%H%M%S").to_string()
}

use crate::error::CliResult;
use drapto_core::CoreError;
use std::path::Path;

/// Setup logging for interactive mode that logs to both console and file
///
/// Logging is controlled by the `RUST_LOG` environment variable:
/// - Default: info level (normal output)
/// - With --verbose flag: debug level (detailed output)
/// - Can be overridden by setting `RUST_LOG` explicitly
pub fn setup_file_logging(log_path: &Path) -> CliResult<()> {
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
                    level_str
                        .parse::<log::LevelFilter>()
                        .unwrap_or(log::LevelFilter::Info)
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
            let msg_str = format!("{message}");

            // Ffmpeg output already has prefixes, don't add more
            if msg_str.starts_with("[info]") || msg_str.starts_with("Svt[info]:") {
                out.finish(format_args!("{message}"));
            } else if record.level() != log::Level::Info {
                out.finish(format_args!("[{}] {}", record.level(), message));
            } else {
                out.finish(format_args!("{message}"));
            }
        })
        .level(log_level)
        .level_for("drapto", log_level)
        .level_for("drapto_cli", log_level)
        .level_for("drapto_core", log_level)
        .level_for("drapto::progress", log::LevelFilter::Off)
        .chain(std::io::stdout());

    let file_dispatch =
        fern::Dispatch::new()
            .format(|out, message, record| {
                let msg_str = format!("{message}");

                let clean_str = console::strip_ansi_codes(&msg_str);
                if clean_str.starts_with("[info]") || clean_str.starts_with("Svt[info]:") {
                    out.finish(format_args!("{clean_str}"));
                } else if record.level() != log::Level::Info {
                    out.finish(format_args!("[{}] {}", record.level(), clean_str));
                } else {
                    out.finish(format_args!("{clean_str}"));
                }
            })
            .level(log_level)
            .level_for("drapto", log_level)
            .level_for("drapto_cli", log_level)
            .level_for("drapto_core", log_level)
            .level_for("drapto::progress", log_level)
            .chain(fern::log_file(log_path).map_err(|e| {
                CoreError::OperationFailed(format!("Failed to open log file: {e}"))
            })?);

    fern::Dispatch::new()
        .level(log_level)
        .level_for("drapto", log_level)
        .level_for("drapto_cli", log_level)
        .level_for("drapto_core", log_level)
        .chain(console_dispatch)
        .chain(file_dispatch)
        .apply()
        .map_err(|e| CoreError::OperationFailed(format!("Failed to initialize logging: {e}")))?;

    Ok(())
}
