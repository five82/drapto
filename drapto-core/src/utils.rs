// ============================================================================
// drapto-core/src/utils.rs
// ============================================================================
//
// UTILITY FUNCTIONS: Common Helper Functions
//
// This module provides general-purpose utility functions used throughout the
// drapto-core library. These include functions for file size retrieval,
// duration formatting, and byte formatting.
//
// KEY COMPONENTS:
// - get_file_size: Retrieves the size of a file
// - format_duration: Formats a Duration into a human-readable string
// - format_bytes: Formats byte counts into human-readable units
//
// DESIGN PHILOSOPHY:
// These utility functions are designed to be simple, reusable, and focused on
// a single responsibility. They help maintain consistency in how common operations
// are performed throughout the codebase.
//
// AI-ASSISTANT-INFO: Utility functions for formatting and file operations

// ---- Standard library imports ----
use std::time::Duration;

// ============================================================================
// FORMATTING FUNCTIONS
// ============================================================================

/// Formats a Duration into a human-readable string in HH:MM:SS format.
///
/// This function converts a Duration into hours, minutes, and seconds and
/// formats it as a string in the standard HH:MM:SS format. This format is
/// consistent with the CLI design guide and provides a uniform way to display
/// durations throughout the application.
///
/// # Arguments
///
/// * `duration` - The Duration to format
///
/// # Returns
///
/// * A String in the format "HH:MM:SS" (e.g., "01:30:45")
///
/// # Examples
///
/// ```rust
/// use drapto_core::format_duration;
/// use std::time::Duration;
///
/// let duration = Duration::from_secs(3725); // 1 hour, 2 minutes, 5 seconds
/// let formatted = format_duration(duration);
/// assert_eq!(formatted, "01:02:05");
/// ```
pub fn format_duration(duration: Duration) -> String {
    format_duration_seconds(duration.as_secs_f64())
}

/// Formats a duration in seconds as HH:MM:SS.
///
/// This function converts a floating-point number of seconds into a
/// formatted string in HH:MM:SS format. It's useful when working with
/// duration values that come from external sources (like FFmpeg) as
/// floating-point seconds.
///
/// # Arguments
///
/// * `seconds` - The duration in seconds
///
/// # Returns
///
/// * A String in the format "HH:MM:SS"
///
/// # Examples
///
/// ```rust
/// use drapto_core::utils::format_duration_seconds;
///
/// let formatted = format_duration_seconds(3725.0);
/// assert_eq!(formatted, "01:02:05");
/// ```
pub fn format_duration_seconds(seconds: f64) -> String {
    // Handle invalid values
    if seconds < 0.0 || !seconds.is_finite() {
        return "??:??:??".to_string();
    }

    let total_seconds = seconds as u64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let secs = total_seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, secs)
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
pub fn format_bytes(bytes: u64) -> String {
    // Define binary unit constants
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;

    // Convert bytes to f64 for division
    let bytes_f64 = bytes as f64;

    // Select the appropriate unit based on size
    if bytes_f64 >= GIB {
        // Format as GiB with 2 decimal places
        format!("{:.2} GiB", bytes_f64 / GIB)
    } else if bytes_f64 >= MIB {
        // Format as MiB with 2 decimal places
        format!("{:.2} MiB", bytes_f64 / MIB)
    } else if bytes_f64 >= KIB {
        // Format as KiB with 2 decimal places
        format!("{:.2} KiB", bytes_f64 / KIB)
    } else {
        // Format as bytes with no decimal places
        format!("{} B", bytes)
    }
}
