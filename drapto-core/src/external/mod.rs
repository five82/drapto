// drapto-core/src/external/mod.rs
//
// Encapsulates interactions with external CLI tools like ffmpeg and ffprobe.
// Provides functions for dependency checking and abstracting tool execution.

#[cfg(not(feature = "test-mocks"))]
use crate::error::CoreError; // Conditionally import CoreError
use crate::error::CoreResult; // Keep CoreResult unconditionally
#[cfg(not(feature = "test-mocks"))]
use std::io; // Conditionally import io
use std::path::Path; // Keep Path unconditionally
#[cfg(not(feature = "test-mocks"))]
use std::process::{Command, Stdio}; // Conditionally import Command and Stdio

// Declare submodules
pub mod ffmpeg; // Contains ffmpeg argument building logic (run_ffmpeg_encode)
pub mod ffmpeg_executor; // Contains traits/impls for executing ffmpeg
pub mod ffprobe_executor; // Contains traits/impls for executing ffprobe
pub mod mocks; // Contains mock implementations for testing

// Re-export traits and real executors for convenience
pub use ffmpeg_executor::{FfmpegProcess, FfmpegSpawner, SidecarProcess, SidecarSpawner};
pub use ffprobe_executor::{CommandFfprobeExecutor, FfprobeExecutor};

// TODO: Move get_video_duration_secs here (if needed).
// Consider creating external/ffprobe.rs for ffprobe-specific logic beyond execution.

/// Checks if a required external command is available and executable.
/// Returns the command parts (e.g., `["ffmpeg"]`) if found,
/// otherwise returns an error.
#[cfg(not(feature = "test-mocks"))]
pub(crate) fn check_dependency(cmd_name: &str) -> CoreResult<Vec<String>> {
    let version_arg = "-version";
    let direct_cmd_parts = vec![cmd_name.to_string()];
    let direct_result = Command::new(&direct_cmd_parts[0])
        .arg(version_arg)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match direct_result {
        Ok(_) => {
            log::debug!("Found dependency directly: {}", cmd_name);
            Ok(direct_cmd_parts)
        }
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound {
                log::warn!("Dependency '{}' not found.", cmd_name);
                Err(CoreError::DependencyNotFound(cmd_name.to_string()))
            } else {
                log::error!("Failed to start dependency check command '{}': {}", cmd_name, e);
                Err(CoreError::CommandStart(cmd_name.to_string(), e))
            }
        }
    }
}


// --- Conditional Export of get_audio_channels ---
// This function now acts as the public interface, using the appropriate executor.
// The test-mock-ffprobe feature is kept for backward compatibility with existing tests/code
// that might rely on the simple default mock behavior. New tests should inject MockFfprobeExecutor.

// Removed #[cfg(not(feature = "test-mock-ffprobe"))]
#[allow(dead_code)] // Allow dead code warning when test-mock-ffprobe feature is active (still needed for test builds)
/// Gets audio channel counts using the real ffprobe command executor.
/// This is the main public-facing function when mocks are not enabled.
pub(crate) fn get_audio_channels(input_path: &Path) -> CoreResult<Vec<u32>> {
    // Instantiate the real executor here for the public-facing function
    CommandFfprobeExecutor.get_audio_channels(input_path)
} // Add missing closing brace
// Removed unused #[cfg(feature = "test-mock-ffprobe")] version of get_audio_channels

// Removed get_video_width function and related structs (FfprobeOutput, StreamInfo)
// as width is now obtained via detection::get_video_properties.