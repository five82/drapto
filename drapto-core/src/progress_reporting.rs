// ============================================================================
// drapto-core/src/progress_reporting.rs
// ============================================================================
//
// PROGRESS REPORTING: Direct Progress Reporting Functions
//
// This module provides direct functions for reporting progress during encoding
// and other operations. It uses the standard `log` crate for output, eliminating
// the need for a separate callback-based progress reporting system.
//
// KEY COMPONENTS:
// - Functions for reporting different types of progress events
// - Helper functions for formatting durations, sizes, etc.
//
// DESIGN PHILOSOPHY:
// This module simplifies the progress reporting system by using the logging
// system directly, rather than through a callback-based abstraction. This
// reduces complexity while maintaining the same functionality.
//
// AI-ASSISTANT-INFO: Direct progress reporting functions using the log crate

// ---- External crate imports ----
use log::{debug, info, warn, error};
use colored::Colorize;

// ---- Standard library imports ----
use std::path::Path;
use std::time::Duration;

// ---- Internal crate imports ----
use crate::styling;

// ============================================================================
// PROGRESS REPORTING FUNCTIONS
// ============================================================================

/// Reports the start of an encoding process.
///
/// # Arguments
///
/// * `input_path` - Path to the input file
/// * `output_path` - Path to the output file
/// * `using_hw_accel` - Whether hardware acceleration is being used
pub fn report_encode_start(input_path: &Path, output_path: &Path, using_hw_accel: bool) {
    let filename = input_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| input_path.to_string_lossy().to_string());

    // Display a more prominent header for the encoding process using the new section formatting
    info!("{}", styling::format_section(&format!("Encoding {}", styling::format_filename(&filename))));

    // Group file information
    let info_lines = vec![
        styling::format_key_value("Input:", &input_path.display().to_string()),
        styling::format_key_value("Output:", &output_path.display().to_string()),
    ];

    info!("{}", styling::format_group("File Information:", &info_lines));

    // Display hardware acceleration status
    if using_hw_accel {
        info!("{}", styling::format_hardware_status(true, "VideoToolbox hardware decoding enabled"));
    }
}

/// Reports progress during an encoding process.
///
/// # Arguments
///
/// * `percent` - Current progress percentage (0.0 to 100.0)
/// * `current_secs` - Current time position in seconds
/// * `total_secs` - Total duration in seconds
/// * `speed` - Encoding speed (e.g., 2.5x means 2.5x realtime)
/// * `fps` - Average frames per second
/// * `eta` - Estimated time remaining
pub fn report_encode_progress(
    percent: f32,
    current_secs: f64,
    total_secs: f64,
    speed: f32,
    fps: f32,
    eta: Duration,
) {
    // Format the ETA
    let eta_str = if eta.as_secs() > 0 {
        crate::format_duration(eta)
    } else {
        "< 1s".to_string()
    };

    // Format the current and total time
    let current_time = format_duration_seconds(current_secs);
    let total_time = format_duration_seconds(total_secs);

    // Use the enhanced progress bar with additional context
    let context = format!("{}/{} at {:.2}x speed ({:.2} fps)",
        current_time,
        total_time,
        speed,
        fps
    );

    // Calculate elapsed time based on progress and total duration
    let elapsed_secs = (total_secs * (percent as f64 / 100.0)) / speed as f64;
    let elapsed = format_duration_seconds(elapsed_secs);

    info!("{}", styling::format_enhanced_progress(
        percent as f64,
        "Encoding:",
        &context,
        Some(&elapsed)
    ));

    // Also log to debug level for potential file logging without colors
    debug!(
        "Encoding progress: {:.2}% ({} / {}), Speed: {:.2}x, Avg FPS: {:.2}, ETA: {}",
        percent,
        current_time,
        total_time,
        speed,
        fps,
        eta_str
    );
}

