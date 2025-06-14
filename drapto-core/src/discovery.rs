//! File discovery module for finding video files to process.
//!
//! This module handles the discovery of video files eligible for processing.
//! Currently only searches for .mkv files (case-insensitive) in the top level
//! of the provided directory.


use crate::error::{CoreError, CoreResult};
use crate::utils::is_valid_video_file;

use std::path::{Path, PathBuf};

/// Finds .mkv files in the top level of the input directory (case-insensitive).
pub fn find_processable_files(input_dir: &Path) -> CoreResult<Vec<PathBuf>> {
    let read_dir = std::fs::read_dir(input_dir)?;
    let files: Vec<PathBuf> = read_dir
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

    if files.is_empty() {
        Err(CoreError::NoFilesFound)
    } else {
        Ok(files)
    }
}
