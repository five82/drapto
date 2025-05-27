//! Library component for the Drapto CLI application.
//!
//! This contains the core functionality, argument definitions, and command logic
//! that the binary crate uses. The library is organized into modules for
//! different aspects of the CLI.

/// Command-line interface definitions using clap
pub mod cli;

/// Command implementations for each subcommand
pub mod commands;

/// Configuration constants and default values
pub mod config;

/// Error handling utilities for the CLI
pub mod error;

/// Logging utilities and helper functions
pub mod logging;

/// Platform-specific functionality and detection
pub mod platform;

/// Terminal UI components and styling
pub mod terminal;

// Re-exports for convenience
pub use cli::{Cli, Commands, EncodeArgs};
pub use commands::encode::run_encode;
pub use platform::{HardwareDecoding, is_macos};
