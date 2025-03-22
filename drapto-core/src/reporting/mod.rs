//! Reporting modules for drapto-core
//!
//! This module handles various reporting functionalities
//! including summaries, statistics, and logs.

pub mod summary;

// Re-export commonly used types
pub use summary::{EncodingSummary, BatchSummary, TimedSummary};