//! Drapto Core Library
//!
//! Responsibilities:
//! - Provide a comprehensive video encoding pipeline using AV1
//! - Handle media analysis, detection, and validation 
//! - Implement segmentation-based parallel encoding
//! - Support HDR, SDR, and Dolby Vision content processing
//! - Offer detailed progress reporting and quality validation
//!
//! This library implements the core functionality for the Drapto video 
//! encoding tool, which offers a complete video encoding pipeline with
//! scene-based segmentation, parallel encoding, and quality optimization.
//! It provides robust error handling, consistent logging, and comprehensive
//! validation to ensure high-quality encoding output.
//!
//! The library is designed as a modular system with clean separation of concerns:
//! - `media`: Media file analysis and information extraction
//! - `detection`: Scene detection and format identification
//! - `encoding`: Video and audio encoding with segmentation support
//! - `validation`: Quality assurance and validation reporting
//! - `util`: Reusable utilities for command execution and parallel processing

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

// Encoding functionality
pub mod encoding;

// Reporting and summary
pub mod reporting;

// Utility functions
pub mod util;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Re-exports of commonly used types for convenience
pub use error::{DraptoError, Result};
pub use config::Config;
pub use media::{MediaInfo, StreamInfo, FormatInfo, StreamType};
pub use validation::{ValidationReport, ValidationLevel, ValidationMessage};
pub use reporting::{EncodingSummary, BatchSummary, TimedSummary};