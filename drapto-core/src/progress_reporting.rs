//! Simplified Progress Reporting API
//!
//! This module provides a minimal API for the core library to report progress
//! and output messages without direct dependencies on CLI-specific formatting.
//!
//! # Design Decisions
//! - Simplified trait with only essential methods
//! - Structured output levels instead of many specific methods
//! - Direct reporter access instead of wrapper functions
//! - No unsafe code or complex global state


use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;
use log::{log_enabled, Level};


/// Represents different levels of output for structured reporting
#[derive(Debug, Clone, Copy)]
pub enum OutputLevel {
    /// Major workflow phases (===== SECTION =====)
    Section,
    /// Subsections (bold text at level 2)
    Subsection,
    /// Processing steps (» Processing)
    Processing,
    /// Status information (key: value)
    Status,
    /// Success messages (✓ Success)
    Success,
    /// Error messages
    Error,
    /// Warning messages
    Warning,
    /// Debug information
    Debug,
    /// General information
    Info,
}

/// Log levels for simple messages
#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Debug,
}


/// A simplified trait for progress reporting
pub trait ProgressReporter: Send + Sync {
    /// Output a message at a specific level
    fn output(&self, level: OutputLevel, text: &str);

    /// Output a key-value status pair
    fn output_status(&self, label: &str, value: &str, highlight: bool);

    /// Report progress with a progress bar
    fn progress_bar(&self, percent: f32, elapsed_secs: f64, total_secs: f64);

    /// Clear any active progress bar
    fn clear_progress_bar(&self);

    /// Log a simple message
    fn log(&self, level: LogLevel, message: &str);

    /// Output raw `FFmpeg` command for debugging
    fn ffmpeg_command(&self, cmd_data: &str);
}


/// Global progress reporter instance
static PROGRESS_REPORTER: std::sync::LazyLock<Mutex<Option<Box<dyn ProgressReporter>>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));

/// Set the global progress reporter
pub fn set_progress_reporter(reporter: Box<dyn ProgressReporter>) {
    if let Ok(mut r) = PROGRESS_REPORTER.lock() {
        *r = Some(reporter);
    }
}

/// Execute a function with the progress reporter if available
#[inline]
pub fn with_reporter<F>(f: F)
where
    F: FnOnce(&dyn ProgressReporter),
{
    if let Ok(guard) = PROGRESS_REPORTER.lock() {
        if let Some(reporter) = guard.as_ref() {
            f(reporter.as_ref());
        }
    }
}


/// Output a section header
pub fn section(title: &str) {
    with_reporter(|r| r.output(OutputLevel::Section, title));
}

/// Output a processing step
pub fn processing(message: &str) {
    with_reporter(|r| r.output(OutputLevel::Processing, message));
}

/// Output a status line
pub fn status(label: &str, value: &str, highlight: bool) {
    with_reporter(|r| r.output_status(label, value, highlight));
}

/// Output a success message
pub fn success(message: &str) {
    with_reporter(|r| r.output(OutputLevel::Success, message));
}

/// Output an error message
pub fn error(message: &str) {
    with_reporter(|r| r.output(OutputLevel::Error, message));
}

/// Output a warning message
pub fn warning(message: &str) {
    with_reporter(|r| r.output(OutputLevel::Warning, message));
}

/// Output debug information
pub fn debug(message: &str) {
    with_reporter(|r| r.output(OutputLevel::Debug, message));
}

/// Output general information
pub fn info(message: &str) {
    with_reporter(|r| r.output(OutputLevel::Info, message));
}

/// Report progress
pub fn progress(percent: f32, elapsed_secs: f64, total_secs: f64) {
    with_reporter(|r| r.progress_bar(percent, elapsed_secs, total_secs));
}

/// Clear progress bar
pub fn clear_progress() {
    with_reporter(|r| r.clear_progress_bar());
}

/// Log a message
pub fn log(level: LogLevel, message: &str) {
    with_reporter(|r| r.log(level, message));
}

/// Report `FFmpeg` command
pub fn ffmpeg_command(cmd_data: &str) {
    with_reporter(|r| r.ffmpeg_command(cmd_data));
}

// Debug variants - only show when debug logging is enabled

/// Output a processing step only if debug logging is enabled
pub fn processing_debug(message: &str) {
    if log_enabled!(Level::Debug) {
        processing(message);
    }
}

/// Output a status line only if debug logging is enabled
pub fn status_debug(label: &str, value: &str, highlight: bool) {
    if log_enabled!(Level::Debug) {
        status(label, value, highlight);
    }
}

