//! Temporary file management utilities.
//!
//! This module provides helper functions for creating and managing temporary
//! files and directories. It leverages the tempfile crate to handle automatic
//! cleanup via the Drop trait, ensuring proper cleanup even in error cases.

use crate::config::CoreConfig;
use crate::error::CoreResult;
use std::path::{Path, PathBuf};
use tempfile::{self, Builder as TempFileBuilder, NamedTempFile, TempDir};

/// Creates a temporary directory with prefix. Auto-cleaned when dropped.
pub fn create_temp_dir(config: &CoreConfig, prefix: &str) -> CoreResult<TempDir> {
    let temp_base_dir = config.temp_dir.as_ref().unwrap_or(&config.output_dir);
    std::fs::create_dir_all(temp_base_dir)?;

    Ok(TempFileBuilder::new()
        .prefix(prefix)
        .tempdir_in(temp_base_dir)?)
}



/// Creates a temporary file with prefix and extension. Auto-deleted when dropped.
pub fn create_temp_file(dir: &Path, prefix: &str, extension: &str) -> CoreResult<NamedTempFile> {
    std::fs::create_dir_all(dir)?;
    let temp_file = TempFileBuilder::new()
        .prefix(&format!("{prefix}_"))
        .suffix(&format!(".{extension}"))
        .tempfile_in(dir)?;

    Ok(temp_file)
}

/// Returns a temporary file path with random suffix. Does not create the file.
pub fn create_temp_file_path(dir: &Path, prefix: &str, extension: &str) -> PathBuf {
    use rand::distributions::Alphanumeric;
    use rand::{Rng, thread_rng};

    let random_suffix: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();

    let filename = format!("{prefix}_{random_suffix}.{extension}");
    dir.join(filename)
}

