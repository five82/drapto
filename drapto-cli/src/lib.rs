// drapto-cli/src/lib.rs
//
// Library portion of the Drapto CLI application.
// Contains argument definitions and command logic.

pub mod cli;
pub mod commands;
pub mod config;
pub mod logging; // Re-added logging module declaration

// Re-export items needed by the binary or integration tests
pub use cli::{Cli, Commands, EncodeArgs};
pub use commands::encode::run_encode;