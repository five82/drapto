// drapto-cli/src/main.rs
//
// Main entry point for the Drapto CLI application.
// Parses arguments and dispatches to command handlers.

use drapto_cli::{Cli, Commands, run_encode};
use drapto_cli::commands::encode::discover_encode_files;
use drapto_cli::logging::get_timestamp;
use clap::Parser;
use daemonize::Daemonize;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli_args = Cli::parse();

    let interactive_mode = cli_args.interactive;


    // Match on the command provided - runs in original process (interactive) or daemon process
    let result = match cli_args.command {
        Commands::Encode(args) => {
            // --- Discover files ---
            let (discovered_files, effective_input_dir) = match discover_encode_files(&args) {
                 Ok(result) => result,
                 Err(e) => {
                     // Use existing error reporting logic (copied from below)
                     let mut stderr = StandardStream::stderr(ColorChoice::Auto);
                     stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
                     writeln!(&mut stderr, "Error during file discovery: {}", e)?;
                     stderr.reset()?;
                     process::exit(1);
                 }
            };

            // --- Calculate potential log path (needed before daemonization for printing) ---
            // This logic mirrors the start of run_encode to predict the log path.
            let (actual_output_dir, _target_filename_override_os) =
                if discovered_files.len() == 1 && args.output_dir.extension().is_some() {
                    let target_file = args.output_dir.clone();
                    let parent_dir = target_file.parent()
                        .map(|p| p.to_path_buf())
                        .filter(|p| !p.as_os_str().is_empty())
                        .unwrap_or_else(|| PathBuf::from("."));
                    let filename_os = target_file.file_name().map(|name| name.to_os_string());
                    (parent_dir, filename_os)
                } else {
                    (args.output_dir.clone(), None)
                };
            let log_dir = args.log_dir.clone().unwrap_or_else(|| actual_output_dir.join("logs"));
            // Note: We don't create the log dir here, run_encode will do it.
            // We also don't need the full log_callback setup here, just the path.
            let main_log_filename = format!("drapto_encode_run_{}.log", get_timestamp());
            let main_log_path = log_dir.join(&main_log_filename);


            // --- Daemonize if needed ---
            if !interactive_mode {
                // Print discovered files *before* daemon message
                if !discovered_files.is_empty() {
                    eprintln!("Will encode the following files:");
                    for file in &discovered_files {
                        eprintln!("  - {}", file.display());
                    }
                } else {
                     eprintln!("No .mkv files found to encode in the specified input.");
                }
                 io::stderr().flush().unwrap_or_else(|e| {
                     eprintln!("Warning: Failed to flush stderr before daemonizing: {}", e);
                 });

                // Print log file path *before* daemon message
                eprintln!("Log file: {}", main_log_path.display());
                io::stderr().flush().unwrap_or_else(|e| {
                    eprintln!("Warning: Failed to flush stderr before daemonizing: {}", e);
                });


                // Print daemon start message *before* attempting to daemonize
                eprintln!("Starting Drapto daemon in the background...");
                io::stderr().flush().unwrap_or_else(|e| {
                     eprintln!("Warning: Failed to flush stderr before daemonizing: {}", e);
                });

                // We don't configure PID file here; it will be handled in run_encode after log setup.
                let daemonize = Daemonize::new()
                    .working_directory(".");
                match daemonize.start() {
                    Ok(_) => {
                        // Parent process exits successfully after fork.
                        // The daemon child process continues execution *after* the match statement.
                    }
                    Err(e) => {
                        // Failed to daemonize, report error to original stderr
                        eprintln!("Error starting daemon: {}", e);
                        process::exit(1);
                    }
                }
                // Child process continues here...
            }

            // --- Run the encode command ---
            // This runs in the original process (interactive) or the daemon process
            run_encode(args, interactive_mode, discovered_files, effective_input_dir)
        } // Add other command arms here
    };

    if let Err(e) = result {
        // Error handling:
        // In interactive mode, this prints to the original terminal's stderr.
        // In daemon mode, stderr might be redirected (e.g., to /dev/null or a file by systemd/launchd).
        // Critical errors *after* daemonization should be logged to the file by run_encode.
        // This block primarily catches errors *before* logging is fully set up or if run_encode itself fails early.
        if interactive_mode {
            let mut stderr = StandardStream::stderr(ColorChoice::Auto);
            stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
            writeln!(&mut stderr, "Error: {}", e)?;
            stderr.reset()?;
        } else {
            // In daemon mode, just print to stderr; it might go nowhere, but worth trying.
            // The primary error reporting mechanism is the log file handled within run_encode.
            eprintln!("Daemon Error: {}", e);
        }
        process::exit(1);
    }

    Ok(())
}