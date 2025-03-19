use thiserror::Error;

/// Custom error types for drapto
#[derive(Error, Debug)]
pub enum DraptoError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Command execution failed: {0}")]
    CommandExecution(String),

    #[error("Invalid configuration: {0}")]
    Config(String),

    #[error("Media file error: {0}")]
    MediaFile(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("External tool error: {0}")]
    ExternalTool(String),

    #[error("Encoding error: {0}")]
    Encoding(String),

    #[error("Unexpected error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, DraptoError>;