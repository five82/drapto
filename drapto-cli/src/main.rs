// drapto-cli/src/main.rs
//
// Main entry point for the Drapto CLI application.
// Parses arguments and dispatches to command handlers.

// Use items from the drapto_cli library crate
use drapto_cli::{Cli, Commands, run_encode};
use clap::Parser;
use std::io::Write; // For writeln
use std::process;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse the top-level arguments using the struct from the cli module
    let cli_args = Cli::parse(); // Use the imported Cli struct

    // Match on the command provided
    let result = match cli_args.command {
        Commands::Encode(args) => { // Use the imported Commands enum
            // Call the specific function for the encode command (now re-exported)
            run_encode(args)
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