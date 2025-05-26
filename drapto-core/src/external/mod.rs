// ============================================================================
// drapto-core/src/external/mod.rs
// ============================================================================
//
// EXTERNAL TOOLS: Interactions with External CLI Tools and File System
//
// This module encapsulates direct interactions with external command-line tools
// like ffmpeg and ffprobe, as well as file system operations. It provides
// simple functions without trait abstractions for cleaner, more direct code.
//
// KEY COMPONENTS:
// - Direct ffmpeg and ffprobe functions
// - File metadata access functions
// - Platform detection utilities
//
// AI-ASSISTANT-INFO: External tool interactions and abstractions for ffmpeg/ffprobe

// ---- Internal crate imports ----
use crate::error::CoreResult;

// ---- Standard library imports ----
use std::path::Path;

// ============================================================================
// SUBMODULES
// ============================================================================

/// Contains ffmpeg argument building logic and encoding parameter structures
pub mod ffmpeg;

/// Contains builder utilities for FFmpeg commands
pub mod ffmpeg_builder;

/// Contains traits and implementations for executing ffprobe commands
pub mod ffprobe_executor;

// ============================================================================
// RE-EXPORTS
// ============================================================================
// These items are re-exported to make them directly accessible to consumers
// without requiring explicit imports from submodules

// ----- FFmpeg Sample Extraction -----
/// Function for extracting video samples
pub use ffmpeg::extract_sample;

// ----- FFmpeg Quality Metrics -----
/// Function for calculating XPSNR between videos
pub use ffmpeg::calculate_xpsnr;

// ----- FFmpeg Command Building -----
/// Builder utilities for FFmpeg commands
pub use ffmpeg_builder::{FfmpegCommandBuilder, SvtAv1ParamsBuilder, VideoFilterChain};

// ----- FFprobe Execution -----
/// Functions for executing ffprobe commands
pub use ffprobe_executor::{
    MediaInfo, get_audio_channels, get_media_info, get_video_properties, run_ffprobe_bitplanenoise,
};

// ============================================================================
// FILE METADATA ACCESS
// ============================================================================

/// Gets the size of the file at the given path in bytes.
///
/// # Arguments
///
/// * `path` - Path to the file to get the size of
///
/// # Returns
///
/// * `Ok(u64)` - The size of the file in bytes
/// * `Err(CoreError)` - If an error occurs accessing the file
pub fn get_file_size(path: &Path) -> CoreResult<u64> {
    Ok(std::fs::metadata(path)?.len())
}

// ============================================================================
// PLATFORM DETECTION
// ============================================================================

// Platform detection has been moved to the hardware_decode module
// Re-export the is_macos function for backward compatibility
pub use crate::hardware_decode::is_macos;
