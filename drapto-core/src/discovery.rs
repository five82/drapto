//! File discovery module for finding video files to process.
//!
//! This module handles the discovery of video files eligible for processing.
//! Currently only searches for .mkv files (case-insensitive) in the top level
//! of the provided directory.


use crate::error::{CoreError, CoreResult};

use std::path::{Path, PathBuf};

/// Finds .mkv files in the top level of the input directory (case-insensitive).
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
