// ============================================================================
// drapto-core/src/external/mod.rs
// ============================================================================
//
// EXTERNAL TOOLS: Interactions with External CLI Tools and File System
//
// This module encapsulates interactions with external command-line tools like
// ffmpeg and ffprobe, as well as file system operations. It provides abstractions
// through traits and concrete implementations to make these external dependencies
// testable and maintainable.
//
// KEY COMPONENTS:
// - Traits for external tool interactions (FfmpegSpawner, FfprobeExecutor)
// - Concrete implementations using ffmpeg-sidecar and ffprobe crates
// - File metadata access abstraction
// - Platform detection utilities
//
// DESIGN PHILOSOPHY:
// This module follows the dependency injection pattern, allowing consumers to
// provide their own implementations of the traits for testing or specialized
// behavior. The default implementations use the ffmpeg-sidecar and ffprobe crates.
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

/// Contains traits and implementations for executing ffmpeg commands
pub mod ffmpeg_executor;

/// Contains traits and implementations for executing ffprobe commands
pub mod ffprobe_executor;

// ============================================================================
// RE-EXPORTS
// ============================================================================
// These items are re-exported to make them directly accessible to consumers
// without requiring explicit imports from submodules

// ----- FFmpeg Execution -----
/// Functions for spawning and interacting with ffmpeg processes
pub use ffmpeg_executor::{spawn_ffmpeg, handle_ffmpeg_events, wait_for_ffmpeg, extract_sample};

// ----- FFprobe Execution -----
/// Functions for executing ffprobe commands
pub use ffprobe_executor::{get_audio_channels, get_video_properties, run_ffprobe_bitplanenoise, get_media_info, MediaInfo};

// ============================================================================
// AUDIO CHANNEL DETECTION
// ============================================================================


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

// Platform detection has been moved to the hardware_accel module
// Re-export the is_macos function for backward compatibility
pub use crate::hardware_accel::is_macos;
