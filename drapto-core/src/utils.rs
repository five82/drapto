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

// ---- Internal crate imports ----
use crate::error::CoreResult;

// ---- Standard library imports ----
use std::fs;
use std::path::Path;
use std::time::Duration;

// ============================================================================
// FILE OPERATIONS
// ============================================================================

/// Gets the size of a file in bytes.
///
/// This function retrieves the size of the file at the specified path using
/// the standard library's fs::metadata function.
///
/// # Arguments
///
/// * `path` - Path to the file to get the size of
///
/// # Returns
///
/// * `Ok(u64)` - The size of the file in bytes
/// * `Err(CoreError::Io)` - If an error occurs accessing the file
///
/// # Examples
///
/// ```rust,no_run
/// // This function is internal to the crate, so we can't call it directly in doctests
/// // Example usage within the crate:
/// // use std::path::Path;
/// //
/// // let path = Path::new("/path/to/file.mkv");
/// // match get_file_size(path) {
/// //     Ok(size) => println!("File size: {} bytes", size),
/// //     Err(e) => eprintln!("Error getting file size: {}", e),
/// // }
/// ```
pub(crate) fn get_file_size(path: &Path) -> CoreResult<u64> {
    // Get the file metadata and extract the size
    Ok(fs::metadata(path)?.len())
}

// ============================================================================
// FORMATTING FUNCTIONS
// ============================================================================

/// Formats a Duration into a human-readable string in the format "Xh Ym Zs".
///
/// This function converts a Duration into hours, minutes, and seconds and
/// formats it as a string. It's useful for displaying encoding times and
/// other durations in a user-friendly way.
///
/// # Arguments
///
/// * `duration` - The Duration to format
///
/// # Returns
///
/// * A String in the format "Xh Ym Zs" (e.g., "1h 30m 45s")
///
/// # Examples
///
/// ```rust
/// use drapto_core::format_duration;
/// use std::time::Duration;
///
/// let duration = Duration::from_secs(3725); // 1 hour, 2 minutes, 5 seconds
/// let formatted = format_duration(duration);
/// assert_eq!(formatted, "1h 2m 5s");
/// ```
pub fn format_duration(duration: Duration) -> String {
    // Convert the duration to total seconds
    let total_seconds = duration.as_secs();

    // Calculate hours, minutes, and seconds
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    // Format as "Xh Ym Zs"
    format!("{}h {}m {}s", hours, minutes, seconds)
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
