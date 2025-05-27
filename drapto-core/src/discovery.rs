//! File discovery module for finding video files to process.
//!
//! This module handles the discovery of video files eligible for processing.
//! Currently only searches for .mkv files (case-insensitive) in the top level
//! of the provided directory.


use crate::error::{CoreError, CoreResult};

use std::path::{Path, PathBuf};

/// Finds video files eligible for processing in the specified directory.
///
/// This function scans the top level of the provided directory for .mkv files
/// (case-insensitive) and returns their paths. It does not search subdirectories.
///
/// # Arguments
///
/// * `input_dir` - The directory to search for video files
///
/// # Returns
///
/// * `Ok(Vec<PathBuf>)` - A vector of paths to the discovered .mkv files
/// * `Err(CoreError::Walkdir)` - If an error occurs during directory traversal
/// * `Err(CoreError::NoFilesFound)` - If no .mkv files are found
///
/// # Examples
///
/// ```rust,no_run
/// use drapto_core::find_processable_files;
/// use std::path::Path;
///
/// let input_dir = Path::new("/path/to/videos");
/// match find_processable_files(input_dir) {
///     Ok(files) => {
///         println!("Found {} video files:", files.len());
///         for file in files {
///             println!("  {}", file.display());
///         }
///     },
///     Err(e) => println!("Error finding video files: {}", e),
/// }
/// ```
pub fn find_processable_files(input_dir: &Path) -> CoreResult<Vec<PathBuf>> {
    let read_dir = std::fs::read_dir(input_dir)?;
    let files: Vec<PathBuf> = read_dir
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();

            if !path.is_file() {
                return None;
            }

            path.extension()
                .and_then(|ext| ext.to_str())
                .filter(|ext_str| ext_str.eq_ignore_ascii_case("mkv"))
                .map(|_| path.clone())
        })
        .collect();

    if files.is_empty() {
        Err(CoreError::NoFilesFound)
    } else {
        Ok(files)
    }
}
