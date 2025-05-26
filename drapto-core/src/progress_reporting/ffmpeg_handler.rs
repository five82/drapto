// ============================================================================
// drapto-core/src/progress_reporting/ffmpeg_handler.rs
// ============================================================================
//
// FFMPEG PROGRESS HANDLER: Structured FFmpeg event handling with progress reporting
//
// This module provides a structured way to handle FFmpeg events and report
// progress using the existing ProgressReporter trait.
//
// AI-ASSISTANT-INFO: FFmpeg-specific progress handling implementation

use ffmpeg_sidecar::event::{FfmpegEvent, FfmpegProgress, LogLevel as FfmpegLogLevel};
use std::time::{Duration, Instant};
use crate::error::CoreResult;
use crate::progress_reporting::LogLevel;

/// Handler for FFmpeg progress events
pub struct FfmpegProgressHandler {
    duration: Option<f64>,
    start_time: Instant,
    last_progress_percent: f64,
    last_log_time: Instant,
    last_logged_percent_threshold: i32,
    is_grain_sample: bool,
    stderr_buffer: String,
}

impl FfmpegProgressHandler {
    /// Creates a new FFmpeg progress handler
    pub fn new(duration: Option<f64>, is_grain_sample: bool) -> Self {
        Self {
            duration,
            start_time: Instant::now(),
            last_progress_percent: -3.0,
            last_log_time: Instant::now(),
            last_logged_percent_threshold: -1,
            is_grain_sample,
            stderr_buffer: String::new(),
        }
    }
    
    /// Handles an FFmpeg event
    pub fn handle_event(&mut self, event: FfmpegEvent) -> CoreResult<()> {
        match event {
            FfmpegEvent::Progress(progress) => self.handle_progress(progress),
            FfmpegEvent::Log(level, message) => self.handle_log(level, &message),
            FfmpegEvent::Error(error) => self.handle_error(&error),
            _ => {} // Ignore other events
        }
        Ok(())
    }
    
    /// Gets the accumulated stderr buffer
    pub fn stderr_buffer(&self) -> &str {
        &self.stderr_buffer
    }
    
    /// Handles progress events
    fn handle_progress(&mut self, progress: FfmpegProgress) {
        if self.is_grain_sample {
            // Don't report progress for grain samples
            return;
        }
        
        // Parse time from progress
        let current_secs = parse_ffmpeg_time(&progress.time).unwrap_or(0.0);
        let percent = self.duration
            .filter(|&d| d > 0.0)
            .map(|d| (current_secs / d * 100.0).min(100.0))
            .unwrap_or(0.0);
        
        // Only report if progress changed significantly
        if percent >= self.last_progress_percent + 3.0 || 
           (percent >= 100.0 && self.last_progress_percent < 100.0) {
            
            // Calculate ETA
            let eta = self.calculate_eta(current_secs, progress.speed);
            
            // Calculate average encoding FPS
            let elapsed = self.start_time.elapsed().as_secs_f64();
            let avg_fps = if elapsed > 0.01 {
                progress.frame as f64 / elapsed
            } else {
                0.0
            };
            
            // Report progress
            crate::progress_reporting::report_encode_progress(
                percent as f32,
                current_secs,
                self.duration.unwrap_or(0.0),
                progress.speed,
                avg_fps as f32,
                Duration::from_secs_f64(eta),
            );
            
            // Log progress to file at regular intervals
            self.log_progress_if_needed(percent, current_secs, progress.speed, avg_fps, eta);
            
            self.last_progress_percent = percent;
        }
    }
    
    /// Handles log events
    fn handle_log(&mut self, level: FfmpegLogLevel, message: &str) {
        // Filter out noisy NAL unit messages
        if message.contains("Skipping NAL unit") {
            return;
        }
        
        // Map FFmpeg log level
        let log_level = map_ffmpeg_log_level(&level);
        
        // Special handling for encoder messages
        if message.starts_with("Svt[info]:") && !self.is_grain_sample {
            crate::progress_reporting::report_encoder_message(message, self.is_grain_sample);
        } else if log_level == log::Level::Info {
            // Downgrade info to debug for cleaner output
            log::debug!(target: "ffmpeg_log", "{}", message);
        } else {
            log::log!(target: "ffmpeg_log", log_level, "{}", message);
        }
    }
    
