//! Centralized logging configuration for Drapto
//!
//! This module handles:
//! - Setting up proper console logging with formatting
//! - Configuring file-based logging with rotation
//! - Managing log levels and output destinations
//! - Coordinating logging across all pipeline components
//! - Providing utility functions for log formatting and structure
//!
//! The logging configuration ensures consistent formatting and proper
//! log aggregation throughout the encoding pipeline.

use log::{debug, info, warn, LevelFilter};
use std::io::Write;
use std::process::Command;
use std::fs;
use std::path::Path;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

/// Initialize the logger for drapto
///
/// Sets up an env_logger with appropriate formatting and log level
pub fn init(verbose: bool) {
    let level = if verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    
    init_with_level(level, verbose);
}

/// Initialize the logger with a specific log level
///
/// Sets up an env_logger with appropriate formatting and the specified log level
pub fn init_with_level(level: LevelFilter, _verbose: bool) {
    env_logger::Builder::new()
        .format(|buf, record| {
            let timestamp = buf.timestamp();
            let level_str = match record.level() {
                log::Level::Error => "ERROR",
                log::Level::Warn => "WARN ",
                log::Level::Info => "INFO ",
                log::Level::Debug => "DEBUG",
                log::Level::Trace => "TRACE",
            };
            
            let level_colored = match record.level() {
                log::Level::Error => level_str.bright_red(),
                log::Level::Warn => level_str.yellow(),
                log::Level::Info => level_str.green(),
                log::Level::Debug => level_str.blue(),
                log::Level::Trace => level_str.magenta(),
            };
            
            writeln!(
                buf,
                "{} {} {}",
                timestamp.to_string().white(),
                level_colored,
                record.args()
            )
        })
        .filter(None, level)
        .init();
    
    debug!("Logger initialized with level: {}", level);
}

/// Initialize a file logger for a specific encoding session
///
/// Creates a log file and returns a file handle to it
pub fn init_file_logger<P: AsRef<Path>>(file_name: P) -> std::io::Result<fs::File> {
    // Make sure the parent directory exists
    if let Some(parent) = file_name.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    
    let file = fs::File::create(file_name)?;
    Ok(file)
}

/// Log an encoding progress update with a rich progress bar
pub fn log_progress(stage: &str, progress: f32) {
    if !(0.0..=1.0).contains(&progress) {
        warn!("Invalid progress value: {}", progress);
        return;
    }
    
    let percentage = (progress * 100.0) as u8;
    
    // Create a temporary progress bar (not persistent)
    let pb = ProgressBar::new(100);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} {msg} [{bar:40.cyan/blue}] {percent}%")
        .unwrap()
        .progress_chars("█▓▒░ "));
    
    pb.set_message(stage.bright_green().to_string());
    pb.set_position(percentage.into());
    
    // Let the progress bar render once
    pb.tick();
    
    // After rendering, we abandon the progress bar and just log the result
    pb.finish_and_clear();
    
    info!("{}: {}% complete", stage.bright_green(), percentage);
}

/// Create a section heading in the logs to separate different processing stages
pub fn log_section(title: &str) {
    info!("");
    info!("{}", "=".repeat(50).bright_blue());
    info!("{}", title.bold().bright_white());
    info!("{}", "=".repeat(50).bright_blue());
    info!("");
}

/// Log a subsection heading
pub fn log_subsection(title: &str) {
    info!("");
    info!("{}", "-".repeat(40).blue());
    info!("{}", title.bold().white());
    info!("{}", "-".repeat(40).blue());
}

/// Log a status message with a colored icon
pub fn log_status(status: &str, message: &str) {
    let (icon, colored_status) = match status.to_lowercase().as_str() {
        "success" | "completed" | "done" => ("✅", status.bright_green().bold()),
        "error" | "failed" => ("❌", status.bright_red().bold()),
        "warning" => ("⚠️ ", status.yellow().bold()),
        "info" => ("ℹ️ ", status.bright_cyan().bold()),
        "processing" | "running" => ("🔄", status.bright_blue().bold()),
        _ => ("•", status.white()),
    };
    
    info!("{} {} {}", icon, colored_status, message);
}

