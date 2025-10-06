//! File discovery module for finding video files to process.
//!
//! This module handles the discovery of video files eligible for processing.
//! Searches for supported video files (case-insensitive) in the top level
//! of the provided directory.

use crate::error::{CoreError, CoreResult};
use crate::utils::{SafePath, is_valid_video_file};

use std::path::{Path, PathBuf};

/// Finds supported video files in the top level of the input directory (case-insensitive).
/// Enhanced with better error reporting and validation.
/// Returns files sorted alphabetically by filename.
pub fn find_processable_files(input_dir: &Path) -> CoreResult<Vec<PathBuf>> {
    // Validate input directory
    if !input_dir.exists() {
        return Err(CoreError::PathError(format!(
            "Directory does not exist: {}",
            input_dir.display()
        )));
    }

    if !input_dir.is_dir() {
        return Err(CoreError::PathError(format!(
            "{} is not a directory",
            input_dir.display()
        )));
    }

    let read_dir = std::fs::read_dir(input_dir).map_err(|e| {
        CoreError::PathError(format!(
            "Cannot read directory {}: {}",
            input_dir.display(),
            e
        ))
    })?;

    let mut files: Vec<PathBuf> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut skipped_count = 0;

    for entry_result in read_dir {
        match entry_result {
            Ok(entry) => {
                let path = entry.path();

                // Skip directories and hidden files
                if path.is_dir() {
                    continue;
                }

                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.starts_with('.') {
                        continue; // Skip hidden files
                    }
                }

                if is_valid_video_file(&path) {
                    files.push(path);
                } else {
                    skipped_count += 1;
                }
            }
            Err(e) => {
                errors.push(format!("Failed to read directory entry: {}", e));
            }
        }
    }

    // Log any errors encountered (non-fatal)
    if !errors.is_empty() {
        log::warn!(
            "Errors during file discovery in {}: {}",
            input_dir.display(),
            errors.join(", ")
        );
    }

    // Log summary
    if skipped_count > 0 {
        log::debug!(
            "Skipped {} non-video files in {}",
            skipped_count,
            input_dir.display()
        );
    }

    if files.is_empty() {
        return Err(CoreError::NoFilesFound);
    }

    // Sort files with improved comparison using SafePath for robustness
    files.sort_by(|a, b| {
        let name_a = SafePath::get_filename_utf8(a)
            .unwrap_or_else(|_| "zzz_invalid_utf8".to_string())
            .to_lowercase();
        let name_b = SafePath::get_filename_utf8(b)
            .unwrap_or_else(|_| "zzz_invalid_utf8".to_string())
            .to_lowercase();
        name_a.cmp(&name_b)
    });

    log::info!(
        "Found {} video files in {}",
        files.len(),
        input_dir.display()
    );
    for (i, file) in files.iter().take(5).enumerate() {
        log::debug!(
            "  {}. {}",
            i + 1,
            SafePath::get_filename_utf8(file).unwrap_or_else(|_| "<invalid UTF-8>".to_string())
        );
    }
    if files.len() > 5 {
        log::debug!("  ... and {} more files", files.len() - 5);
    }

    Ok(files)
}

/// Recursively finds video files in a directory tree (for future use)
/// Currently not used but provides enhanced discovery for deep directory structures
#[allow(dead_code)]
pub fn find_processable_files_recursive(
    input_dir: &Path,
    max_depth: usize,
) -> CoreResult<Vec<PathBuf>> {
    if max_depth == 0 {
        return find_processable_files(input_dir);
    }

    let mut all_files = Vec::new();
    let mut directories_to_search = vec![(input_dir.to_path_buf(), 0)];

    while let Some((current_dir, depth)) = directories_to_search.pop() {
        match find_processable_files(&current_dir) {
            Ok(mut files) => all_files.append(&mut files),
            Err(CoreError::NoFilesFound) => {} // Expected in some directories
            Err(e) => {
                log::warn!(
                    "Failed to search directory {}: {}",
                    current_dir.display(),
                    e
                );
                continue;
            }
        }

        // Add subdirectories if we haven't reached max depth
        if depth < max_depth {
            if let Ok(read_dir) = std::fs::read_dir(&current_dir) {
                for entry_result in read_dir {
                    if let Ok(entry) = entry_result {
                        let path = entry.path();
                        if path.is_dir() {
                            // Skip hidden directories
                            if let Some(dirname) = path.file_name().and_then(|n| n.to_str()) {
                                if !dirname.starts_with('.') {
                                    directories_to_search.push((path, depth + 1));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if all_files.is_empty() {
        Err(CoreError::NoFilesFound)
    } else {
        // Sort the combined results
        all_files.sort_by(|a, b| {
            let name_a = SafePath::get_filename_utf8(a)
                .unwrap_or_else(|_| "zzz_invalid_utf8".to_string())
                .to_lowercase();
            let name_b = SafePath::get_filename_utf8(b)
                .unwrap_or_else(|_| "zzz_invalid_utf8".to_string())
                .to_lowercase();
            name_a.cmp(&name_b)
        });

        log::info!(
            "Found {} video files across directory tree starting from {}",
            all_files.len(),
            input_dir.display()
        );
        Ok(all_files)
    }
}
