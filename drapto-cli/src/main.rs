// ============================================================================
// drapto-cli/src/main.rs
// ============================================================================
//
// MAIN ENTRY POINT: Drapto CLI Application
//
// This file contains the main entry point for the Drapto CLI application, which
// is a video encoding tool that uses ffmpeg to convert video files to AV1 format.
// It handles command-line argument parsing, logging setup, daemonization, and
// dispatching to the appropriate command handlers.
//
// KEY COMPONENTS:
// - Command-line argument parsing (via clap)
// - Logging configuration (via env_logger)
// - Daemonization support (via daemonize)
// - Error handling and reporting
//
// ARCHITECTURE:
// The application follows a modular design where:
// 1. Main parses arguments and sets up the environment
// 2. Command handlers (like run_encode) implement specific functionality
// 3. Core logic is delegated to the drapto-core library
//
// AI-ASSISTANT-INFO: Entry point for CLI application, handles arg parsing and command dispatch

// ---- Internal crate imports ----
use drapto_cli::commands::encode::discover_encode_files;
use drapto_cli::logging::{get_timestamp, setup_file_logging};
use drapto_cli::terminal;
use drapto_cli::{Cli, Commands, run_encode};

// ---- External crate imports ----
use anyhow::{Context, Result};
use clap::Parser;
use daemonize::Daemonize;
use drapto_core::notifications::NtfyNotificationSender;

// ---- Standard library imports ----
use std::io::{self, Write};
use std::path::PathBuf;

// ---- Logging imports ----
use env_logger::Env;
use log::Level;