/// Output general information only if debug logging is enabled
pub fn info_debug(message: &str) {
    if log_enabled!(Level::Debug) {
        crate::progress_reporting::info(message);
    }
}

/// Report encoding summary
pub fn encoding_summary(filename: &str, duration: Duration, input_size: u64, output_size: u64) {
    success("Encoding complete");
    status("File", filename, false);
    status(
        "Duration",
        &format!("{:.1}s", duration.as_secs_f64()),
        false,
    );
    status("Input size", &crate::format_bytes(input_size), false);
    status("Output size", &crate::format_bytes(output_size), false);

    let reduction = crate::utils::calculate_size_reduction(input_size, output_size);
    status("Reduction", &format!("{reduction}%"), reduction >= 50);
}

/// Report encode start
pub fn encode_start(input_path: &Path, output_path: &Path) {
    let filename = crate::utils::get_filename_safe(input_path)
        .unwrap_or_else(|_| input_path.display().to_string());

    processing(&format!("Encoding: {filename}"));
    debug(&format!("Output: {}", output_path.display()));
}

/// Report encode error
pub fn encode_error(input_path: &Path, message: &str) {
    let filename = crate::utils::get_filename_safe(input_path)
        .unwrap_or_else(|_| input_path.display().to_string());

    error(&format!("Error encoding {filename}: {message}"));
}

// Common progress reporting patterns

/// Reports the start of a processing operation.
/// Displays a processing indicator with the operation name.
pub fn report_processing_step(operation: &str) {
    processing(operation);
}

/// Reports the completion of an operation with a single result.
/// Shows a success message followed by the result status.
pub fn report_operation_complete(operation: &str, result_label: &str, result_value: &str) {
    success(&format!("{} complete", operation));
    status(result_label, result_value, false);
}

/// Reports multiple analysis results as status lines.
/// Each item is a tuple of (label, value, highlight).
pub fn report_analysis_results(items: &[(&str, String, bool)]) {
    for (label, value, highlight) in items {
        status(label, value, *highlight);
    }
}

/// Reports a configuration section with a header and multiple status items.
/// All items are displayed without highlighting.
pub fn report_configuration_section(title: &str, items: &[(&str, String)]) {
    section(title);
    for (label, value) in items {
        status(label, value, false);
    }
}

/// Reports the completion of an operation with timing information.
/// Shows a completion message with the duration.
pub fn report_timed_completion(operation: &str, filename: &str, duration: Duration) {
    success(&format!("{}: {}", operation, filename));
    status("Time", &crate::utils::format_duration(duration.as_secs_f64()), false);
}

/// Terminal-based implementation of ProgressReporter
pub struct TerminalProgressReporter;

impl Default for TerminalProgressReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalProgressReporter {
    pub fn new() -> Self {
        Self
    }
}

impl ProgressReporter for TerminalProgressReporter {
    fn output(&self, level: OutputLevel, text: &str) {
        match level {
            OutputLevel::Section => crate::terminal::print_section(text),
            OutputLevel::Subsection => crate::terminal::print_subsection(text),
            OutputLevel::Processing => crate::terminal::print_processing(text),
            OutputLevel::Status => crate::terminal::print_sub_item(text),
            OutputLevel::Success => crate::terminal::print_success(text),
            OutputLevel::Error => crate::terminal::print_error("Error", text, None),
            OutputLevel::Warning => crate::terminal::print_warning(text),
            OutputLevel::Debug => {
                // Only show debug in verbose mode
                if log_enabled!(Level::Debug) {
                    crate::terminal::print_sub_item(&format!("[debug] {}", text));
                }
            }
            OutputLevel::Info => crate::terminal::print_sub_item(text),
        }
    }

    fn output_status(&self, label: &str, value: &str, highlight: bool) {
        crate::terminal::print_status(label, value, highlight);
    }

    fn progress_bar(&self, percent: f32, elapsed_secs: f64, total_secs: f64) {
        crate::terminal::print_progress_bar(percent, elapsed_secs, total_secs, None, None, None);
    }

    fn clear_progress_bar(&self) {
        crate::terminal::clear_progress_bar();
    }

    fn log(&self, level: LogLevel, message: &str) {
        match level {
            LogLevel::Info => log::info!("{}", message),
            LogLevel::Warning => log::warn!("{}", message),
            LogLevel::Error => log::error!("{}", message),
            LogLevel::Debug => log::debug!("{}", message),
        }
    }

    fn ffmpeg_command(&self, cmd_data: &str) {
        log::debug!("FFmpeg command: {}", cmd_data);
    }
}
