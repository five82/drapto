//! Utility functions for formatting and file operations.
//!
//! This module provides general-purpose utility functions used throughout the
//! drapto-core library. These include functions for duration formatting,
//! byte formatting, and path manipulation.

use std::path::Path;

/// Checks if the given path is a valid video file that can be processed.
/// Currently only supports .mkv files (case-insensitive).
#[must_use]
pub fn is_valid_video_file(path: &Path) -> bool {
    path.is_file() && 
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext_str| ext_str.eq_ignore_ascii_case("mkv"))
        .unwrap_or(false)
}

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

/// Calculates the percentage size reduction from input to output.
/// Returns 0 if input_size is 0 to avoid division by zero.
#[must_use] pub fn calculate_size_reduction(input_size: u64, output_size: u64) -> u64 {
    if input_size == 0 {
        0
    } else if output_size >= input_size {
        0  // No reduction if output is larger
    } else {
        100 - ((output_size * 100) / input_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_video_file() {
        use std::fs::File;
        
        // Create a temporary test file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_video.mkv");
        let test_file_upper = temp_dir.join("test_video.MKV");
        let test_file_mixed = temp_dir.join("test_video.Mkv");
        let test_file_mp4 = temp_dir.join("test_video.mp4");
        
        // Create test files
        let _ = File::create(&test_file);
        let _ = File::create(&test_file_upper);
        let _ = File::create(&test_file_mixed);
        let _ = File::create(&test_file_mp4);
        
        // Test valid MKV files (case insensitive)
        assert!(is_valid_video_file(&test_file));
        assert!(is_valid_video_file(&test_file_upper));
        assert!(is_valid_video_file(&test_file_mixed));
        
        // Test invalid files
        assert!(!is_valid_video_file(&test_file_mp4));
        assert!(!is_valid_video_file(Path::new("test.mkv"))); // Non-existent file
        assert!(!is_valid_video_file(Path::new("test.mp4")));
        assert!(!is_valid_video_file(Path::new("test.avi")));
        assert!(!is_valid_video_file(Path::new("test.txt")));
        assert!(!is_valid_video_file(Path::new("test")));
        assert!(!is_valid_video_file(Path::new("")));
        
        // Test directories
        assert!(!is_valid_video_file(Path::new("/")));
        assert!(!is_valid_video_file(&temp_dir));
        
        // Cleanup
        let _ = std::fs::remove_file(&test_file);
        let _ = std::fs::remove_file(&test_file_upper);
        let _ = std::fs::remove_file(&test_file_mixed);
        let _ = std::fs::remove_file(&test_file_mp4);
    }

    #[test]
    fn test_format_duration() {
        // Test normal cases
        assert_eq!(format_duration(0.0), "00:00:00");
        assert_eq!(format_duration(59.0), "00:00:59");
        assert_eq!(format_duration(60.0), "00:01:00");
        assert_eq!(format_duration(3599.0), "00:59:59");
        assert_eq!(format_duration(3600.0), "01:00:00");
        assert_eq!(format_duration(3661.0), "01:01:01");
        assert_eq!(format_duration(86399.0), "23:59:59");
        assert_eq!(format_duration(86400.0), "24:00:00");
        assert_eq!(format_duration(90061.0), "25:01:01");
        
        // Test fractional seconds (should truncate)
        assert_eq!(format_duration(59.9), "00:00:59");
        assert_eq!(format_duration(60.1), "00:01:00");
        
        // Test invalid inputs
        assert_eq!(format_duration(-1.0), "??:??:??");
        assert_eq!(format_duration(f64::INFINITY), "??:??:??");
        assert_eq!(format_duration(f64::NEG_INFINITY), "??:??:??");
        assert_eq!(format_duration(f64::NAN), "??:??:??");
    }

    #[test]
    fn test_format_bytes() {
        // Test bytes
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1), "1 B");
        assert_eq!(format_bytes(1023), "1023 B");
        
        // Test KiB
        assert_eq!(format_bytes(1024), "1.00 KiB");
        assert_eq!(format_bytes(1536), "1.50 KiB");
        assert_eq!(format_bytes(1024 * 1023), "1023.00 KiB");
        
        // Test MiB
        assert_eq!(format_bytes(1024 * 1024), "1.00 MiB");
        assert_eq!(format_bytes(1024 * 1024 * 2), "2.00 MiB");
        assert_eq!(format_bytes(1024 * 1024 * 1023), "1023.00 MiB");
        
        // Test GiB
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GiB");
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 2), "2.00 GiB");
        assert_eq!(format_bytes(u64::MAX), "17179869184.00 GiB");
    }

    #[test]
    fn test_parse_ffmpeg_time() {
        // Test valid times
        assert_eq!(parse_ffmpeg_time("00:00:00"), Some(0.0));
        assert_eq!(parse_ffmpeg_time("00:00:01"), Some(1.0));
        assert_eq!(parse_ffmpeg_time("00:01:00"), Some(60.0));
        assert_eq!(parse_ffmpeg_time("01:00:00"), Some(3600.0));
        assert_eq!(parse_ffmpeg_time("01:02:03"), Some(3723.0));
        assert_eq!(parse_ffmpeg_time("23:59:59"), Some(86399.0));
        
        // Test with fractional seconds
        assert_eq!(parse_ffmpeg_time("00:00:00.5"), Some(0.5));
        assert_eq!(parse_ffmpeg_time("00:00:01.25"), Some(1.25));
        assert_eq!(parse_ffmpeg_time("01:30:45.75"), Some(5445.75));
        
        // Test invalid formats
        assert_eq!(parse_ffmpeg_time(""), None);
        assert_eq!(parse_ffmpeg_time("00:00"), None);
        assert_eq!(parse_ffmpeg_time("00:00:00:00"), None);
        assert_eq!(parse_ffmpeg_time("invalid"), None);
        assert_eq!(parse_ffmpeg_time("00:60:00"), Some(3600.0)); // FFmpeg allows this
        assert_eq!(parse_ffmpeg_time("00:00:60"), Some(60.0)); // FFmpeg allows this
        assert_eq!(parse_ffmpeg_time("aa:bb:cc"), None);
    }

    #[test]
    fn test_get_filename_safe() {
        // Test valid paths
        assert_eq!(
            get_filename_safe(Path::new("/path/to/file.mkv")).unwrap(),
            "file.mkv"
        );
        assert_eq!(
            get_filename_safe(Path::new("file.mkv")).unwrap(),
            "file.mkv"
        );
        assert_eq!(
            get_filename_safe(Path::new("./file.mkv")).unwrap(),
            "file.mkv"
        );
        
        // Test edge cases
        assert!(get_filename_safe(Path::new("/")).is_err());
        assert!(get_filename_safe(Path::new("")).is_err());
    }

    #[test]
    fn test_calculate_size_reduction() {
        // Normal cases
        assert_eq!(calculate_size_reduction(100, 50), 50);
        assert_eq!(calculate_size_reduction(1000, 250), 75);
        assert_eq!(calculate_size_reduction(1000, 999), 1); // 1% reduction (rounds down)
        assert_eq!(calculate_size_reduction(1000, 1), 100); // Integer division rounds down
        assert_eq!(calculate_size_reduction(1000, 0), 100);
        
        // Edge cases
        assert_eq!(calculate_size_reduction(0, 0), 0);
        assert_eq!(calculate_size_reduction(0, 100), 0);
        assert_eq!(calculate_size_reduction(100, 100), 0);
        assert_eq!(calculate_size_reduction(100, 150), 0); // Output larger
        assert_eq!(calculate_size_reduction(u64::MAX, 0), 100);
    }
}
