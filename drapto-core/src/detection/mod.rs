//! Video detection module
//!
//! This module contains functionality for detecting various aspects of video files,
//! including scene detection and format detection.

pub mod scene;
pub mod format;

// Re-export scene detection functions
pub use scene::{detect_scenes, detect_scenes_with_config};