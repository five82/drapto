// ============================================================================
// drapto-cli/src/lib.rs
// ============================================================================
//
// LIBRARY COMPONENT: Drapto CLI Application
//
// This file defines the library portion of the Drapto CLI application, which
// contains the core functionality, argument definitions, and command logic.
// The binary crate (main.rs) depends on this library crate for its implementation.
//
// KEY COMPONENTS:
// - Command-line argument structures (cli module)
// - Command implementations (commands module)
// - Configuration constants (config module)
// - Logging utilities (logging module)
//
// ARCHITECTURE:
// The library follows a modular design where:
// - cli.rs: Defines the command-line interface using clap
// - commands/: Contains implementations of each subcommand
// - config.rs: Defines default configuration values
// - logging.rs: Provides logging utilities
//
// AI-ASSISTANT-INFO: Library component for CLI application, contains core functionality

// ---- Module declarations ----
/// Command-line interface definitions using clap
pub mod cli;

/// Command implementations for each subcommand
pub mod commands;

/// Configuration constants and default values
pub mod config;

/// Logging utilities and helper functions
pub mod logging;

/// Platform-specific functionality and detection
pub mod platform;

/// CLI-specific progress reporting implementation
pub mod progress;

/// Terminal UI components and styling
pub mod terminal;

// ---- Public re-exports ----
// These items are re-exported to make them directly accessible to the binary crate
// and integration tests without requiring explicit imports from submodules

/// Command-line interface types
pub use cli::{Cli, Commands, EncodeArgs};

/// Command implementation functions
pub use commands::encode::run_encode;

/// Platform-specific functionality
pub use platform::{HardwareAcceleration, is_macos};
