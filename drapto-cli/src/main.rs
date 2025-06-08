//! Main entry point for the Drapto CLI application.
//!
//! This handles command-line argument parsing, logging setup, daemonization,
//! and dispatching to the appropriate command handlers. The application can run
//! in either interactive mode (with terminal output) or daemon mode (background process).

use drapto_cli::commands::encode::discover_encode_files;
use drapto_cli::error::CliResult;
use drapto_cli::logging::get_timestamp;
use drapto_cli::{Cli, Commands, run_encode};

use clap::Parser;
use daemonize::Daemonize;
use drapto_core::CoreError;
use drapto_core::events::{Event, EventDispatcher};
use drapto_core::file_logging::{setup::setup_file_logging, FileLoggingHandler};
use drapto_core::presentation::template_event_handler::TemplateEventHandler;
use drapto_core::notifications::{NotificationSender, NtfyNotificationSender};

use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;
use log::LevelFilter;

/// Main entry point with clean separation of concerns
fn main() -> CliResult<()> {
    let cli_args = Cli::parse();
    let interactive_mode = cli_args.interactive;

    // Determine log level based on verbose flag
    let log_level = if cli_args.verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    let _ = match cli_args.command {
        Commands::Encode(args) => {
            let (discovered_files, effective_input_dir) =
                discover_encode_files(&args).map_err(|e| 
                    CoreError::OperationFailed(format!("Error during file discovery: {}", e))
                )?;

            // Calculate log path
            let (actual_output_dir, _target_filename_override_os) =
                if discovered_files.len() == 1 && args.output_dir.extension().is_some() {
                    let target_file = args.output_dir.clone();
                    let parent_dir = target_file
                        .parent()
                        .map(std::path::Path::to_path_buf)
                        .filter(|p| !p.as_os_str().is_empty())
                        .unwrap_or_else(|| PathBuf::from("."));
                    let filename_os = target_file.file_name().map(std::ffi::OsStr::to_os_string);
                    (parent_dir, filename_os)
                } else {
                    (args.output_dir.clone(), None)
                };

            let log_dir = args
                .log_dir
                .clone()
                .unwrap_or_else(|| actual_output_dir.join("logs"));

            let main_log_filename = format!("drapto_encode_run_{}.log", get_timestamp());
            let main_log_path = log_dir.join(&main_log_filename);

            // Create log directory
            std::fs::create_dir_all(&log_dir).map_err(|e| {
                CoreError::OperationFailed(format!(
                    "Failed to create log directory: {}: {}",
                    log_dir.display(),
                    e
                ))
            })?;

            // Set up file logging for both modes
            setup_file_logging(&main_log_path, log_level).map_err(|e| {
                CoreError::OperationFailed(format!(
                    "Failed to set up file logging to {}: {}",
                    main_log_path.display(),
                    e
                ))
            })?;

            // Create event dispatcher
            let mut event_dispatcher = EventDispatcher::new();
            
            // Always add file logging handler
            event_dispatcher.add_handler(Arc::new(FileLoggingHandler::new()));
            
            // Add terminal handler only in interactive mode
            if interactive_mode {
                event_dispatcher.add_handler(Arc::new(TemplateEventHandler::new()));
            }

            if !interactive_mode {
                // Pre-daemonization output
                eprintln!("===== DAEMON MODE =====");
                eprintln!();
                eprintln!("Files to process: {}", discovered_files.len());
                for (i, file) in discovered_files.iter().enumerate() {
                    eprintln!("  {}. {}", i + 1, file.display());
                }
                eprintln!();
                eprintln!("Log file: {}", main_log_path.display());
                eprintln!();
                eprintln!("Starting daemon process...");

                if let Err(e) = io::stderr().flush() {
                    eprintln!("Failed to flush stderr: {}", e);
                }

                // Open log file for daemon stdout/stderr redirection
                let log_file = std::fs::File::create(&main_log_path).map_err(|e| {
                    CoreError::OperationFailed(format!(
                        "Failed to create log file: {}: {}",
                        main_log_path.display(),
                        e
                    ))
                })?;

                let log_file_stderr = log_file.try_clone().map_err(|e| {
                    CoreError::OperationFailed(format!("Failed to clone log file handle: {e}"))
                })?;

                let daemonize = Daemonize::new()
                    .working_directory(".")
                    .stdout(log_file)
                    .stderr(log_file_stderr);
                    
                daemonize.start().map_err(|e| {
                    CoreError::OperationFailed(format!("Failed to start daemon process: {e}"))
                })?;
            }

            // Create notification sender if provided
            let notification_sender = if let Some(ref topic) = args.ntfy {
                match NtfyNotificationSender::new(topic) {
                    Ok(sender) => Some(sender),
                    Err(e) => {
                        event_dispatcher.emit(Event::Warning {
                            message: format!("Failed to create notification sender: {}", e),
                        });
                        None
                    }
                }
            } else {
                None
            };

            // Log startup information
            log::info!("Drapto encoder starting in {} mode", 
                if interactive_mode { "interactive" } else { "daemon" });
            
            if log_level == LevelFilter::Debug {
                log::info!("Debug level logging enabled");
            }

            run_encode(
                notification_sender.as_ref().map(|s| s as &dyn NotificationSender),
                args,
                interactive_mode,
                discovered_files,
                effective_input_dir,
                event_dispatcher,
            )
        }
    };

    Ok(())
}