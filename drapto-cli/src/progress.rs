// ============================================================================
// drapto-cli/src/progress.rs
// ============================================================================
//
// PROGRESS REPORTING: CLI-specific progress reporting
//
// This module provides CLI-specific implementations of the progress reporting
// abstractions defined in drapto-core. It handles formatting and displaying
// progress information in a user-friendly way in the terminal.
//
// KEY COMPONENTS:
// - CliProgressCallback: Implementation of ProgressCallback for CLI output
// - Terminal formatting utilities
//
// DESIGN PHILOSOPHY:
// This module separates the presentation concerns from the core library,
// allowing for different presentation styles without changing the core logic.
// It uses the colored crate for terminal formatting.
//
// AI-ASSISTANT-INFO: CLI-specific progress reporting implementation

// ---- External crate imports ----
use colored::*;
use drapto_core::{ProgressCallback, ProgressEvent, LogLevel};
use drapto_core::format_bytes;
use drapto_core::format_duration;
use log::{info, warn, error, debug};

// ---- Standard library imports ----


// ============================================================================
// CLI PROGRESS CALLBACK
// ============================================================================

/// Implementation of ProgressCallback for CLI output.
///
/// This struct provides a CLI-specific implementation of the ProgressCallback
/// trait, formatting progress events for display in the terminal.
#[derive(Debug, Clone, Default)]
pub struct CliProgressCallback {
    /// Whether to use verbose output
    pub verbose: bool,
}

impl CliProgressCallback {
    /// Creates a new CliProgressCallback with the specified verbosity.
    ///
    /// # Arguments
    ///
    /// * `verbose` - Whether to use verbose output
    ///
    /// # Returns
    ///
    /// * A new CliProgressCallback instance
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

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
}

impl ProgressCallback for CliProgressCallback {
    fn on_progress(&self, event: ProgressEvent) {
        match event {
            ProgressEvent::EncodeStart { input_path, output_path, using_hw_accel } => {
                // Extract filename for logging using to_string_lossy for consistent display
                let filename = input_path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| input_path.to_string_lossy().to_string());

                info!("Starting FFmpeg encode for: {}", filename.yellow());
                info!("  Output: {}", output_path.display());

                if using_hw_accel {
                    info!("  {} {}", "Hardware:".cyan(), "VideoToolbox hardware decoding enabled".green().bold());
                }
            },

            ProgressEvent::EncodeProgress { percent, current_secs, total_secs, speed, fps, eta } => {
                // Format the ETA
                let eta_str = if eta.as_secs() > 0 {
                    format_duration(eta)
                } else {
                    "< 1s".to_string()
                };

                // Format the current and total time
                let current_time = Self::format_duration_seconds(current_secs);
                let total_time = Self::format_duration_seconds(total_secs);

                info!(
                    "⏳ {} {:.2}% ({} / {}), Speed: {}, Avg FPS: {:.2}, ETA: {}",
                    "Encoding progress:".cyan(),
                    percent.to_string().green().bold(),
                    current_time.yellow(),
                    total_time.yellow(),
                    format!("{:.2}x", speed).green().bold(),
                    fps,
                    eta_str.green().bold()
                );
            },

            ProgressEvent::EncodeComplete { input_path, output_path: _, input_size, output_size, duration } => {
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

                info!("{}", filename.yellow().bold());
                info!("  {:<13} {}", "Encode time:".cyan(), format_duration(duration).green());
                info!("  {:<13} {}", "Input size:".cyan(), format_bytes(input_size).green());
                info!("  {:<13} {}", "Output size:".cyan(), format_bytes(output_size).green());
                info!("  {:<13} {}", "Reduced by:".cyan(), format!("{}%", reduction).green());
                info!("{}", "----------------------------------------".cyan());
            },

            ProgressEvent::EncodeError { input_path, message } => {
                // Extract filename for logging
                let filename = input_path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| input_path.to_string_lossy().to_string());

                error!("Error encoding {}: {}", filename.red().bold(), message);
            },

            ProgressEvent::HardwareAcceleration { available, acceleration_type } => {
                if available {
                    info!("{} {}", "Hardware:".cyan(), format!("{} hardware decoding available", acceleration_type).green().bold());
                } else {
                    info!("{} {}", "Hardware:".cyan(), "Using software decoding (hardware acceleration not available on this platform)".yellow());
                }
            },

            ProgressEvent::LogMessage { message, level } => {
                match level {
                    LogLevel::Debug => {
                        if self.verbose {
                            debug!("{}", message);
                        }
                    },
                    LogLevel::Info => info!("{}", message),
                    LogLevel::Warning => warn!("{}", message),
                    LogLevel::Error => error!("{}", message),
                }
            },
        }
    }
}