/// Log a command being executed
pub fn log_command(cmd: &Command) {
    let program = cmd.get_program().to_string_lossy();
    let args: Vec<_> = cmd.get_args().map(|arg| arg.to_string_lossy()).collect();
    
    debug!("Executing command: {} {}", program.cyan(), args.join(" ").blue());
}

/// Log memory usage statistics
pub fn log_memory_stats(total_mb: u64, available_mb: u64, percentage: f32, count: usize, max: usize, encoder: &str) {
    let percentage_str = format!("{:.1}%", percentage * 100.0);
    let styled_percentage = if percentage < 0.5 {
        percentage_str.green()
    } else if percentage < 0.8 {
        percentage_str.yellow()
    } else {
        percentage_str.red()
    };
    
    let count_str = format!("{}/{}", count, max);
    let ratio = count as f32 / (max as f32);
    let styled_count = if ratio < 0.5 {
        count_str.green()
    } else if ratio < 0.8 {
        count_str.yellow()
    } else {
        count_str.red()
    };
    
    info!(
        "{} Total: {}MB, Available: {}MB, Used: {}, Safe job count: {} ({})",
        "Memory status:".magenta().bold(),
        total_mb.to_string().cyan(),
        available_mb.to_string().cyan(),
        styled_percentage,
        styled_count,
        encoder.white().italic()
    );
}

/// Log segment encoding completion
pub fn log_segment_completion(
    segment_name: &str, 
    duration_s: f64, 
    size_mb: f64, 
    bitrate_kbps: u64,
    encode_time_s: f64,
    realtime_factor: f64,
    resolution: &str,
    vmaf: Option<f64>
) {
    log_subsection(&format!("Segment encoding complete: {}", segment_name.white().bold()));
    
    // Format the duration
    info!("   {}: {:.2}s", "Duration".yellow(), duration_s);
    
    // Format the size with color based on efficiency (lower is better)
    let size_styled = if size_mb < 2.0 {
        format!("{:.2} MB", size_mb).green()
    } else if size_mb < 5.0 {
        format!("{:.2} MB", size_mb).yellow()
    } else {
        format!("{:.2} MB", size_mb).red()
    };
    info!("   {}: {}", "Size".yellow(), size_styled);
    
    // Format the bitrate with color based on efficiency (lower is better)
    let bitrate_styled = if bitrate_kbps < 5000 {
        format!("{} kbps", bitrate_kbps).green()
    } else if bitrate_kbps < 10000 {
        format!("{} kbps", bitrate_kbps).yellow()
    } else {
        format!("{} kbps", bitrate_kbps).red()
    };
    info!("   {}: {}", "Bitrate".yellow(), bitrate_styled);
    
    // Format the encoding time and realtime factor
    let rt_factor_styled = if realtime_factor > 1.5 {
        format!("{:.2}x realtime", realtime_factor).green()
    } else if realtime_factor > 0.8 {
        format!("{:.2}x realtime", realtime_factor).yellow()
    } else {
        format!("{:.2}x realtime", realtime_factor).red()
    };
    info!("   {}: {:.2}s ({})", "Encoding time".yellow(), encode_time_s, rt_factor_styled);
    
    // Resolution
    info!("   {}: {}", "Resolution".yellow(), resolution);
    
    // VMAF scores if available
    if let Some(vmaf_score) = vmaf {
        let vmaf_styled = if vmaf_score >= 95.0 {
            format!("{:.2}", vmaf_score).green()
        } else if vmaf_score >= 90.0 {
            format!("{:.2}", vmaf_score).yellow()
        } else {
            format!("{:.2}", vmaf_score).red()
        };
        info!("   {}: {}", "VMAF score".yellow(), vmaf_styled);
    } else {
        info!("   {}: {}", "VMAF scores".yellow(), "No VMAF scores available".dimmed());
    }
}