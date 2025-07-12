//! File discovery module for finding video files to process.
//!
//! This module handles the discovery of video files eligible for processing.
//! Searches for supported video files (case-insensitive) in the top level
//! of the provided directory.


use crate::error::{CoreError, CoreResult};
use crate::utils::is_valid_video_file;

use std::path::{Path, PathBuf};

/// Finds supported video files in the top level of the input directory (case-insensitive).
/// Returns files sorted alphabetically by filename.
pub fn find_processable_files(input_dir: &Path) -> CoreResult<Vec<PathBuf>> {
    let read_dir = std::fs::read_dir(input_dir)?;
    let mut files: Vec<PathBuf> = read_dir
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();

            if is_valid_video_file(&path) {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    // Sort files alphabetically by filename
    files.sort_by(|a, b| {
        a.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_lowercase()
            .cmp(&b.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("")
                .to_lowercase())
    });

    if files.is_empty() {
        Err(CoreError::NoFilesFound)
    } else {
        Ok(files)
    }
}
