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
use drapto_cli::{Cli, Commands, run_encode};
use drapto_cli::commands::encode::discover_encode_files;
use drapto_cli::logging::get_timestamp;

// ---- External crate imports ----
use drapto_core::external::{SidecarSpawner, CrateFfprobeExecutor};
use drapto_core::notifications::NtfyNotifier;
use clap::Parser;
use daemonize::Daemonize;
use colored::*;

// ---- Standard library imports ----
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;

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
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // SECTION: Logging Setup
    // Configure env_logger with custom format that only shows log level for non-INFO messages
    env_logger::Builder::from_env(Env::default().default_filter_or("drapto=info"))
        .format(|buf, record| {
            if record.level() != Level::Info {
                writeln!(buf, "[{}] {}", record.level(), record.args())
            } else {
                writeln!(buf, "{}", record.args())
            }
        })
        .init();

    // Provide feedback about logging level to help with debugging
    if log::log_enabled!(log::Level::Trace) {
        log::info!("{}", "Trace level logging enabled.".yellow().bold());
    } else if log::log_enabled!(log::Level::Debug) {
        log::info!("{}", "Debug level logging enabled.".yellow().bold());
    }

    // SECTION: Command-line Argument Parsing
    // Parse command-line arguments using clap
    let cli_args = Cli::parse();

    // Extract interactive mode flag (affects daemonization)
    let interactive_mode = cli_args.interactive;


    // SECTION: Command Dispatch
    // Process the selected command - execution happens either in the original process
    // (interactive mode) or in a daemon process (non-interactive mode)
    let result = match cli_args.command {
        Commands::Encode(args) => {
            // STEP 1: Discover files to encode
            // Find all .mkv files in the input directory or validate the input file
            let (discovered_files, effective_input_dir) = match discover_encode_files(&args) {
                 Ok(result) => result,
                 Err(e) => {
                     // Format error message with color for better visibility
                     eprintln!("{} {}", "Error during file discovery:".red().bold(), e);
                     process::exit(1);
                 }
            };

            // STEP 2: Calculate log path (needed before daemonization for user feedback)
            // This logic mirrors the start of run_encode to predict the log path
            // and provide consistent information to the user
            let (actual_output_dir, _target_filename_override_os) =
                if discovered_files.len() == 1 && args.output_dir.extension().is_some() {
                    // Single file mode: output_dir is treated as a target filename
                    let target_file = args.output_dir.clone();
                    let parent_dir = target_file.parent()
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
            let log_dir = args.log_dir.clone().unwrap_or_else(|| actual_output_dir.join("logs"));

            // Generate log filename with timestamp for uniqueness
            let main_log_filename = format!("drapto_encode_run_{}.log", get_timestamp());
            let main_log_path = log_dir.join(&main_log_filename);


            // STEP 3: Daemonize if running in non-interactive mode
            if !interactive_mode {
                // Display discovered files to user before daemonizing
                // This provides immediate feedback about what will be processed
                if !discovered_files.is_empty() {
                    eprintln!("{}", "Will encode the following files:".cyan().bold());
                    for file in &discovered_files {
                        eprintln!("  - {}", file.display().to_string().green());
                    }
                } else {
                     eprintln!("{}", "No .mkv files found to encode in the specified input.".yellow());
                }
                // Ensure output is flushed before daemonizing
                io::stderr().flush().unwrap_or_else(|e| {
                     eprintln!("{} Failed to flush stderr before daemonizing: {}", "Warning:".yellow().bold(), e);
                });

                // Show log file path so user knows where to find output
                eprintln!("{} {}", "Log file:".cyan(), main_log_path.display().to_string().green());
                io::stderr().flush().unwrap_or_else(|e| {
                    eprintln!("{} Failed to flush stderr before daemonizing: {}", "Warning:".yellow().bold(), e);
                });

                // Inform user that process is going to background
                eprintln!("{}", "Starting Drapto daemon in the background...".green().bold());
                io::stderr().flush().unwrap_or_else(|e| {
                     eprintln!("{} Failed to flush stderr before daemonizing: {}", "Warning:".yellow().bold(), e);
                });

                // Create daemonize configuration
                // Note: PID file is handled in run_encode after log setup
                let daemonize = Daemonize::new()
                    .working_directory("."); // Keep working directory

                // Attempt to daemonize the process
                match daemonize.start() {
                    Ok(_) => {
                        // Parent process exits here after successful fork
                        // The daemon child process continues execution below
                    }
                    Err(e) => {
                        // Failed to daemonize, report error and exit
                        eprintln!("{} {}", "Error starting daemon:".red().bold(), e);
                        process::exit(1);
                    }
                }
                // Child process continues execution from this point
            }

            // STEP 4: Run the encode command
            // Initialize required dependencies with concrete implementations
            let spawner = SidecarSpawner;                      // For spawning ffmpeg processes
            let ffprobe_executor = CrateFfprobeExecutor::new(); // For executing ffprobe commands
            let notifier = NtfyNotifier::new()?;               // For sending notifications

            // Execute the encode command with all necessary parameters
            // This runs in either the original process (interactive mode)
            // or the daemon child process (non-interactive mode)
            run_encode(
                &spawner,
                &ffprobe_executor,
                &notifier,
                args,
                interactive_mode,
                discovered_files,
                effective_input_dir
            )
        }
        // Future commands would be added here as additional match arms
    };

    // SECTION: Error Handling
    if let Err(e) = result {
        // Handle errors differently based on mode:
        // - Interactive mode: Display error to user's terminal
        // - Daemon mode: Log error (primary mechanism) and attempt to write to stderr (backup)
        //
        // Note: This primarily catches errors that occur:
        // 1. Before logging is fully set up
        // 2. If run_encode itself fails early
        // 3. In the parent process before daemonization
        if interactive_mode {
            // Interactive mode: Display error directly to user
            eprintln!("{} {}", "Error:".red().bold(), e);
        } else {
            // Daemon mode: Attempt to log to stderr (may be redirected)
            // The primary error reporting is via the log file in run_encode
            eprintln!("{} {}", "Daemon Error:".red().bold(), e);
        }
        process::exit(1); // Exit with error code
    }

    // Success
    Ok(())
}