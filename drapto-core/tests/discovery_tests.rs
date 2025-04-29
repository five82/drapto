// drapto-core/tests/discovery_tests.rs

use drapto_core::discovery::find_processable_files; // Import necessary function
use drapto_core::error::CoreError; // Import error type
use std::fs::{self, File};
use std::path::PathBuf;
use tempfile::tempdir;

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