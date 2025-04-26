// drapto-core/src/utils.rs
//
// Provides general-purpose utility functions for file size retrieval,
// duration formatting, and byte formatting used across `drapto-core`.

use crate::error::CoreResult;
use std::fs;
use std::path::Path;
use std::time::Duration;

/// Gets the size of a file.
pub(crate) fn get_file_size(path: &Path) -> CoreResult<u64> {
    Ok(fs::metadata(path)?.len())
}

/// Formats duration into Hh Mm Ss format
pub fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{}h {}m {}s", hours, minutes, seconds)
}

/// Formats bytes into human-readable format (KiB, MiB, GiB)
pub fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;

    if bytes as f64 >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB)
    } else if bytes as f64 >= MIB {
        format!("{:.2} MiB", bytes as f64 / MIB)
    } else if bytes as f64 >= KIB {
        format!("{:.2} KiB", bytes as f64 / KIB)
    } else {
        format!("{} B", bytes)
    }
}