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
// - Dependency checking functions
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
use crate::error::{CoreError, CoreResult};

// ---- Standard library imports ----
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};
use std::env;

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
/// Traits and implementations for spawning and interacting with ffmpeg processes
pub use ffmpeg_executor::{FfmpegProcess, FfmpegSpawner, SidecarProcess, SidecarSpawner};

// ----- FFprobe Execution -----
/// Traits and implementations for executing ffprobe commands
pub use ffprobe_executor::{CrateFfprobeExecutor, FfprobeExecutor};

// ============================================================================
// DEPENDENCY CHECKING
// ============================================================================

/// Checks if a required external command is available and executable.
///
/// This function attempts to run the specified command with a `-version` argument
/// to verify that it exists and is executable. It is used to check for the
/// presence of required external tools like ffmpeg and ffprobe.
///
/// # Arguments
///
/// * `cmd_name` - The name of the command to check (e.g., "ffmpeg")
///
/// # Returns
///
/// * `Ok(Vec<String>)` - A vector containing the command parts if the command is found
/// * `Err(CoreError::DependencyNotFound)` - If the command is not found
/// * `Err(CoreError::CommandStart)` - If the command exists but fails to start
///
/// # Examples
///
/// ```rust,no_run
/// // This function is internal to the crate, so we can't call it directly in doctests
/// // Example usage within the crate:
/// // match check_dependency("ffmpeg") {
/// //     Ok(_) => println!("ffmpeg is available"),
/// //     Err(e) => eprintln!("ffmpeg check failed: {}", e),
/// // }
/// ```
pub(crate) fn check_dependency(cmd_name: &str) -> CoreResult<Vec<String>> {
    // Define the argument to use for version checking
    let version_arg = "-version";

    // Create a vector with the command name
    let direct_cmd_parts = vec![cmd_name.to_string()];

    // Attempt to run the command with the version argument
    let direct_result = Command::new(&direct_cmd_parts[0])
        .arg(version_arg)
        .stdout(Stdio::null())  // Discard stdout
        .stderr(Stdio::null())  // Discard stderr
        .status();              // Just check the exit status

    // Handle the result
    match direct_result {
        Ok(_) => {
            // Command executed successfully
            log::debug!("Found dependency directly: {}", cmd_name);
            Ok(direct_cmd_parts)
        }
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound {
                // Command not found
                log::warn!("Dependency '{}' not found.", cmd_name);
                Err(CoreError::DependencyNotFound(cmd_name.to_string()))
            } else {
                // Command exists but failed to start
                log::error!("Failed to start dependency check command '{}': {}", cmd_name, e);
                Err(CoreError::CommandStart(cmd_name.to_string(), e))
            }
        }
    }
}

// ============================================================================
// AUDIO CHANNEL DETECTION
// ============================================================================

/// Gets the number of audio channels for each audio stream in a video file.
///
/// This function uses the CrateFfprobeExecutor to analyze the audio streams
/// in the specified video file and return the number of channels for each stream.
///
/// # Arguments
///
/// * `input_path` - Path to the video file to analyze
///
/// # Returns
///
/// * `Ok(Vec<u32>)` - A vector containing the number of channels for each audio stream
/// * `Err(CoreError)` - If an error occurs during ffprobe execution or parsing
///
/// # Note
///
/// This function is marked with #[allow(dead_code)] because it may not be used
/// in all configurations but is still needed for test builds.
#[allow(dead_code)]
pub(crate) fn get_audio_channels(input_path: &Path) -> CoreResult<Vec<u32>> {
    // Create a new instance of the ffprobe executor and delegate to it
    CrateFfprobeExecutor::new().get_audio_channels(input_path)
}

// ============================================================================
// FILE METADATA ACCESS
// ============================================================================

/// Trait for abstracting file metadata access operations.
///
/// This trait provides an abstraction over file system operations to retrieve
/// metadata about files, such as their size. It allows for dependency injection
/// and easier testing by decoupling the code from direct file system access.
///
/// # Examples
///
/// ```rust,no_run
/// use drapto_core::external::FileMetadataProvider;
/// use drapto_core::CoreResult;
/// use std::path::Path;
///
/// struct MockMetadataProvider;
///
/// impl FileMetadataProvider for MockMetadataProvider {
///     fn get_size(&self, _path: &Path) -> CoreResult<u64> {
///         // Return a fixed size for testing
///         Ok(1_000_000)
///     }
/// }
///
/// // Use the mock provider in tests
/// let provider = MockMetadataProvider;
/// let size = provider.get_size(Path::new("/fake/path")).unwrap();
/// assert_eq!(size, 1_000_000);
/// ```
pub trait FileMetadataProvider {
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
    fn get_size(&self, path: &Path) -> CoreResult<u64>;
}

/// Standard implementation of FileMetadataProvider using the standard library.
///
/// This implementation uses std::fs::metadata to get the size of files.
/// It is the default implementation used in production code.
#[derive(Debug, Clone, Default)]
pub struct StdFsMetadataProvider;

impl FileMetadataProvider for StdFsMetadataProvider {
    fn get_size(&self, path: &Path) -> CoreResult<u64> {
        // Get the file metadata and extract the size
        Ok(std::fs::metadata(path)?.len())
    }
}

// ============================================================================
// PLATFORM DETECTION
// ============================================================================

/// Checks if the current platform is macOS.
///
/// This function uses the `std::env::consts::OS` constant to determine
/// if the current operating system is macOS.
///
/// # Returns
///
/// * `true` - If the current platform is macOS
/// * `false` - Otherwise
///
/// # Examples
///
/// ```rust
/// use drapto_core::external::is_macos;
///
/// if is_macos() {
///     println!("Running on macOS, can use VideoToolbox");
/// } else {
///     println!("Not running on macOS");
/// }
/// ```
pub fn is_macos() -> bool {
    env::consts::OS == "macos"
}