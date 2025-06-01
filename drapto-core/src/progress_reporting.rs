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

pub mod ffmpeg_handler;

use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;


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

    let reduction = if input_size > 0 {
        ((input_size - output_size) as f64 / input_size as f64 * 100.0) as u64
    } else {
        0
    };
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
