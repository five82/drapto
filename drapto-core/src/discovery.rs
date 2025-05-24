// ============================================================================
// drapto-core/src/discovery.rs
// ============================================================================
//
// FILE DISCOVERY: Finding Video Files for Processing
//
// This module handles the discovery of video files eligible for processing.
// It provides functions to scan directories and identify files that match
// specific criteria for video encoding.
//
// KEY COMPONENTS:
// - find_processable_files: Main function to find .mkv files in a directory
//
// DESIGN NOTES:
// - Currently only searches for .mkv files (case-insensitive)
// - Only searches the top level of the provided directory (no recursion)
// - Returns a CoreError::NoFilesFound if no matching files are found
//
// FUTURE ENHANCEMENTS:
// - Support for additional video formats (e.g., .mp4, .avi)
// - Optional recursive directory scanning
// - Filtering based on file size or other criteria
//
// AI-ASSISTANT-INFO: File discovery module for finding video files to process

// ---- Internal crate imports ----
use crate::error::{CoreError, CoreResult};


// ---- Standard library imports ----
use std::path::{Path, PathBuf};

// ============================================================================
// PUBLIC FUNCTIONS
// ============================================================================

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
    // Read directory entries using standard library
    let read_dir = std::fs::read_dir(input_dir)?;
    
    // Filter the entries to find only .mkv files
    let files: Vec<PathBuf> = read_dir
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            
            // Only include files (not directories)
            if !path.is_file() {
                return None;
            }
            
            // Check for .mkv extension (case-insensitive)
            path.extension()
                .and_then(|ext| ext.to_str())
                .filter(|ext_str| ext_str.eq_ignore_ascii_case("mkv"))
                .map(|_| path.clone())
        })
        .collect();

    // Return an error if no files were found, otherwise return the files
    if files.is_empty() {
        Err(CoreError::NoFilesFound)
    } else {
        Ok(files)
    }
}
