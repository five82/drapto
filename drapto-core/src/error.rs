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

    #[error("Segmentation error: {0}")]
    Segmentation(#[from] crate::encoding::segmentation::SegmentationError),
    
    #[error("Audio encoding error: {0}")]
    AudioEncoding(#[from] crate::encoding::audio::AudioEncodingError),
    
    #[error("Segment merger error: {0}")]
    SegmentMerger(#[from] crate::encoding::merger::SegmentMergerError),
    
    #[error("Muxing error: {0}")]
    Muxing(#[from] crate::encoding::muxer::MuxingError),
    
    #[error("Pipeline error: {0}")]
    Pipeline(#[from] crate::encoding::pipeline::PipelineError),

    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    #[error("Input file not found: {0}")]
    InputNotFound(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Unexpected error: {0}")]
    Other(String),
}

/// Result type for drapto operations
pub type Result<T> = std::result::Result<T, DraptoError>;