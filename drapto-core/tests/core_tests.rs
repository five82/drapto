use drapto_core::*; // Import items from the drapto_core crate
use std::fs::{self, File};
use std::path::PathBuf;
use std::time::Duration;
use tempfile::tempdir; // Use tempfile for creating temporary directories/files

// --- Test Helper Functions ---

// Note: We can't directly test the private `get_file_size` function here
// as integration tests only access the public API.
// If testing private functions is crucial, they might need to be
// refactored or tested within the `lib.rs` file's `mod tests`.
// However, `get_file_size` is indirectly tested via `process_videos`
// if we were to write integration tests for that (which is complex due to dependencies).
// For now, we'll keep the public API tests.

#[test]
fn test_calculate_audio_bitrate() {
    // This function is private, so we can't call it directly from an integration test.
    // To test this, it would either need to be made public (perhaps pub(crate))
    // or tested within the lib.rs `mod tests`. Let's assume for now it remains private
    // and we focus on testing the public API.
    // If you want this tested, we can move its test back to lib.rs or make the function pub(crate).
    // assert_eq!(calculate_audio_bitrate(1), 64); // Cannot call private function
}

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

// We cannot test the private `get_file_size` directly here.
// #[test]
// fn test_get_file_size() -> Result<(), Box<dyn std::error::Error>> { ... }


#[test]
fn test_find_processable_files() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let input_dir = dir.path();

    // Create some files
    File::create(input_dir.join("video1.mkv"))?;
    File::create(input_dir.join("video2.MKV"))?; // Test case insensitivity
    File::create(input_dir.join("document.txt"))?;
    File::create(input_dir.join("image.jpg"))?;
    fs::create_dir(input_dir.join("subdir"))?;
    File::create(input_dir.join("subdir").join("nested_video.mkv"))?; // Should not be found (max_depth 1)

    let result = find_processable_files(input_dir);
    assert!(result.is_ok());
    let mut files = result.unwrap();

    // Sort for consistent comparison
    files.sort();

    assert_eq!(files.len(), 2);
    assert_eq!(files[0].file_name().unwrap(), "video1.mkv");
    assert_eq!(files[1].file_name().unwrap(), "video2.MKV"); // Original case preserved

    dir.close()?;
    Ok(())
}

#[test]
fn test_find_processable_files_empty() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let input_dir = dir.path();

    File::create(input_dir.join("document.txt"))?;
    fs::create_dir(input_dir.join("subdir"))?;

    let result = find_processable_files(input_dir);
    assert!(result.is_err());
    match result.err().unwrap() {
        CoreError::NoFilesFound => {} // Expected error
        e => panic!("Unexpected error type: {:?}", e),
    }

    dir.close()?;
    Ok(())
}

#[test]
fn test_find_processable_files_nonexistent_dir() {
    let non_existent_path = PathBuf::from("surely_this_does_not_exist_42_integration");
    let result = find_processable_files(&non_existent_path);
    // walkdir::Error should be wrapped in CoreError::Walkdir
    assert!(result.is_err());
    match result.err().unwrap() {
        CoreError::Walkdir(_) => {} // Expected error type
        e => panic!("Unexpected error type: {:?}", e),
    }
}