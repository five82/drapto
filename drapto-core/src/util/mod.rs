//! Utility functions and helpers
//!
//! This module contains utility functions for command execution, logging, and other
//! common tasks used throughout the codebase.

pub mod command;
pub mod jobs;

// Re-export commonly used types and functions
pub use command::{run_command, run_command_with_progress, ProgressCallback, CommandError};
pub use jobs::{CommandJob, FFmpegEncodeJob, FFprobeJob, AudioEncodeJob, SegmentationJob, ConcatenationJob};