// drapto-core/tests/utils_tests.rs

use drapto_core::utils::{format_bytes, format_duration}; // Import necessary functions
use std::time::Duration;

#[test]
fn test_format_duration() {
    assert_eq!(format_duration(Duration::from_secs(0)), "0h 0m 0s");
    assert_eq!(format_duration(Duration::from_secs(59)), "0h 0m 59s");
    assert_eq!(format_duration(Duration::from_secs(60)), "0h 1m 0s");
    assert_eq!(format_duration(Duration::from_secs(61)), "0h 1m 1s");
    assert_eq!(format_duration(Duration::from_secs(3599)), "0h 59m 59s");
    assert_eq!(format_duration(Duration::from_secs(3600)), "1h 0m 0s");
    assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m 1s");
    assert_eq!(
        format_duration(Duration::from_secs(3600 * 2 + 60 * 30 + 15)),
        "2h 30m 15s"
    );
}

#[test]
fn test_format_bytes() {
    assert_eq!(format_bytes(0), "0 B");
    assert_eq!(format_bytes(1023), "1023 B");
    assert_eq!(format_bytes(1024), "1.00 KiB");
    assert_eq!(format_bytes(1536), "1.50 KiB");
    assert_eq!(format_bytes(1024 * 1024 - 1), "1024.00 KiB"); // Check rounding
    assert_eq!(format_bytes(1024 * 1024), "1.00 MiB");
    assert_eq!(format_bytes(1024 * 1024 * 1536 / 1024), "1.50 MiB");
    assert_eq!(format_bytes(1024 * 1024 * 1024 - 1), "1024.00 MiB"); // Check rounding
    assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GiB");
    assert_eq!(
        format_bytes(1024 * 1024 * 1024 * 1536 / 1024),
        "1.50 GiB"
    );
}

// Note: test_calculate_audio_bitrate cannot be tested here as it's private.
// It should remain as a unit test within src/processing/audio.rs.