/// Main entry point for the Drapto CLI application.
///
/// This function:
/// 1. Sets up logging with env_logger
/// 2. Parses command-line arguments
/// 3. Handles daemonization if requested
/// 4. Dispatches to the appropriate command handler
/// 5. Handles errors and returns appropriate exit codes
///
/// # Returns
/// - `Ok(())` if the application completes successfully
/// - `Err(...)` if an error occurs during execution
fn main() -> Result<()> {
    // SECTION: Command-line Argument Parsing
    // Parse command-line arguments using clap
    let cli_args = Cli::parse();

    // Extract interactive mode flag (affects daemonization)
    let interactive_mode = cli_args.interactive;

    // Configure logging level based on --verbose flag
    // When --verbose is set, we enable debug-level logging
    if cli_args.verbose && std::env::var("RUST_LOG").is_err() {
        // Only set RUST_LOG if it's not already set by the user
        unsafe {
            std::env::set_var("RUST_LOG", "drapto=debug");
        }
    }

    // Configure color settings (disabled if --no-color flag is used)
    terminal::set_color(!cli_args.no_color);

    // Register the CLI progress reporter to centralize all formatting
    terminal::register_cli_reporter();

    // SECTION: Command Dispatch
    // Process the selected command - execution happens either in the original process
    // (interactive mode) or in a daemon process (non-interactive mode)
    let _ = match cli_args.command {
        Commands::Encode(args) => {
            // STEP 1: Discover files to encode
            // Find all .mkv files in the input directory or validate the input file
            let (discovered_files, effective_input_dir) =
                discover_encode_files(&args).with_context(|| "Error during file discovery")?;

            // STEP 2: Calculate log path (needed before daemonization for user feedback)
            // This logic mirrors the start of run_encode to predict the log path
            // and provide consistent information to the user
            let (actual_output_dir, _target_filename_override_os) =
                if discovered_files.len() == 1 && args.output_dir.extension().is_some() {
                    // Single file mode: output_dir is treated as a target filename
                    let target_file = args.output_dir.clone();
                    let parent_dir = target_file
                        .parent()
                        .map(|p| p.to_path_buf())
                        .filter(|p| !p.as_os_str().is_empty())
                        .unwrap_or_else(|| PathBuf::from("."));
                    let filename_os = target_file.file_name().map(|name| name.to_os_string());
                    (parent_dir, filename_os)
                } else {
                    // Directory mode: output_dir is treated as a directory
                    (args.output_dir.clone(), None)
                };

            // Determine log directory (either specified by user or default to output_dir/logs)
            let log_dir = args
                .log_dir
                .clone()
                .unwrap_or_else(|| actual_output_dir.join("logs"));

            // Generate log filename with timestamp for uniqueness
            let main_log_filename = format!("drapto_encode_run_{}.log", get_timestamp());
            let main_log_path = log_dir.join(&main_log_filename);

            // SECTION: Logging Setup
            // Set up logging based on mode
            if interactive_mode {
                // For interactive mode, use fern to log to both console and file
                setup_file_logging(&main_log_path)
                    .with_context(|| format!("Failed to set up file logging to: {}", main_log_path.display()))?;
            } else {
                // For daemon mode, use env_logger (stdout/stderr will be redirected to file)
                env_logger::Builder::from_env(Env::default().default_filter_or("drapto=info"))
                    .format(|buf, record| {
                        if record.level() != Level::Info {
                            writeln!(buf, "[{}] {}", record.level(), record.args())
                        } else {
                            writeln!(buf, "{}", record.args())
                        }
                    })
                    .init();
            }

            // Provide feedback about logging level to help with debugging
            if log::log_enabled!(log::Level::Trace) {
                log::info!("Trace level logging enabled.");
            } else if log::log_enabled!(log::Level::Debug) {
                log::info!("Debug level logging enabled.");
            }

            // STEP 3: Daemonize if running in non-interactive mode
            if !interactive_mode {
                // Display all pre-daemonization information in a single block
                // to reduce the number of flush operations needed

                // Show pre-daemonization messages using the centralized functions
                // These use eprintln! internally instead of log macros
                // since logging will be redirected to the daemon log file
                crate::terminal::print_daemon_file_list(&discovered_files);
                crate::terminal::print_daemon_log_info(&main_log_path);
                crate::terminal::print_daemon_starting();

                // Single flush operation after all messages
                if let Err(e) = io::stderr().flush() {
                    eprintln!("Warning: Failed to flush stderr before daemonizing: {}", e);
                }

                // Create log directory if it doesn't exist
                std::fs::create_dir_all(&log_dir).with_context(|| {
                    format!("Failed to create log directory: {}", log_dir.display())
                })?;

                // Create and open log file for the daemon's stdout/stderr
                let log_file = std::fs::File::create(&main_log_path).with_context(|| {
                    format!("Failed to create log file: {}", main_log_path.display())
                })?;

                // Clone the file handle for stderr
                let log_file_stderr = log_file
                    .try_clone()
                    .context("Failed to clone log file handle")?;

                // Create daemonize configuration
                // Note: PID file is handled in run_encode after log setup
                let daemonize = Daemonize::new()
                    .working_directory(".") // Keep working directory
                    .stdout(log_file) // Redirect stdout to our log file
                    .stderr(log_file_stderr); // Redirect stderr to our log file

                // Attempt to daemonize the process
                daemonize
                    .start()
                    .with_context(|| "Failed to start daemon process")?;
                // Parent process exits here after successful fork
                // The daemon child process continues execution below
                // Child process continues execution from this point
            }

            // STEP 4: Run the encode command
            // Initialize required dependencies with concrete implementations

            // Execute the encode command with all necessary parameters based on notification type
            // This runs in either the original process (interactive mode)
            // or the daemon child process (non-interactive mode)
            // Create notification sender if a topic is provided
            let notification_sender = if let Some(ref topic) = args.ntfy {
                match NtfyNotificationSender::new(topic) {
                    Ok(sender) => Some(sender),
                    Err(e) => {
                        log::warn!("Warning: Failed to create notification sender: {}", e);
                        None
                    }
                }
            } else {
                None
            };

            // Run the encode command with the notification sender (or None)
            run_encode(
                notification_sender.as_ref(),
                args,
                interactive_mode,
                discovered_files,
                effective_input_dir,
            )
        } // Future commands would be added here as additional match arms
    };

    // Success
    Ok(())
}
