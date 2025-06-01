//! Utility functions for formatting and file operations.
//!
//! This module provides general-purpose utility functions used throughout the
//! drapto-core library. These include functions for duration formatting
//! and byte formatting.

/// Formats a duration into a human-readable string in HH:MM:SS format.
///
/// This function accepts a duration in seconds and formats it as a string 
/// in the standard HH:MM:SS format. This format is consistent with the CLI 
/// design guide and provides a uniform way to display durations throughout 
/// the application.
///
/// # Arguments
///
/// * `seconds` - The duration in seconds. Can be obtained from Duration::as_secs_f64()
///              or from external sources like FFmpeg
///
/// # Returns
///
/// * A String in the format "HH:MM:SS" (e.g., "01:30:45")
/// * Returns "??:??:??" for negative or non-finite values
///
/// # Examples
///
/// ```rust
/// use drapto_core::format_duration;
/// use std::time::Duration;
///
/// // From Duration
/// let duration = Duration::from_secs(3725); // 1 hour, 2 minutes, 5 seconds
/// let formatted = format_duration(duration.as_secs_f64());
/// assert_eq!(formatted, "01:02:05");
///
/// // From raw seconds
/// let formatted = format_duration(3725.0);
/// assert_eq!(formatted, "01:02:05");
/// ```
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

/// Formats a byte count into a human-readable string with appropriate units.
///
/// This function converts a raw byte count into a human-readable string with
/// binary units (KiB, MiB, GiB). It automatically selects the appropriate unit
/// based on the size and formats the value with two decimal places for larger units.
///
/// # Arguments
///
/// * `bytes` - The number of bytes to format
///
/// # Returns
///
/// * A String with the formatted byte count and appropriate unit
///   (e.g., "1.50 MiB", "720.00 KiB", "500 B")
///
/// # Examples
///
/// ```rust
/// use drapto_core::format_bytes;
///
/// let size = 1536 * 1024; // 1.5 MiB in bytes
/// let formatted = format_bytes(size);
/// assert_eq!(formatted, "1.50 MiB");
///
/// let small_size = 500;
/// let small_formatted = format_bytes(small_size);
/// assert_eq!(small_formatted, "500 B");
/// ```
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

/// Parses `FFmpeg` time string (HH:MM:SS.MS format) to seconds.
///
/// This function parses time strings in the format used by `FFmpeg` progress
/// output (e.g., "01:23:45.67") and converts them to a floating-point number
/// of seconds. This is useful for parsing `FFmpeg` progress reports and
/// calculating encoding progress.
///
/// # Arguments
///
/// * `time` - A time string in HH:MM:SS.MS format
///
/// # Returns
///
/// * `Some(f64)` containing the time in seconds if parsing succeeds
/// * `None` if the string format is invalid or parsing fails
///
/// # Examples
///
/// ```rust
/// use drapto_core::parse_ffmpeg_time;
///
/// let time = parse_ffmpeg_time("01:23:45.67");
/// assert_eq!(time, Some(5025.67));
///
/// let invalid = parse_ffmpeg_time("invalid");
/// assert_eq!(invalid, None);
/// ```
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
