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
// DESIGN PHILOSOPHY:
// This module leverages the tempfile crate to handle automatic cleanup via
// the Drop trait, ensuring that temporary files are properly cleaned up even
// in error cases. It provides a simple interface for temporary file operations.
//
// AI-ASSISTANT-INFO: Temporary file management utilities

use crate::config::CoreConfig;
use crate::error::CoreResult;
use std::path::{Path, PathBuf};
use tempfile::{self, Builder as TempFileBuilder, NamedTempFile, TempDir};

// ============================================================================
// TEMPORARY DIRECTORY FUNCTIONS
// ============================================================================

/// Creates a temporary directory within the configured temp directory.
/// The directory will be automatically cleaned up when the returned TempDir is dropped.
///
/// # Arguments
///
/// * `config` - The core configuration containing path information
/// * `prefix` - A prefix to use for the temporary directory name
///
/// # Returns
///
/// * `CoreResult<TempDir>` - A temporary directory that will be automatically
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
///     let temp_dir = temp_files::create_temp_dir(&config, "analysis_")?;
///     let temp_dir_path = temp_dir.path();
///     // Use temp_dir_path for operations
///     // temp_dir is automatically cleaned up when it goes out of scope
///     Ok(())
/// }
/// ```
pub fn create_temp_dir(config: &CoreConfig, prefix: &str) -> CoreResult<TempDir> {
    let temp_base_dir = config.temp_dir.as_ref().unwrap_or(&config.output_dir);
    std::fs::create_dir_all(temp_base_dir)?;

    Ok(TempFileBuilder::new()
        .prefix(prefix)
        .tempdir_in(temp_base_dir)?)
}

/// Creates a temporary directory for grain analysis samples.
/// This is a convenience function that calls create_temp_dir with a specific prefix.
///
/// # Arguments
///
/// * `config` - The core configuration containing path information
///
/// # Returns
///
/// * `CoreResult<TempDir>` - A temporary directory that will be automatically
///   cleaned up when dropped
pub fn create_grain_analysis_dir(config: &CoreConfig) -> CoreResult<TempDir> {
    create_temp_dir(config, "grain_analysis_")
}

/// Creates a temporary directory for analysis operations.
/// This is a convenience function that calls create_temp_dir with the provided prefix.
///
/// # Arguments
///
/// * `config` - The core configuration containing path information
/// * `prefix` - A prefix to use for the temporary directory name
///
/// # Returns
///
/// * `CoreResult<TempDir>` - A temporary directory that will be automatically
///   cleaned up when dropped
pub fn create_analysis_dir(config: &CoreConfig, prefix: &str) -> CoreResult<TempDir> {
    create_temp_dir(config, prefix)
}

// ============================================================================
// TEMPORARY FILE FUNCTIONS
// ============================================================================

/// Creates a temporary file with the given prefix and extension.
/// The file will be created in the specified directory and will be automatically
/// deleted when the returned NamedTempFile is dropped.
///
/// # Arguments
///
/// * `dir` - The directory in which to create the temporary file
/// * `prefix` - A prefix to use for the temporary file name
/// * `extension` - The file extension to use (without the dot)
///
/// # Returns
///
/// * `CoreResult<NamedTempFile>` - A temporary file that will be automatically
///   cleaned up when dropped
///
/// # Example
///
/// ```rust,no_run
/// use drapto_core::temp_files;
/// use std::path::Path;
/// use std::io::Write;
///
/// fn example() -> Result<(), Box<dyn std::error::Error>> {
///     let temp_dir_path = Path::new("/tmp");
///     let mut temp_file = temp_files::create_temp_file(temp_dir_path, "config", "json")?;
///
///     // Write to the temporary file
///     writeln!(temp_file, "{{\"key\": \"value\"}}")?;
///
///     // Get the path to the temporary file
///     let temp_file_path = temp_file.path().to_path_buf();
///
///     // The file will be automatically deleted when temp_file goes out of scope
///     Ok(())
/// }
/// ```
pub fn create_temp_file(dir: &Path, prefix: &str, extension: &str) -> CoreResult<NamedTempFile> {
    // Ensure the directory exists
    std::fs::create_dir_all(dir)?;

    // Create a temporary file with the given prefix and extension
    let temp_file = TempFileBuilder::new()
        .prefix(&format!("{}_", prefix))
        .suffix(&format!(".{}", extension))
        .tempfile_in(dir)?;

    Ok(temp_file)
}

/// Creates a temporary file path within a directory.
/// Unlike create_temp_file, this function only returns a path and does not create a file.
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
    use rand::distributions::Alphanumeric;
    use rand::{Rng, thread_rng};

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

// ============================================================================
// CLEANUP FUNCTIONS
// ============================================================================

/// Cleans up any empty temporary directories in the configured temp directory.
/// This is optional and can be called at the end of processing.
///
/// Note: Most temporary directories are automatically cleaned up when their TempDir
/// objects are dropped, so this function is mainly useful for cleaning up any
/// directories that might have been left behind due to process crashes or other issues.
///
/// # Arguments
///
/// * `config` - The core configuration containing path information
///
/// # Returns
///
/// * `CoreResult<()>` - Ok if cleanup was successful, Err otherwise
pub fn cleanup_base_dirs(config: &CoreConfig) -> CoreResult<()> {
    // Get the base temporary directory
    let temp_base_dir = config.temp_dir.as_ref().unwrap_or(&config.output_dir);

    // If the directory doesn't exist, there's nothing to clean up
    if !temp_base_dir.exists() {
        return Ok(());
    }

    // Iterate through all entries in the temp directory
    for entry in std::fs::read_dir(temp_base_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Only process directories
        if path.is_dir() {
            // Check if the directory name starts with a known prefix
            let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
            if dir_name.starts_with("grain_analysis_")
                || dir_name.starts_with("crop_analysis_")
                || dir_name.starts_with("analysis_")
            {
                // Check if directory is empty
                if std::fs::read_dir(&path)?.next().is_none() {
                    log::debug!("Removing empty temporary directory: {}", path.display());
                    std::fs::remove_dir(&path)?;
                } else {
                    log::debug!(
                        "Temporary directory not empty, skipping cleanup: {}",
                        path.display()
                    );
                }
            }
        }
    }

    Ok(())
}
