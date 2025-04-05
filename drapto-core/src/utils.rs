// drapto-core/src/utils.rs
//
// This module provides general-purpose utility functions used across the
// `drapto-core` library. These functions are not tied to a specific domain
// like encoding or discovery but offer common helper functionalities.
//
// Includes:
// - `get_file_size`: A private helper function (`pub(crate)`) to retrieve the
//   size of a file in bytes, returning a `CoreResult`.
// - `format_duration`: A public function (`pub`) that takes a `std::time::Duration`
//   and formats it into a human-readable string (e.g., "1h 23m 45s").
// - `format_bytes`: A public function (`pub`) that takes a number of bytes (u64)
//   and formats it into a human-readable string with appropriate binary prefixes
//   (B, KiB, MiB, GiB).

use crate::error::CoreResult; // Use crate:: prefix
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