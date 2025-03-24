//! Media information and probing module
//!
//! Responsibilities:
//! - Provide media file analysis and metadata extraction
//! - Define data structures for representing media information
//! - Execute and parse FFprobe commands to extract media details
//! - Cache media probe results for efficient repeated access
//! - Expose stream, format, and chapter information from media files
//!
//! This module provides comprehensive functionality for analyzing media files
//! using FFprobe, including structured data types for representing media information
//! and efficient caching mechanisms for repeated operations.

pub mod info;
pub mod probe;
pub mod session;

// Re-export commonly used types
pub use info::{MediaInfo, StreamInfo, FormatInfo, StreamType};
pub use probe::FFprobe;
pub use session::ProbeSession;