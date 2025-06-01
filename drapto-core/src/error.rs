//! Custom error types and result definitions for drapto-core.
//!
//! This module provides a comprehensive error type hierarchy that covers all possible
//! error conditions during video processing operations, including I/O errors,
//! external command failures, parsing errors, and video processing errors.

use thiserror::Error;

use std::io;
use std::process::ExitStatus;

/// Represents the kind of command error that occurred.
#[derive(Debug)]
pub enum CommandErrorKind {
    /// Error occurred when attempting to start a command
    Start(io::Error),

    /// Error occurred when waiting for a command to complete
    Wait(io::Error),

    /// Command completed but returned a non-zero exit status
    Failed(ExitStatus, String), // exit status and stderr output
}

/// Represents an error that occurred when executing an external command.
#[derive(Debug)]
pub struct CommandError {
    /// The name of the command that failed (e.g., "ffmpeg", "ffprobe")
    pub command: String,

    /// The specific kind of error that occurred
    pub kind: CommandErrorKind,
}

/// All possible errors in drapto-core operations.
#[derive(Error, Debug)]
pub enum CoreError {
    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Path-related errors
    #[error("Path error: {0}")]
    PathError(String),

    /// External command execution errors
    #[error("{}", format_command_error(.0))]
    Command(CommandError),

    /// FFprobe output parsing errors
    #[error("ffprobe output parsing error: {0}")]
    FfprobeParse(String),

    /// JSON parsing errors
    #[error("Failed to parse JSON output: {0}")]
    JsonParseError(String),

    /// Video info extraction errors
    #[error("Failed to extract video information: {0}")]
    VideoInfoError(String),

    /// Configuration validation errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// No suitable video files found
    #[error("No suitable video files found in input directory")]
    NoFilesFound,

    /// General operation failure
    #[error("Operation failed: {0}")]
    OperationFailed(String),


    /// Notification sending errors
    #[error("Notification error: {0}")]
    NotificationError(String),

    /// Error indicating that ffmpeg reported no streams found
    #[error("FFmpeg reported 'No streams found' for input file: {0}")]
    NoStreamsFound(String),
}

/// Result type alias for drapto-core operations.
pub type CoreResult<T> = Result<T, CoreError>;

/// Helper function to format command errors for display.
fn format_command_error(err: &CommandError) -> String {
    match &err.kind {
        CommandErrorKind::Start(io_err) => {
            format!("Failed to execute {}: {}", err.command, io_err)
        }
        CommandErrorKind::Wait(io_err) => {
            format!("Failed to wait for {}: {}", err.command, io_err)
        }
        CommandErrorKind::Failed(status, stderr) => {
            format!(
                "Command {} failed with status {}. Stderr: {}",
                err.command, status, stderr
            )
        }
    }
}

/// Creates CommandStart error.
pub fn command_start_error(command: impl Into<String>, error: io::Error) -> CoreError {
    CoreError::Command(CommandError {
        command: command.into(),
        kind: CommandErrorKind::Start(error),
    })
}

/// Creates CommandWait error.
pub fn command_wait_error(command: impl Into<String>, error: io::Error) -> CoreError {
    CoreError::Command(CommandError {
        command: command.into(),
        kind: CommandErrorKind::Wait(error),
    })
}

/// Creates CommandFailed error.
pub fn command_failed_error(
    command: impl Into<String>,
    status: ExitStatus,
    stderr: impl Into<String>,
) -> CoreError {
    CoreError::Command(CommandError {
        command: command.into(),
        kind: CommandErrorKind::Failed(status, stderr.into()),
    })
}
