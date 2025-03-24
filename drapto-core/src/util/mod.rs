//! Utility functions and helpers module
//!
//! Responsibilities:
//! - Provide command execution infrastructure with progress reporting
//! - Define job abstractions for various media processing tasks
//! - Implement memory-aware scheduling for resource management
//! - Support logging and progress tracking during operations
//! - Offer common utilities shared across different subsystems
//!
//! This module contains fundamental utility functions and structures
//! that support the core functionality throughout the codebase, including
//! command execution, job management, and resource scheduling.

pub mod command;
pub mod jobs;
pub mod scheduler;
pub mod logging;

// Re-export commonly used types and functions
pub use command::{run_command, run_command_with_progress, ProgressCallback, CommandError};
pub use jobs::{CommandJob, FFmpegEncodeJob, FFprobeJob, AudioEncodeJob, SegmentationJob, ConcatenationJob};
pub use scheduler::{MemoryAwareScheduler, SchedulerBuilder, TaskState, TaskStatus};