//! Drapto Core Library
//!
//! This library provides core functionality for the Drapto video encoding tool.
//! It handles media information, validation, detection, and encoding operations.

// Core modules
pub mod error;
pub mod config;
pub mod logging;

// Media information and probing
pub mod media;

// Detection algorithms
pub mod detection;

// Validation functionality
pub mod validation;

// Encoding functionality (placeholder for future phases)
pub mod encoding;

// Utility functions
pub mod util;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Re-exports of commonly used types for convenience
pub use error::{DraptoError, Result};
pub use config::Config;
pub use media::{MediaInfo, StreamInfo, FormatInfo, StreamType};
pub use validation::{ValidationReport, ValidationLevel, ValidationMessage};