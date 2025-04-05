// drapto-cli/src/main.rs
//
// Main entry point for the Drapto CLI application.
// Parses arguments and dispatches to command handlers.

use clap::Parser;
use std::io::Write; // For writeln
use std::process;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

// Declare modules
mod cli;
mod commands;
mod config; // Keep config for potential future use or direct defaults access
mod logging;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse the top-level arguments using the struct from the cli module
    let cli_args = cli::Cli::parse();

    // Match on the command provided
    let result = match cli_args.command {
        cli::Commands::Encode(args) => {
            // Call the specific function for the encode command from the commands module
            commands::encode::run_encode(args)
        } // Add other command arms here -> cli::Commands::Other(args) => commands::other::run_other(args),
    };

    // Handle the result from the command function
    if let Err(e) = result {
        // Use termcolor for stderr error reporting
        let mut stderr = StandardStream::stderr(ColorChoice::Auto);
        stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
        writeln!(&mut stderr, "Error: {}", e)?;
        stderr.reset()?;
        process::exit(1); // Exit with error code
    }

    // If result was Ok, the process will exit with 0 implicitly when main returns Ok.
    Ok(())
}