    /// Handles error events
    fn handle_error(&mut self, error: &str) {
        let is_non_critical = is_non_critical_ffmpeg_error(error);
        
        if is_non_critical {
            log::debug!("ffmpeg non-critical message: {}", error);
        } else {
            crate::progress_reporting::report_log_message(
                &format!("ffmpeg stderr error: {}", error),
                LogLevel::Error
            );
        }
        
        // Always capture errors in buffer
        self.stderr_buffer.push_str(&format!("{}\n", error));
    }
    
    /// Calculates ETA based on current progress
    fn calculate_eta(&self, current_secs: f64, speed: f32) -> f64 {
        if let Some(total) = self.duration {
            if speed > 0.01 && total > current_secs {
                (total - current_secs) / (speed as f64)
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
    
    /// Logs progress at regular intervals for daemon mode
    fn log_progress_if_needed(
        &mut self,
        percent: f64,
        current_secs: f64,
        speed: f32,
        avg_fps: f64,
        eta_seconds: f64,
    ) {
        let current_threshold = (percent as i32 / 10) * 10;
        let should_log = !self.is_grain_sample && (
            // Log when crossing 10% thresholds
            (current_threshold > self.last_logged_percent_threshold && current_threshold >= 10) ||
            // Log at start
            (percent >= 0.0 && self.last_logged_percent_threshold < 0) ||
            // Log at completion
            percent >= 100.0 ||
            // Log every 5 minutes
            self.last_log_time.elapsed() >= Duration::from_secs(300)
        );
        
        if should_log {
            log::info!(
                target: "drapto::progress",
                "Encoding progress: {:.1}% complete | Time: {} / {} | Speed: {:.2}x | FPS: {:.1} | ETA: {}",
                percent,
                crate::utils::format_duration_seconds(current_secs),
                crate::utils::format_duration_seconds(self.duration.unwrap_or(0.0)),
                speed,
                avg_fps,
                crate::utils::format_duration_seconds(eta_seconds)
            );
            self.last_log_time = Instant::now();
            self.last_logged_percent_threshold = current_threshold;
        }
    }
}

/// Maps FFmpeg log level to Rust log level
fn map_ffmpeg_log_level(level: &FfmpegLogLevel) -> log::Level {
    match level {
        FfmpegLogLevel::Fatal | FfmpegLogLevel::Error => log::Level::Error,
        FfmpegLogLevel::Warning => log::Level::Warn,
        FfmpegLogLevel::Info => log::Level::Info,
        _ => log::Level::Trace,
    }
}

/// Determines if an FFmpeg error message is non-critical
/// 
/// These are FFmpeg messages that appear in stderr but don't indicate actual problems:
/// - "deprecated pixel format" - FFmpeg warning about legacy pixel formats
/// - "No accelerated colorspace conversion" - Hardware acceleration info, not an error
/// - "Stream map" - Stream mapping information, not an error
/// - "automatically inserted filter" - FFmpeg adding filters automatically
/// - "Timestamps are unset" - Common with certain codecs, FFmpeg handles it
/// - "does not match the corresponding codec" - Codec capability warnings
/// - "Queue input is backward" - Frame ordering warnings that FFmpeg handles
/// - "No streams found" - Spurious error from hqdn3d filter that doesn't affect encoding
fn is_non_critical_ffmpeg_error(error: &str) -> bool {
    error.contains("deprecated pixel format")
        || error.contains("No accelerated colorspace conversion")
        || error.contains("Stream map")
        || error.contains("automatically inserted filter")
        || error.contains("Timestamps are unset")
        || error.contains("does not match the corresponding codec")
        || error.contains("Queue input is backward")
        || error.contains("No streams found")  // hqdn3d filter spurious error
}

/// Parses FFmpeg time string (HH:MM:SS.MS format) to seconds
fn parse_ffmpeg_time(time: &str) -> Option<f64> {
    let parts: Vec<&str> = time.split(':').collect();
    if parts.len() == 3 {
        let hours = parts[0].parse::<f64>().ok()?;
        let minutes = parts[1].parse::<f64>().ok()?;
        let seconds = parts[2].parse::<f64>().ok()?;
        Some(hours * 3600.0 + minutes * 60.0 + seconds)
    } else {
        None
    }
}

