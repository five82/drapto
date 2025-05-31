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

/// Comprehensive error type for the drapto-core library.
///
/// This enum represents all possible errors that can occur during video processing
/// operations. Each variant includes a descriptive error message and, where
/// appropriate, additional context about the error.
///
/// # Examples
///
/// ```rust,no_run
/// use drapto_core::{CoreError, CoreResult};
/// use std::path::Path;
///
/// fn process_file(path: &Path) -> CoreResult<()> {
///     if !path.exists() {
///         return Err(CoreError::PathError(format!(
///             "File does not exist: {}",
///             path.display()
///         )));
///     }
///     // Process the file...
///     Ok(())
/// }
/// ```
#[derive(Error, Debug)]
pub enum CoreError {
    // I/O and Filesystem Errors
    /// Standard I/O errors from the `std::io` module
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// General path-related errors (invalid paths, missing files, etc.)
    #[error("Path error: {0}")]
    PathError(String),

    // External Command Errors
    /// Errors that occur when executing external commands
    #[error("{}", format_command_error(.0))]
    Command(CommandError),

    // Parsing Errors
    /// Errors that occur when parsing ffprobe output
    #[error("ffprobe output parsing error: {0}")]
    FfprobeParse(String),

    /// Errors that occur when parsing JSON output
    #[error("Failed to parse JSON output: {0}")]
    JsonParseError(String),

    /// Errors that occur when extracting video information
    #[error("Failed to extract video information: {0}")]
    VideoInfoError(String),

    // Video Processing Errors
    /// Error indicating that no suitable video files were found
    #[error("No suitable video files found in input directory")]
    NoFilesFound,

    /// General operation failure
    #[error("Operation failed: {0}")]
    OperationFailed(String),


    // Notification Errors
    /// Errors that occur when sending notifications
    #[error("Notification error: {0}")]
    NotificationError(String),

    /// Error indicating that ffmpeg reported no streams found
    #[error("FFmpeg reported 'No streams found' for input file: {0}")]
    NoStreamsFound(String),
}

/// Type alias for Result using our custom error type.
///
/// This type alias is used throughout the library to provide a consistent
/// return type for functions that can fail. It simplifies function signatures
/// and makes it clear that the function can return a `CoreError`.
///
/// # Examples
///
/// ```rust,no_run
/// use drapto_core::CoreResult;
/// use std::path::Path;
///
/// // Function that returns a CoreResult
/// fn read_video_duration(path: &Path) -> CoreResult<f64> {
///     // Implementation...
///     # Ok(0.0)
/// }
///
/// // Using the function with ? operator
/// fn process_video(path: &Path) -> CoreResult<()> {
///     let duration = read_video_duration(path)?;
///     println!("Video duration: {} seconds", duration);
///     Ok(())
/// }
/// ```
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

/// Convenience function to create a `CommandStart` error
pub fn command_start_error(command: impl Into<String>, error: io::Error) -> CoreError {
    CoreError::Command(CommandError {
        command: command.into(),
        kind: CommandErrorKind::Start(error),
    })
}

/// Convenience function to create a `CommandWait` error
pub fn command_wait_error(command: impl Into<String>, error: io::Error) -> CoreError {
    CoreError::Command(CommandError {
        command: command.into(),
        kind: CommandErrorKind::Wait(error),
    })
}

/// Convenience function to create a `CommandFailed` error
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