/// Reports the completion of an encoding process.
///
/// # Arguments
///
/// * `input_path` - Path to the input file
/// * `_output_path` - Path to the output file (unused but kept for API consistency)
/// * `input_size` - Size of the input file in bytes
/// * `output_size` - Size of the output file in bytes
/// * `duration` - Total encoding time
pub fn report_encode_complete(
    input_path: &Path,
    _output_path: &Path,
    input_size: u64,
    output_size: u64,
    duration: Duration,
) {
    // Extract filename for logging
    let filename = input_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| input_path.to_string_lossy().to_string());

    // Calculate size reduction percentage
    let reduction = if input_size > 0 {
        100 - ((output_size * 100) / input_size)
    } else {
        0
    };

    // Display a more prominent completion message with improved visual structure
    info!("{}", styling::format_section(&format!("Encode Complete: {}", styling::format_filename(&filename))));

    // Group encode statistics with improved formatting
    let is_significant = reduction > 30;
    let reduction_value = if is_significant {
        format!("{}%", reduction).color(styling::COLOR_HIGHLIGHT).bold().to_string()
    } else {
        format!("{}%", reduction).color(styling::COLOR_VALUE).to_string()
    };

    let stats_lines = vec![
        styling::format_key_value("Encode time:", &crate::format_duration(duration)),
        styling::format_key_value("Input size:", &crate::format_bytes(input_size)),
        styling::format_key_value("Output size:", &crate::format_bytes(output_size)),
        styling::format_key_value("Reduced by:", &reduction_value),
    ];

    info!("{}", styling::format_group("Encode Statistics:", &stats_lines));

    // Also log to debug level for potential file logging without colors
    debug!(
        "Encode complete for {}: Duration: {}, Input: {} bytes, Output: {} bytes, Reduction: {}%",
        filename,
        crate::format_duration(duration),
        input_size,
        output_size,
        reduction
    );
}

/// Reports an error during an encoding process.
///
/// # Arguments
///
/// * `input_path` - Path to the input file
/// * `message` - Error message
pub fn report_encode_error(input_path: &Path, message: &str) {
    // Extract filename for logging
    let filename = input_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| input_path.to_string_lossy().to_string());

    // Display a more detailed error message with context
    error!("{}", styling::format_detailed_error(
        message,
        &format!("Failed to encode {}", styling::format_filename(&filename)),
        Some("Check FFmpeg output for more details")
    ));
}

/// Reports hardware acceleration status.
///
/// # Arguments
///
/// * `available` - Whether hardware acceleration is available
/// * `acceleration_type` - Type of hardware acceleration (e.g., "VideoToolbox")
pub fn report_hardware_acceleration(available: bool, acceleration_type: &str) {
    let status_icon = if available { "✅" } else { "ℹ️" };
    let status_text = if available {
        format!("{} hardware decoding enabled", acceleration_type).color(styling::COLOR_SUCCESS).bold().to_string()
    } else {
        "Using software decoding (hardware acceleration not available)".color(styling::COLOR_INFO).to_string()
    };

    info!("{} {}", status_icon, status_text);
}

/// Reports a log message with the specified level.
///
/// # Arguments
///
/// * `message` - The log message
/// * `level` - The log level (debug, info, warning, error)
pub fn report_log_message(message: &str, level: LogLevel) {
    match level {
        LogLevel::Debug => debug!("{}", message),
        LogLevel::Info => info!("{}", message),
        LogLevel::Warning => warn!("{}", message),
        LogLevel::Error => error!("{}", message),
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Formats a duration in seconds as a human-readable string.
///
/// # Arguments
///
/// * `seconds` - Duration in seconds
///
/// # Returns
///
/// * A formatted string (e.g., "01:30:45")
fn format_duration_seconds(seconds: f64) -> String {
    let hours = (seconds / 3600.0) as u64;
    let minutes = ((seconds % 3600.0) / 60.0) as u64;
    let secs = (seconds % 60.0) as u64;

    format!("{:02}:{:02}:{:02}", hours, minutes, secs)
}

/// Log levels for progress reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Debug-level message (verbose)
    Debug,
    /// Informational message
    Info,
    /// Warning message
    Warning,
    /// Error message
    Error,
}
