//! Utility functions for formatting and file operations.
//!
//! This module provides general-purpose utility functions used throughout the
//! drapto-core library. These include functions for duration formatting,
//! byte formatting, and path manipulation.

/// Formats seconds as HH:MM:SS (e.g., 3725.0 -> "01:02:05"). Returns "??:??:??" for invalid inputs.
#[must_use] pub fn format_duration(seconds: f64) -> String {
    if seconds < 0.0 || !seconds.is_finite() {
        return "??:??:??".to_string();
    }

    let total_seconds = seconds as u64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let secs = total_seconds % 60;
    format!("{hours:02}:{minutes:02}:{secs:02}")
}

/// Formats bytes with appropriate binary units (B, KiB, MiB, GiB).
#[must_use] pub fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;

    let bytes_f64 = bytes as f64;
    if bytes_f64 >= GIB {
        format!("{:.2} GiB", bytes_f64 / GIB)
    } else if bytes_f64 >= MIB {
        format!("{:.2} MiB", bytes_f64 / MIB)
    } else if bytes_f64 >= KIB {
        format!("{:.2} KiB", bytes_f64 / KIB)
    } else {
        format!("{bytes} B")
    }
}

/// Parses FFmpeg time string (HH:MM:SS.MS) to seconds. Returns None if invalid.
#[must_use] pub fn parse_ffmpeg_time(time: &str) -> Option<f64> {
    let parts: Vec<&str> = time.split(':').collect();
    if parts.len() == 3 {
        let hours = parts[0].parse::<f64>().ok()?;
        let minutes = parts[1].parse::<f64>().ok()?;
        let seconds = parts[2].parse::<f64>().ok()?;
        Some(hours * 3600.0 + minutes * 60.0 + seconds)
    } else {
        None
    }
}

/// Safely extracts filename from a path with consistent error handling.
/// Returns the filename as a String, or an error if the path has no filename component.
pub fn get_filename_safe(path: &std::path::Path) -> crate::CoreResult<String> {
    Ok(path.file_name()
        .ok_or_else(|| {
            crate::CoreError::PathError(format!(
                "Failed to get filename for {}",
                path.display()
            ))
        })?
        .to_string_lossy()
        .to_string())
}
