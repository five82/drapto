//! Command implementation module for Drapto CLI
//!
//! Responsibilities:
//! - Implement command-line subcommands (encode, validate, info)
//! - Process command-line arguments into core library calls
//! - Handle command-specific error conditions
//! - Format and present command outputs
//!
//! This module contains the implementation of all CLI commands,
//! translating user input into operations on the core library.

pub mod encode;
pub mod validate;
pub mod info;

// Re-export common command functionality
pub use encode::{execute_encode, execute_encode_directory};
pub use validate::execute_validate;
pub use info::execute_ffmpeg_info;