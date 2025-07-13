//! Encode command implementation using event-based architecture

use crate::error::CliResult;
use crate::EncodeArgs;

use drapto_core::{
    CoreConfig, CoreError,
    discovery::find_processable_files,
    events::{Event, EventDispatcher},
    notifications::NotificationSender,
    processing::process_videos,
    utils::calculate_size_reduction,
};

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Discover video files to encode based on the provided arguments
pub fn discover_encode_files(args: &EncodeArgs) -> CliResult<(Vec<PathBuf>, PathBuf)> {
    let input_path = &args.input_path;
    
    if input_path.is_file() {
        // Single file input
        let effective_input_dir = input_path.parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        Ok((vec![input_path.clone()], effective_input_dir))
    } else if input_path.is_dir() {
        // Directory input - find all processable files
        let files = find_processable_files(input_path)?;
        Ok((files, input_path.clone()))
    } else {
        Err(CoreError::PathError(
            format!("Input path does not exist: {}", input_path.display())
        ))
    }
}

/// Run the encode command with the event-based architecture
pub fn run_encode(
    notification_sender: Option<&dyn NotificationSender>,
    args: EncodeArgs,
    interactive_mode: bool,
    discovered_files: Vec<PathBuf>,
    effective_input_dir: PathBuf,
    event_dispatcher: EventDispatcher,
) -> CliResult<()> {
    // PID file setup for daemon mode
    let pid_file_path = if !interactive_mode {
        let pid_filename = format!(
            "drapto_encode_pid_{}.txt",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        );
        let log_dir = args
            .log_dir
            .clone()
            .unwrap_or_else(|| args.output_dir.join("logs"));
        let pid_path = log_dir.join(pid_filename);
        
        // Write PID file
        if let Ok(mut pid_file) = std::fs::File::create(&pid_path) {
            let _ = writeln!(pid_file, "{}", std::process::id());
            log::info!("PID file created: {}", pid_path.display());
        }
        
        Some(pid_path)
    } else {
        None
    };

    // Notify about startup
    if let Some(sender) = notification_sender {
        let hostname = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("COMPUTERNAME"))
            .unwrap_or_else(|_| "local".to_string());
            
        let _ = sender.send(&format!(
            "Drapto encoding started on {} - Processing {} files",
            hostname,
            discovered_files.len()
        ));
    }

    let start_time = SystemTime::now();

    // Convert args to CoreConfig
    let mut config = CoreConfig::new(
        effective_input_dir.clone(),
        args.output_dir.clone(),
        args.log_dir.clone().unwrap_or_else(|| args.output_dir.join("logs")),
    );

    // Apply command line arguments to config
    config.enable_denoise = !args.no_denoise;

    if let Some(quality) = args.quality_sd {
        config.quality_sd = quality;
    }

    if let Some(quality) = args.quality_hd {
        config.quality_hd = quality;
    }

    if let Some(quality) = args.quality_uhd {
        config.quality_uhd = quality;
    }

    config.crop_mode = if args.disable_autocrop {
        "none".to_string()
    } else {
        drapto_core::config::DEFAULT_CROP_MODE.to_string()
    };

    if let Some(topic) = args.ntfy.clone() {
        config.ntfy_topic = Some(topic);
    }

    if let Some(preset) = args.preset {
        config.svt_av1_preset = preset;
    }

    // Validate configuration
    config.validate()?;

    let results = if discovered_files.is_empty() {
        event_dispatcher.emit(Event::Warning {
            message: "No video files found to process".to_string(),
        });
        vec![]
    } else {
        // Process videos with the new event-based system
        match process_videos(
            notification_sender,
            &config,
            &discovered_files,
            None,
            Some(&event_dispatcher),
        ) {
            Ok(results) => results,
            Err(e) => {
                event_dispatcher.emit(Event::Error {
                    title: "Processing failed".to_string(),
                    message: e.to_string(),
                    context: None,
                    suggestion: None,
                });
                return Err(e);
            }
        }
    };

    // Summary
    let elapsed = start_time.elapsed().unwrap_or_default();
    let total_duration = format!(
        "{:02}:{:02}:{:02}",
        elapsed.as_secs() / 3600,
        (elapsed.as_secs() % 3600) / 60,
        elapsed.as_secs() % 60
    );

    if !results.is_empty() {
        log::info!("");
        log::info!("===== BATCH SUMMARY =====");
        log::info!("");
        log::info!("Total files processed: {}", results.len());
        log::info!("Total time: {}", total_duration);
        
        let total_input_size: u64 = results.iter().map(|r| r.input_size).sum();
        let total_output_size: u64 = results.iter().map(|r| r.output_size).sum();
        let total_reduction = calculate_size_reduction(total_input_size, total_output_size) as f64;
        
        log::info!(
            "Total size reduction: {:.1}% ({} → {})",
            total_reduction,
            drapto_core::format_bytes(total_input_size),
            drapto_core::format_bytes(total_output_size)
        );
    }

    // Send completion notification
    if let Some(sender) = notification_sender {
        let hostname = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("COMPUTERNAME"))
            .unwrap_or_else(|_| "local".to_string());
            
        let message = if results.is_empty() {
            format!("Drapto encoding on {} completed - No files processed", hostname)
        } else {
            format!(
                "Drapto encoding on {} completed - {} files processed in {}",
                hostname,
                results.len(),
                total_duration
            )
        };
        
        let _ = sender.send(&message);
    }

    // Clean up PID file
    if let Some(pid_path) = pid_file_path {
        if let Err(e) = std::fs::remove_file(&pid_path) {
            log::warn!("Failed to remove PID file: {}", e);
        }
    }

    Ok(())
}