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

// ---- Standard library imports ----
use std::path::Path;
use std::time::Duration;

// ============================================================================
// PROGRESS REPORTING FUNCTIONS
// ============================================================================

/// Reports the start of an encoding process.
///
/// # Arguments
///
/// * `input_path` - Path to the input file
/// * `output_path` - Path to the output file
pub fn report_encode_start(input_path: &Path, output_path: &Path) {
    let filename = input_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| input_path.to_string_lossy().to_string());

    info!("Starting FFmpeg encode for: {}", filename);
    info!("  Output: {}", output_path.display());
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

    info!(
        "Encoding progress: {:.2}% ({} / {}), Speed: {:.2}x, Avg FPS: {:.2}, ETA: {}",
        percent,
        current_time,
        total_time,
        speed,
        fps,
        eta_str
    );

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

    info!("{}", filename);
    info!("  {:<13} {}", "Encode time:", crate::format_duration(duration));
    info!("  {:<13} {}", "Input size:", crate::format_bytes(input_size));
    info!("  {:<13} {}", "Output size:", crate::format_bytes(output_size));
    info!("  {:<13} {}", "Reduced by:", format!("{}%", reduction));
    info!("{}", "----------------------------------------");

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

    error!("Error encoding {}: {}", filename, message);
}

/// Reports hardware acceleration status.
///
/// # Arguments
///
/// * `available` - Whether hardware acceleration is available
/// * `acceleration_type` - Type of hardware acceleration (e.g., "VideoToolbox")
pub fn report_hardware_acceleration(available: bool, acceleration_type: &str) {
    if available {
        info!("Hardware: {} hardware decoding available", acceleration_type);
    } else {
        info!("Hardware: Using software decoding (hardware acceleration not available on this platform)");
    }
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
