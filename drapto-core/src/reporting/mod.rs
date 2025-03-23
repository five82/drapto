//! Reporting and statistics module
//!
//! Responsibilities:
//! - Generate detailed encoding summaries and statistics
//! - Format reports for user presentation
//! - Track encoding progress and timing information
//! - Serialize reports for storage and later analysis
//! - Provide batch operation summaries for multiple encodings
//!
//! This module handles various reporting functionalities including
//! detailed summaries of encoding operations, statistical analysis,
//! and formatted output for user consumption.

pub mod summary;

// Re-export commonly used types
pub use summary::{EncodingSummary, BatchSummary, TimedSummary};