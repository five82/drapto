// ============================================================================
// drapto-core/src/temp_files.rs
// ============================================================================
//
// TEMPORARY FILE MANAGEMENT: Helper Functions for Temporary Files
//
// This module provides helper functions for creating and managing temporary
// files and directories used throughout the drapto-core library. It centralizes
// temporary file operations to ensure consistent behavior and proper cleanup.
//
// KEY COMPONENTS:
// - Functions for creating temporary directories for different purposes
// - Optional cleanup of base directories
// - Integration with CoreConfig for customizable temporary file locations
//
// DESIGN PHILOSOPHY:
// This module leverages the tempfile crate to handle automatic cleanup via
// the Drop trait, ensuring that temporary files are properly cleaned up even
// in error cases. It provides a consistent interface for temporary file
// operations throughout the codebase.
//
// AI-ASSISTANT-INFO: Temporary file management utilities

use std::path::{Path, PathBuf};
use tempfile::Builder as TempFileBuilder;
use crate::error::CoreResult;
use crate::config::CoreConfig;

/// Creates a temporary directory for grain analysis samples.
/// The directory will be automatically cleaned up when the returned TempDir is dropped.
///
/// # Arguments
///
/// * `config` - The core configuration containing path information
///
/// # Returns
///
/// * `CoreResult<tempfile::TempDir>` - A temporary directory that will be automatically
///   cleaned up when dropped
///
/// # Example
///
/// ```rust,no_run
/// use drapto_core::config::CoreConfig;
/// use drapto_core::temp_files;
/// use std::error::Error;
///
/// fn example() -> Result<(), Box<dyn Error>> {
///     let config = CoreConfig::default();
///     let temp_dir = temp_files::create_grain_analysis_dir(&config)?;
///     let temp_dir_path = temp_dir.path();
///     // Use temp_dir_path for operations
///     // temp_dir is automatically cleaned up when it goes out of scope
///     Ok(())
/// }
/// ```
pub fn create_grain_analysis_dir(config: &CoreConfig) -> CoreResult<tempfile::TempDir> {
    let base_dir = config.temp_dir.as_ref().unwrap_or(&config.output_dir).join("grain_samples_tmp");
    std::fs::create_dir_all(&base_dir)?;

    Ok(TempFileBuilder::new()
        .prefix("analysis_grain_")
        .tempdir_in(&base_dir)?)
}

/// Creates a temporary directory for other analysis operations.
///
/// # Arguments
///
/// * `config` - The core configuration containing path information
/// * `prefix` - A prefix to use for the temporary directory name
///
/// # Returns
///
/// * `CoreResult<tempfile::TempDir>` - A temporary directory that will be automatically
///   cleaned up when dropped
///
/// # Example
///
/// ```rust,no_run
/// use drapto_core::config::CoreConfig;
/// use drapto_core::temp_files;
/// use std::error::Error;
///
/// fn example() -> Result<(), Box<dyn Error>> {
///     let config = CoreConfig::default();
///     let temp_dir = temp_files::create_analysis_dir(&config, "crop_analysis_")?;
///     let temp_dir_path = temp_dir.path();
///     // Use temp_dir_path for operations
///     // temp_dir is automatically cleaned up when it goes out of scope
///     Ok(())
/// }
/// ```
pub fn create_analysis_dir(config: &CoreConfig, prefix: &str) -> CoreResult<tempfile::TempDir> {
    let base_dir = config.temp_dir.as_ref().unwrap_or(&config.output_dir).join("analysis_tmp");
    std::fs::create_dir_all(&base_dir)?;

    Ok(TempFileBuilder::new()
        .prefix(prefix)
        .tempdir_in(&base_dir)?)
}

/// Cleans up the base temporary directories if they're empty.
/// This is optional and can be called at the end of processing.
///
/// # Arguments
///
/// * `config` - The core configuration containing path information
///
/// # Returns
///
/// * `CoreResult<()>` - Ok if cleanup was successful, Err otherwise
///
/// # Example
///
/// ```rust,no_run
/// use drapto_core::config::CoreConfig;
/// use drapto_core::temp_files;
/// use std::error::Error;
///
/// fn example() -> Result<(), Box<dyn Error>> {
///     let config = CoreConfig::default();
///     // After all processing is complete
///     temp_files::cleanup_base_dirs(&config)?;
///     Ok(())
/// }
/// ```

/// Creates a temporary file path within a directory.
///
/// # Arguments
///
/// * `dir` - The directory in which to create the temporary file
/// * `prefix` - A prefix to use for the temporary file name
/// * `extension` - The file extension to use (without the dot)
///
/// # Returns
///
/// * `PathBuf` - The path to the temporary file
///
/// # Example
///
/// ```rust,no_run
/// use drapto_core::temp_files;
/// use std::path::Path;
///
/// fn example() {
///     let temp_dir_path = Path::new("/tmp");
///     let temp_file_path = temp_files::create_temp_file_path(temp_dir_path, "sample", "mkv");
///     // Use temp_file_path for operations
/// }
/// ```
pub fn create_temp_file_path(dir: &Path, prefix: &str, extension: &str) -> PathBuf {
    use rand::{thread_rng, Rng};
    use rand::distributions::Alphanumeric;

    // Generate a random suffix
    let random_suffix: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(6) // 6 random characters
        .map(char::from)
        .collect();

    // Create the filename with prefix, random suffix, and extension
    let filename = format!("{}_{}.{}", prefix, random_suffix, extension);

    // Return the full path
    dir.join(filename)
}

pub fn cleanup_base_dirs(config: &CoreConfig) -> CoreResult<()> {
    let dirs_to_check = [
        config.temp_dir.as_ref().unwrap_or(&config.output_dir).join("grain_samples_tmp"),
        config.temp_dir.as_ref().unwrap_or(&config.output_dir).join("analysis_tmp"),
    ];

    for dir in dirs_to_check {
        if dir.exists() {
            // Check if directory is empty
            if std::fs::read_dir(&dir)?.next().is_none() {
                std::fs::remove_dir(&dir)?;
            }
        }
    }

    Ok(())
}
