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

    /// Finish progress bar (leave final state visible)
    fn finish_progress_bar(&self);

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

/// Finish progress bar (leave final state visible)
pub fn finish_progress() {
    with_reporter(|r| r.finish_progress_bar());
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

// Consolidated reporting functions for reducing redundancy

/// Report video analysis in a consolidated format
pub fn report_video_analysis(filename: &str, video_width: u32, video_height: u32, duration_secs: f64, category: &str, is_hdr: bool, audio_channels: &[u32]) {
    section("VIDEO INFO");
    status("File", filename, false);
    status("Resolution", &format!("{}x{} ({})", video_width, video_height, category), category == "UHD");
    status("Duration", &crate::utils::format_duration(duration_secs), duration_secs > 3600.0);
    status("Dynamic range", if is_hdr { "HDR" } else { "SDR" }, is_hdr);
    
    // Add audio information
    if !audio_channels.is_empty() {
        let audio_summary = if audio_channels.len() == 1 {
            match audio_channels[0] {
                1 => "Mono".to_string(),
                2 => "Stereo".to_string(),
                6 => "5.1 surround".to_string(),
                8 => "7.1 surround".to_string(),
                ch => format!("{} channels", ch),
            }
        } else {
            let track_descriptions: Vec<String> = audio_channels.iter().map(|&ch| match ch {
                1 => "Mono".to_string(),
                2 => "Stereo".to_string(),
                6 => "5.1".to_string(),
                8 => "7.1".to_string(),
                ch => format!("{}ch", ch),
            }).collect();
            format!("{} tracks ({})", audio_channels.len(), track_descriptions.join(", "))
        };
        status("Audio", &audio_summary, audio_channels.iter().any(|&ch| ch >= 6));
    } else {
        status("Audio", "None detected", false);
    }
}

/// Report consolidated encoding configuration without duplication
pub fn report_encoding_configuration(quality: u32, preset: u8, tune: u8, audio_channels: &[u32], hqdn3d_params: Option<&str>) {
    // Debug logging for detailed information
    log::debug!("Encoding configuration - Quality: {}, Preset: {}, Tune: {}, Audio channels: {:?}, Denoising: {:?}", 
                quality, preset, tune, audio_channels, hqdn3d_params);
    
    section("ENCODING CONFIGURATION");
    
    // Video settings grouped together
    status("Encoder", "SVT-AV1", false);
    status("Quality (CRF)", &quality.to_string(), false);
    status("Preset", &preset.to_string(), false);
    status("Tune", &tune.to_string(), false);  // SVT-AV1 tune parameter
    
    // Audio settings
    if !audio_channels.is_empty() {
        status("Audio codec", "Opus", false);
        
        // Calculate bitrates for each stream
        let bitrates: Vec<u32> = audio_channels.iter().map(|&ch| crate::processing::audio::calculate_audio_bitrate(ch)).collect();
        
        if audio_channels.len() == 1 {
            // Single audio stream - show channel count and bitrate on one line
            let ch_name = match audio_channels[0] {
                1 => "Mono".to_string(),
                2 => "Stereo".to_string(),
                6 => "5.1".to_string(),
                8 => "7.1".to_string(),
                ch => format!("{} channels", ch),
            };
            status("Audio", &format!("{} @ {}kbps", ch_name, bitrates[0]), bitrates[0] >= 256);
        } else {
            // Multiple audio streams - show count summary first
            status("Audio streams", &format!("{} tracks", audio_channels.len()), false);
            
            // Then show each track's details
            for (i, (&channels, &bitrate)) in audio_channels.iter().zip(bitrates.iter()).enumerate() {
                let ch_name = match channels {
                    1 => "Mono".to_string(),
                    2 => "Stereo".to_string(),
                    6 => "5.1".to_string(),
                    8 => "7.1".to_string(),
                    ch => format!("{}ch", ch),
                };
                status(&format!("  Track {}", i + 1), &format!("{} @ {}kbps", ch_name, bitrate), bitrate >= 256);
            }
        }
    } else {
        status("Audio", "None", false);
    }
    
    // Processing options
    if let Some(params) = hqdn3d_params {
        status("Denoising", &format!("{} (HQDN3D)", params), false);
        status("Film grain", &format!("{} (synthesis)", crate::config::FIXED_FILM_GRAIN_VALUE), false);
    } else {
        status("Denoising", "Disabled", false);
        status("Film grain", "0 (disabled)", false);
    }
}

/// Report final results without repeating file information
pub fn report_final_results(duration: Duration, input_size: u64, output_size: u64) {
    section("ENCODING RESULTS");
    
    // Group related metrics together
    status("Time elapsed", &crate::utils::format_duration(duration.as_secs_f64()), duration.as_secs() > 3600);
    status("Input size", &crate::utils::format_bytes(input_size), input_size > 1024*1024*1024);
    status("Output size", &crate::utils::format_bytes(output_size), false);
    
    let reduction = crate::utils::calculate_size_reduction(input_size, output_size);
    status("Size reduction", &format!("{}%", reduction), reduction >= 50);
}

/// Report consolidated batch summary for multiple files
pub fn report_batch_summary(results: &[crate::EncodeResult], total_duration: std::time::Duration) {
    if results.len() <= 1 {
        // Single file or no files - no additional summary needed
        return;
    }
    
    section("BATCH ENCODING SUMMARY");
    
    // Summary stats at the top
    status("Files encoded", &results.len().to_string(), false);
    status("Total time", &crate::utils::format_duration(total_duration.as_secs_f64()), total_duration.as_secs() > 3600);
    
    let total_input: u64 = results.iter().map(|r| r.input_size).sum();
    let total_output: u64 = results.iter().map(|r| r.output_size).sum();
    let total_reduction = crate::utils::calculate_size_reduction(total_input, total_output);
    
    status("Total input", &crate::utils::format_bytes(total_input), total_input > 1024*1024*1024*10);
    status("Total output", &crate::utils::format_bytes(total_output), false);
    status("Reduction", &format!("{}% overall", total_reduction), total_reduction >= 50);
    
    // Individual file results
    info("");
    with_reporter(|r| r.output(OutputLevel::Subsection, "Individual Results"));
    
    for result in results {
        // More compact display for individual files
        let reduction = crate::utils::calculate_size_reduction(result.input_size, result.output_size);
        info(&format!("  {} - {} → {} (-{}%) in {}", 
            result.filename,
            crate::utils::format_bytes(result.input_size),
            crate::utils::format_bytes(result.output_size),
            reduction,
            crate::utils::format_duration(result.duration.as_secs_f64())
        ));
    }
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
            OutputLevel::Processing => crate::terminal::print_processing_no_spacing(text),
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

    fn finish_progress_bar(&self) {
        crate::terminal::finish_progress_bar();
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
