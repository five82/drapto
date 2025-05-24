// ============================================================================
// drapto-core/src/progress_reporting.rs
// ============================================================================
//
// PROGRESS REPORTING: Core Progress Reporting API
//
// This module provides an API for the core library to report progress and
// output messages in a consistent format, without direct dependencies on
// CLI-specific formatting. It uses the standard `log` crate for output.
//
// KEY COMPONENTS:
// - Progress callback system for real-time progress reporting
// - Standard message formatting functions with consistent styling
// - Verbosity control for filtering messages
//
// ARCHITECTURAL DESIGN:
// This module serves as an abstraction layer between the core library and
// the CLI interface. It maintains consistent formatting while allowing the
// CLI to control the actual rendering and styling. This creates a clean
// separation of concerns:
//
// 1. Core library: Reports events and progress via this API
// 2. CLI layer: Handles actual rendering and user interaction
//
// The core library uses standard logging interfaces while the CLI can
// implement more sophisticated output formatting.
//
// AI-ASSISTANT-INFO: Core progress reporting API with consistent formatting

// ---- External crate imports ----
use log::debug;
use once_cell::sync::Lazy;

// ---- Standard library imports ----
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

// ============================================================================
// PROGRESS REPORTER INTERFACE
// ============================================================================

/// A trait that defines how progress and status information should be displayed
/// This allows the core library to delegate all formatting decisions to the CLI
pub trait ProgressReporter: Send + Sync {
    /// Report a section header (major workflow phase)
    fn section(&self, title: &str);

    /// Report a subsection header
    fn subsection(&self, title: &str);

    /// Report a processing step (» Processing step)
    fn processing_step(&self, message: &str);

    /// Report a status line (key-value pair with optional highlighting)
    fn status(&self, label: &str, value: &str, highlight: bool);

    /// Report success message (✓ Success)
    fn success(&self, message: &str);

    /// Report a sub-item (indented under a processing step)
    fn sub_item(&self, message: &str);

    /// Report a completion with associated status (Success + Status line)
    fn completion_with_status(&self, success_message: &str, status_label: &str, status_value: &str);

    /// Report an analysis step with emoji
    fn analysis_step(&self, emoji: &str, message: &str);

    /// Report an encoding summary with formatted details
    fn encoding_summary(
        &self,
        filename: &str,
        duration: Duration,
        input_size: u64,
        output_size: u64,
    );

    /// Report video filters being applied
    fn video_filters(&self, filters_str: &str, is_sample: bool);

    /// Report film grain synthesis settings
    fn film_grain(&self, level: Option<u8>, is_sample: bool);

    /// Report duration used for progress calculation
    fn duration(&self, duration_secs: f64, is_sample: bool);

    /// Report an encoder message
    fn encoder_message(&self, message: &str, is_sample: bool);

    /// Report empty lines for spacing
    fn section_separator(&self);

    /// Report hardware acceleration status
    fn hardware_acceleration(&self, available: bool, acceleration_type: &str);

    /// Report encode start
    fn encode_start(&self, input_path: &Path, output_path: &Path);

    /// Report encode error
    fn encode_error(&self, input_path: &Path, message: &str);

    /// Report a simple message with specific log level
    fn log_message(&self, message: &str, level: LogLevel);

    /// Report progress with a progress bar
    fn progress_bar(
        &self,
        percent: f32,
        elapsed_secs: f64,
        total_secs: f64,
        speed: Option<f32>,
        fps: Option<f32>,
        eta: Option<Duration>,
    );

    /// Clear any active progress bar
    fn clear_progress_bar(&self);

    /// Report FFmpeg command details
    fn ffmpeg_command(&self, cmd_debug: &str, is_sample: bool);
}

// ============================================================================
// PROGRESS REPORTER REGISTRY
// ============================================================================

// Global progress reporter instance using lazily initialized static
static PROGRESS_REPORTER: Lazy<Mutex<Option<Box<dyn ProgressReporter>>>> =
    Lazy::new(|| Mutex::new(None));

/// Register a progress reporter implementation
/// This should be called by the CLI to provide formatting implementation
pub fn set_progress_reporter(reporter: Box<dyn ProgressReporter>) {
    if let Ok(mut r) = PROGRESS_REPORTER.lock() {
        *r = Some(reporter);
    }
}

/// Get a reference to the current progress reporter, if any
fn get_progress_reporter() -> Option<&'static dyn ProgressReporter> {
    // First try to get the lock
    if let Ok(reporter_guard) = PROGRESS_REPORTER.lock() {
        // If we have a reporter, return a reference to it
        if let Some(reporter) = &*reporter_guard {
            // This is unsafe but necessary because we need to return a static reference
            // The actual reporter is stored in a static Lazy<Mutex> so it lives for the program duration
            let reporter_ptr = reporter.as_ref() as *const dyn ProgressReporter;
            return Some(unsafe { &*reporter_ptr });
        }
    }
    None
}

