//! Detection algorithms for media content analysis
//!
//! Responsibilities:
//! - Scene detection for video segmentation
//! - Content format detection (HDR, SDR, Dolby Vision)
//! - Stream properties analysis
//! - Video quality assessment
//!
//! This module contains detection algorithms that analyze media files
//! to determine their characteristics and optimal processing parameters.

pub mod scene;
pub mod format;

// Re-export scene detection functions
pub use scene::{detect_scenes, detect_scenes_with_config};