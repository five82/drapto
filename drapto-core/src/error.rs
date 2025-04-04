// drapto-core/src/error.rs
// Responsibility: Define custom error types and results for the crate.

use std::io;
use thiserror::Error; // Import the macro

// --- Custom Error Type ---

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error), // Auto-implements From<io::Error>

    #[error("Directory traversal error: {0}")]
    Walkdir(#[from] walkdir::Error),

    #[error("Path error: {0}")]
    PathError(String),

    #[error("Failed to execute {0}: {1}")]
    CommandStart(String, io::Error), // e.g., "ffprobe", source error

    #[error("Failed to wait for {0}: {1}")]
    CommandWait(String, io::Error), // e.g., "HandBrakeCLI", source error

    #[error("Command {0} failed with status {1}. Stderr: {2}")]
    CommandFailed(String, std::process::ExitStatus, String), // e.g., "ffprobe", status, stderr

    #[error("ffprobe output parsing error: {0}")]
    FfprobeParse(String),

    #[error("No suitable video files found in input directory")]
    NoFilesFound,

    #[error("Required external command '{0}' not found or failed to execute. Please ensure it's installed and in your PATH.")]
    DependencyNotFound(String),

    // --- Film Grain Errors ---
    #[error("Film grain sample extraction/encoding failed: {0}")]
    FilmGrainEncodingFailed(String),
    #[error("Film grain analysis failed: {0}")]
    FilmGrainAnalysisFailed(String),
}

// Type alias for Result using our custom error
pub type CoreResult<T> = Result<T, CoreError>;