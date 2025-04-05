// drapto-core/src/discovery.rs
//
// This module handles the discovery of video files eligible for processing.
//
// The primary function, `find_processable_files`, scans a given input directory
// for files matching specific criteria (currently, only `.mkv` files, case-insensitive).
// It uses the `walkdir` crate to iterate through directory entries but is configured
// to only search the top level (`max_depth(1)`) of the provided path.
//
// It returns a `CoreResult` containing either a vector of `PathBuf`s for the
// found `.mkv` files or a `CoreError` if an issue occurs during directory traversal
// (`CoreError::Walkdir`) or if no `.mkv` files are found (`CoreError::NoFilesFound`).

use crate::error::{CoreError, CoreResult}; // Use crate:: prefix
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Finds processable video files (currently hardcoded to .mkv).
pub fn find_processable_files(input_dir: &Path) -> CoreResult<Vec<PathBuf>> {
    // Use collect to handle potential WalkDir errors first
    let entries: Vec<walkdir::DirEntry> = WalkDir::new(input_dir)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .collect::<Result<Vec<_>, _>>() // Collect results, propagating the first error
        .map_err(CoreError::Walkdir)?; // Map walkdir::Error to CoreError::Walkdir

    let files: Vec<PathBuf> = entries
        .into_iter()
        .filter(|e| e.file_type().is_file())
        .filter_map(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str()) // Ensure extension is valid UTF-8
                .filter(|ext_str| ext_str.eq_ignore_ascii_case("mkv"))
                .map(|_| entry.path().to_path_buf()) // If it's an mkv, keep the path
        })
        .collect();

    if files.is_empty() {
        // If entries were successfully collected but no MKV files were found
        Err(CoreError::NoFilesFound)
    } else {
        Ok(files)
    }
}