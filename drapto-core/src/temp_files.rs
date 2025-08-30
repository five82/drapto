//! Temporary file management utilities.
//!
//! This module provides helper functions for creating and managing temporary
//! files and directories. It leverages the tempfile crate to handle automatic
//! cleanup via the Drop trait, ensuring proper cleanup even in error cases.

use crate::config::CoreConfig;
use crate::error::{CoreResult, CoreError};
use crate::utils::SafePath;
use std::path::{Path, PathBuf};
use tempfile::{self, Builder as TempFileBuilder, NamedTempFile, TempDir};

/// Minimum free space required for temporary operations (in MB)
const MIN_TEMP_SPACE_MB: u64 = 100;

/// Creates a temporary directory with prefix and enhanced safety checks.
/// Auto-cleaned when dropped.
pub fn create_temp_dir(config: &CoreConfig, prefix: &str) -> CoreResult<TempDir> {
    let temp_base = config.temp_dir.as_ref().unwrap_or(&config.output_dir);
    
    // Validate base directory is writable
    SafePath::ensure_directory_writable(temp_base)?;
    
    // Check available disk space (warning only)
    if let Some(available_bytes) = SafePath::get_available_space(temp_base) {
        let available_mb = available_bytes / (1024 * 1024);
        if available_mb < MIN_TEMP_SPACE_MB {
            log::warn!(
                "Low disk space in temp directory {}: {} MB available (minimum recommended: {} MB)",
                temp_base.display(),
                available_mb,
                MIN_TEMP_SPACE_MB
            );
        }
    }
    
    let temp_dir = TempFileBuilder::new()
        .prefix(&format!("{}_", prefix))
        .tempdir_in(temp_base)
        .map_err(|e| CoreError::OperationFailed(
            format!("Failed to create temp directory in {}: {}", temp_base.display(), e)
        ))?;
    
    log::debug!("Created temp directory: {}", temp_dir.path().display());
    Ok(temp_dir)
}



/// Creates a temporary file with prefix and extension. Auto-deleted when dropped.
pub fn create_temp_file(dir: &Path, prefix: &str, extension: &str) -> CoreResult<NamedTempFile> {
    SafePath::ensure_directory_writable(dir)?;
    
    let temp_file = TempFileBuilder::new()
        .prefix(&format!("{}_", prefix))
        .suffix(&format!(".{}", extension))
        .tempfile_in(dir)
        .map_err(|e| CoreError::OperationFailed(
            format!("Failed to create temp file in {}: {}", dir.display(), e)
        ))?;
    
    log::debug!("Created temp file: {}", temp_file.path().display());
    Ok(temp_file)
}

/// Returns a temporary file path with random suffix. Does not create the file.
/// Validates the directory exists and is writable first.
pub fn create_temp_file_path(dir: &Path, prefix: &str, extension: &str) -> CoreResult<PathBuf> {
    use rand::distr::Alphanumeric;
    use rand::prelude::*;
    
    SafePath::ensure_directory_writable(dir)?;

    let mut rng = rand::rng();
    let random_suffix: String = (0..8) // Increased from 6 to 8 for better uniqueness
        .map(|_| rng.sample(Alphanumeric) as char)
        .collect();

    let filename = format!("{}_{}.{}", prefix, random_suffix, extension);
    let temp_path = dir.join(filename);
    
    // Ensure the path doesn't already exist (extremely unlikely but safer)
    if temp_path.exists() {
        log::warn!("Temporary file path already exists: {}, retrying...", temp_path.display());
        return create_temp_file_path(dir, prefix, extension); // Recursive retry
    }
    
    Ok(temp_path)
}

/// Clean up stale temporary files in a directory (best effort)
pub fn cleanup_stale_temp_files(dir: &Path, prefix: &str, max_age_hours: u64) -> CoreResult<usize> {
    if !dir.exists() {
        return Ok(0);
    }
    
    let mut cleaned_count = 0;
    let max_age = std::time::Duration::from_secs(max_age_hours * 3600);
    let now = std::time::SystemTime::now();
    
    match std::fs::read_dir(dir) {
        Ok(entries) => {
            for entry_result in entries {
                if let Ok(entry) = entry_result {
                    let path = entry.path();
                    
                    // Check if filename starts with our prefix
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        if filename.starts_with(&format!("{}_", prefix)) {
                            // Check file age
                            if let Ok(metadata) = entry.metadata() {
                                if let Ok(modified) = metadata.modified() {
                                    if let Ok(age) = now.duration_since(modified) {
                                        if age > max_age {
                                            match std::fs::remove_file(&path) {
                                                Ok(()) => {
                                                    log::debug!("Cleaned up stale temp file: {}", path.display());
                                                    cleaned_count += 1;
                                                }
                                                Err(e) => {
                                                    log::warn!("Failed to remove stale temp file {}: {}", path.display(), e);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            log::warn!("Failed to read temp directory for cleanup: {}", e);
        }
    }
    
    if cleaned_count > 0 {
        log::info!("Cleaned up {} stale temporary files", cleaned_count);
    }
    
    Ok(cleaned_count)
}

