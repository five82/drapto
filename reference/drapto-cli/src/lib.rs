//! Library component for the Drapto CLI application.
//!
//! This contains the core functionality, argument definitions, and command logic
//! that the binary crate uses. The library is organized into modules for
//! different aspects of the CLI.

/// Command-line interface definitions using clap
pub mod cli;

/// Command implementations for each subcommand
pub mod commands;

/// Error handling utilities for the CLI
pub mod error;

/// Logging utilities and helper functions
pub mod logging;

/// Output path resolution utilities
pub mod output_path;

// Re-exports for convenience
pub use cli::{Cli, Commands, EncodeArgs, parse_cli, parse_cli_from};
pub use commands::encode::run_encode;
