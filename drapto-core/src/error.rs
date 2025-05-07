// ============================================================================
// drapto-core/src/error.rs
// ============================================================================
//
// ERROR HANDLING: Custom Error Types and Result Definitions
//
// This module defines the custom error handling infrastructure for the drapto-core
// library. It provides a comprehensive error type hierarchy that covers all possible
// error conditions that can occur during video processing operations.
//
// KEY COMPONENTS:
// - CoreError: Enum of all possible errors with descriptive messages
// - CoreResult: Type alias for Result<T, CoreError> for consistent return types
//
// ERROR CATEGORIES:
// - I/O and filesystem errors (Io, Walkdir, PathError)
// - External command errors (CommandStart, CommandWait, CommandFailed)
// - Parsing errors (FfprobeParse, JsonParseError)
// - Video processing errors (VideoInfoError, NoFilesFound)
// - Dependency errors (DependencyNotFound)
// - Film grain analysis errors (FilmGrainEncodingFailed, FilmGrainAnalysisFailed)
// - Notification errors (NotificationError)
//
// USAGE:
// Functions in the library return CoreResult<T> to provide consistent error
// handling. Consumers can use the ? operator to propagate errors or match
// on specific error variants for custom handling.
//
// AI-ASSISTANT-INFO: Error handling infrastructure for the drapto-core library

// ---- External crate imports ----
use thiserror::Error;

// ---- Standard library imports ----
use std::io;

// ============================================================================
// ERROR TYPES
// ============================================================================

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
    // ---- I/O and Filesystem Errors ----

    /// Standard I/O errors from the std::io module
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Errors that occur during directory traversal with walkdir
    #[error("Directory traversal error: {0}")]
    Walkdir(#[from] walkdir::Error),

    /// General path-related errors (invalid paths, missing files, etc.)
    #[error("Path error: {0}")]
    PathError(String),

    // ---- External Command Errors ----

    /// Errors that occur when attempting to start an external command
    #[error("Failed to execute {0}: {1}")]
    CommandStart(String, io::Error), // e.g., "ffprobe", source error

    /// Errors that occur when waiting for an external command to complete
    #[error("Failed to wait for {0}: {1}")]
    CommandWait(String, io::Error), // e.g., "ffmpeg", source error

    /// Errors that occur when an external command exits with a non-zero status
    #[error("Command {0} failed with status {1}. Stderr: {2}")]
    CommandFailed(String, std::process::ExitStatus, String), // e.g., "ffprobe", status, stderr

    // ---- Parsing Errors ----

    /// Errors that occur when parsing ffprobe output
    #[error("ffprobe output parsing error: {0}")]
    FfprobeParse(String),

    /// Errors that occur when parsing JSON output
    #[error("Failed to parse JSON output: {0}")]
    JsonParseError(String),

    /// Errors that occur when extracting video information
    #[error("Failed to extract video information: {0}")]
    VideoInfoError(String),

    // ---- Video Processing Errors ----

    /// Error indicating that no suitable video files were found
    #[error("No suitable video files found in input directory")]
    NoFilesFound,

    /// Error indicating that a required external dependency is missing
    #[error("Required external command '{0}' not found or failed to execute. Please ensure it's installed and in your PATH.")]
    DependencyNotFound(String),

    /// General operation failure
    #[error("Operation failed: {0}")]
    OperationFailed(String),

    // ---- Film Grain Analysis Errors ----

    /// Errors that occur during film grain sample extraction or encoding
    #[error("Film grain sample extraction/encoding failed: {0}")]
    FilmGrainEncodingFailed(String),

    /// Errors that occur during film grain analysis
    #[error("Film grain analysis failed: {0}")]
    FilmGrainAnalysisFailed(String),

    /// Error indicating that grain analysis returned no data
    #[error("Grain analysis using ffprobe returned no data for file: {0}")]
    GrainAnalysisNoData(String),

    // ---- Notification Errors ----

    /// Errors that occur when sending notifications
    #[error("Notification error: {0}")]
    NotificationError(String),

    /// Error indicating that ffmpeg reported no streams found
    #[error("FFmpeg reported 'No streams found' for input file: {0}")]
    NoStreamsFound(String),
}

// ============================================================================
// RESULT TYPE ALIAS
// ============================================================================

/// Type alias for Result using our custom error type.
///
/// This type alias is used throughout the library to provide a consistent
/// return type for functions that can fail. It simplifies function signatures
/// and makes it clear that the function can return a CoreError.
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