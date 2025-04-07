// drapto-cli/src/main.rs
//
// Main entry point for the Drapto CLI application.
// Parses arguments and dispatches to command handlers.

// Use items from the drapto_cli library crate
use drapto_cli::{Cli, Commands, run_encode};
use drapto_cli::commands::encode::discover_encode_files; // Import the discovery function
use clap::Parser;
use daemonize::Daemonize; // Import Daemonize
use std::io::{self, Write}; // Import io for stderr().flush()
// Removed unused PathBuf import
use std::process;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
// Removed unused thread and Duration imports

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse the top-level arguments using the struct from the cli module
    let cli_args = Cli::parse();

    // Store interactive flag before potentially moving cli_args
    let interactive_mode = cli_args.interactive;

    // Daemonization logic moved inside the match arm for Encode command

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
                     process::exit(1); // Exit on discovery error
                 }
            };

            // --- Daemonize if needed ---
            if !interactive_mode {
                // Print discovered files *before* daemon message
                if !discovered_files.is_empty() {
                    eprintln!("Will encode the following files:");
                    for file in &discovered_files { // Borrow files here
                        eprintln!("  - {}", file.display());
                    }
                } else {
                     eprintln!("No .mkv files found to encode in the specified input.");
                }
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
                    .working_directory("."); // Keep current working directory
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

    // Handle the result from the command function
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
        process::exit(1); // Exit with error code
    }

    // If result was Ok, the process will exit with 0 implicitly when main returns Ok.
    Ok(())
}