// ============================================================================
// PROGRESS CALLBACK SYSTEM
// ============================================================================

/// Type definition for progress callback functions
pub type ProgressCallback = Box<dyn FnMut(f32, f64, f64, f32, f32, Duration) + Send + 'static>;

// Type alias for the complex callback storage type
type CallbackStorage = Mutex<Option<ProgressCallback>>;

// Global progress callback using lazily initialized static
static PROGRESS_CALLBACK: Lazy<CallbackStorage> = Lazy::new(|| Mutex::new(None));

/// Sets a callback function for progress reporting
///
/// This callback will be called whenever `report_encode_progress` is called,
/// allowing clients to handle progress updates in a custom way.
///
/// # Arguments
///
/// * `callback` - A boxed function that takes progress parameters
///   - percent: f32 - Progress percentage (0.0 to 100.0)
///   - current_secs: f64 - Current time position in seconds
///   - total_secs: f64 - Total duration in seconds
///   - speed: f32 - Encoding speed (e.g., 2.5x means 2.5x realtime)
///   - fps: f32 - Average frames per second
///   - eta: Duration - Estimated time remaining
pub fn set_progress_callback(callback: ProgressCallback) {
    if let Ok(mut cb) = PROGRESS_CALLBACK.lock() {
        *cb = Some(callback);
    }
}

