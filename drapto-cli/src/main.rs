//! Main entry point for the Drapto CLI application.
//!
//! This handles command-line argument parsing, logging setup, and dispatching
//! to the appropriate command handlers. The application now always runs in
//! foreground mode, emitting either a terminal UI or JSON progress events.

use drapto::commands::encode::discover_encode_files;
use drapto::error::CliResult;
use drapto::logging::get_timestamp;
use drapto::{Commands, parse_cli, run_encode};
use drapto_core::CoreError;
use drapto_core::file_logging::setup::setup_file_logging;
use drapto_core::reporting::{JsonReporter, Reporter, TerminalReporter};

use log::LevelFilter;
use std::path::PathBuf;

/// Main entry point with clean separation of concerns
fn main() -> CliResult<()> {
    let cli_args = parse_cli();

    // Determine log level based on verbose flag
    let log_level = if cli_args.verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    let _ = match cli_args.command {
        Commands::Encode(args) => {
            let (discovered_files, effective_input_dir) =
                discover_encode_files(&args).map_err(|e| {
                    CoreError::OperationFailed(format!("Error during file discovery: {}", e))
                })?;

            // Calculate log path
            let (actual_output_dir, target_filename_override_os) =
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

            // Log startup information
            log::info!("Drapto encoder starting in foreground mode");

            if log_level == LevelFilter::Debug {
                log::info!("Debug level logging enabled");
            }

            let reporter: Box<dyn Reporter> = if args.progress_json {
                Box::new(JsonReporter::new())
            } else {
                Box::new(TerminalReporter::new())
            };

            // Update args to use the calculated actual output directory
            let mut corrected_args = args.clone();
            corrected_args.output_dir = actual_output_dir;

            run_encode(
                corrected_args,
                discovered_files,
                effective_input_dir,
                target_filename_override_os,
                reporter.as_ref(),
            )
        }
    };

    Ok(())
}