/// Clears the progress callback
pub fn clear_progress_callback() {
    if let Ok(mut cb) = PROGRESS_CALLBACK.lock() {
        *cb = None;
    }
}

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
    if let Some(reporter) = get_progress_reporter() {
        reporter.encode_start(input_path, output_path);
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
    // Call progress callback if set
    let use_callback = if let Ok(mut callback_opt) = PROGRESS_CALLBACK.lock() {
        if let Some(callback) = &mut *callback_opt {
            // Call the callback with the progress parameters
            (callback)(percent, current_secs, total_secs, speed, fps, eta);
            true
        } else {
            false
        }
    } else {
        false
    };

    // Early return if callback was successfully called
    if use_callback {
        return;
    }

    // Otherwise, use the progress reporter for central formatting
    if let Some(reporter) = get_progress_reporter() {
        // Use the dedicated progress bar function
        reporter.progress_bar(
            percent,
            current_secs,
            total_secs,
            Some(speed),
            Some(fps),
            Some(eta),
        );
    }
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

    // Use the centralized encoding summary function
    report_encoding_summary(&filename, duration, input_size, output_size);

    // Calculate reduction percentage
    let reduction = if input_size > 0 {
        100.0 - ((output_size as f64 / input_size as f64) * 100.0)
    } else {
        0.0
    };

    // Also log to debug level for potential file logging without colors
    debug!(
        "Encode complete for {}: Duration: {}, Input: {} bytes, Output: {} bytes, Reduction: {:.2}%",
        filename,
        crate::utils::format_duration(duration),
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
    if let Some(reporter) = get_progress_reporter() {
        reporter.encode_error(input_path, message);
    }
}

/// Reports hardware acceleration status.
///
/// # Arguments
///
/// * `available` - Whether hardware acceleration is available
/// * `acceleration_type` - Type of hardware acceleration (e.g., "VideoToolbox")
pub fn report_hardware_acceleration(available: bool, acceleration_type: &str) {
    if let Some(reporter) = get_progress_reporter() {
        reporter.hardware_acceleration(available, acceleration_type);
    }
}

/// Reports a log message with the specified level.
///
/// # Arguments
///
/// * `message` - The log message
/// * `level` - The log level (debug, info, warning, error)
pub fn report_log_message(message: &str, level: LogLevel) {
    if let Some(reporter) = get_progress_reporter() {
        reporter.log_message(message, level);
    }
}

// ============================================================================
// TERMINAL UI STYLE HELPERS
// ============================================================================
// These functions provide a bridge to the CLI's terminal.rs formatting
// while keeping the core library independent of CLI-specific code.
// They use standard logging to avoid direct dependencies.

/// Reports a processing step in a consistent format
/// This automatically adds appropriate spacing before the step
///
/// # Arguments
///
/// * `message` - The processing step message
pub fn report_processing_step(message: &str) {
    if let Some(reporter) = get_progress_reporter() {
        reporter.processing_step(message);
    }
}

/// Reports a subsection header
///
/// # Arguments
///
/// * `title` - The subsection title
pub fn report_subsection(title: &str) {
    if let Some(reporter) = get_progress_reporter() {
        reporter.subsection(title);
    }
}

/// Reports a section header
///
/// # Arguments
///
/// * `title` - The section title
pub fn report_section(title: &str) {
    if let Some(reporter) = get_progress_reporter() {
        reporter.section(title);
    }
}

/// Reports a sub-item under a processing step
///
/// # Arguments
///
/// * `message` - The sub-item message
pub fn report_sub_item(message: &str) {
    if let Some(reporter) = get_progress_reporter() {
        reporter.sub_item(message);
    }
}

/// Reports a success message with checkmark
/// This automatically adds appropriate spacing before the message
///
/// # Arguments
///
/// * `message` - The success message
pub fn report_success(message: &str) {
    if let Some(reporter) = get_progress_reporter() {
        reporter.success(message);
    }
}

/// Reports a completion with associated status in a consistent format
/// This should be used for cases like "Crop detection complete" + "Detected crop: XXX"
///
/// # Arguments
///
/// * `success_message` - The main completion/success message
/// * `status_label` - The label for the status line
/// * `status_value` - The value for the status line
pub fn report_completion_with_status(
    success_message: &str,
    status_label: &str,
    status_value: &str,
) {
    if let Some(reporter) = get_progress_reporter() {
        reporter.completion_with_status(success_message, status_label, status_value);
    }
}

/// Reports a specialized analysis step with emoji
///
/// # Arguments
///
/// * `emoji` - The emoji character to use
/// * `message` - The message to display
pub fn report_analysis_step(emoji: &str, message: &str) {
    if let Some(reporter) = get_progress_reporter() {
        reporter.analysis_step(emoji, message);
    }
}

/// Reports an encoding summary with consistent formatting
///
/// # Arguments
///
/// * `filename` - Name of the encoded file
/// * `duration` - Encoding duration
/// * `input_size` - Size of input file in bytes
/// * `output_size` - Size of output file in bytes
pub fn report_encoding_summary(
    filename: &str,
    duration: Duration,
    input_size: u64,
    output_size: u64,
) {
    if let Some(reporter) = get_progress_reporter() {
        reporter.encoding_summary(filename, duration, input_size, output_size);
    }
}

/// Reports empty lines to separate logical groups
/// This follows the design guide by using proper spacing instead of separator lines
pub fn report_section_separator() {
    if let Some(reporter) = get_progress_reporter() {
        reporter.section_separator();
    }
}

/// Reports filters being applied to video encode
///
/// # Arguments
///
/// * `filters_str` - String describing the filters being applied
/// * `is_sample` - Whether this is a grain analysis sample (reduces verbosity)
pub fn report_video_filters(filters_str: &str, is_sample: bool) {
    if let Some(reporter) = get_progress_reporter() {
        reporter.video_filters(filters_str, is_sample);
    }
}

/// Reports film grain synthesis settings
///
/// # Arguments
///
/// * `level` - Film grain synthesis level
/// * `is_sample` - Whether this is a grain analysis sample (reduces verbosity)
pub fn report_film_grain(level: Option<u8>, is_sample: bool) {
    if let Some(reporter) = get_progress_reporter() {
        reporter.film_grain(level, is_sample);
    }
}

/// Reports duration used for progress calculation
///
/// # Arguments
///
/// * `duration_secs` - Duration in seconds
/// * `is_sample` - Whether this is a grain analysis sample (reduces verbosity)
pub fn report_duration(duration_secs: f64, is_sample: bool) {
    if let Some(reporter) = get_progress_reporter() {
        reporter.duration(duration_secs, is_sample);
    }
}

/// Reports encoder message from FFmpeg
///
/// # Arguments
///
/// * `message` - The message from the encoder
/// * `is_sample` - Whether this is a grain analysis sample (reduces verbosity)
pub fn report_encoder_message(message: &str, is_sample: bool) {
    if let Some(reporter) = get_progress_reporter() {
        reporter.encoder_message(message, is_sample);
    }
}

/// Reports a status line with a label and value
///
/// # Arguments
///
/// * `label` - The label for the status
/// * `value` - The value to display
pub fn report_status(label: &str, value: &str) {
    if let Some(reporter) = get_progress_reporter() {
        reporter.status(label, value, false);
    }
}

/// Reports debug-level information (previously verbose mode)
///
/// # Arguments
///
/// * `message` - The debug info message
pub fn report_debug_info(message: &str) {
    if let Some(reporter) = get_progress_reporter() {
        reporter.log_message(message, LogLevel::Debug);
    }
}


/// Reports FFmpeg command details with proper formatting
///
/// # Arguments
///
/// * `cmd_debug` - The debug string representation of the FFmpeg command
/// * `is_sample` - Whether this is for a grain analysis sample
pub fn report_ffmpeg_command(cmd_debug: &str, is_sample: bool) {
    if let Some(reporter) = get_progress_reporter() {
        reporter.ffmpeg_command(cmd_debug, is_sample);
    }
}

/// Clear any active progress bar
pub fn clear_progress_bar() {
    if let Some(reporter) = get_progress_reporter() {
        reporter.clear_progress_bar();
    }
}

// Removed separator line function as it's not used in the CLI design guide

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

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

// Verbosity is now handled by standard log levels (info, debug, trace)
// Use log::info! for normal output, log::debug! for verbose output
