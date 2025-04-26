// drapto-cli/tests/ntfy_integration.rs

// Use the crate itself for accessing its public items
use drapto_cli::cli::EncodeArgs;
use drapto_cli::commands;
use std::fs::{self, File};
use std::io::Read;
use std::path::PathBuf; // Keep only one import
use tempfile::tempdir; // Now available via dev-dependencies

// Helper to create a dummy mkv file
fn create_dummy_mkv(dir: &PathBuf, filename: &str) -> PathBuf {
    let file_path = dir.join(filename);
    File::create(&file_path).expect("Failed to create dummy file");
    file_path
}

#[test]
fn test_ntfy_warnings_logged_on_failure() {
    // --- Setup ---
    let input_dir = tempdir().expect("Failed to create temp input dir");
    let output_dir = tempdir().expect("Failed to create temp output dir");
    let log_dir_path = output_dir.path().join("logs"); // Define where logs *should* go

    let _dummy_mkv = create_dummy_mkv(&input_dir.path().to_path_buf(), "test_video.mkv");

    let invalid_ntfy_url = "http://localhost:1"; // Guaranteed to fail connection

    // --- Construct Args ---
    let args = EncodeArgs {
        input_path: input_dir.path().to_path_buf(),
        output_dir: output_dir.path().to_path_buf(),
        log_dir: None, // Let it default to output_dir/logs
        quality_sd: None,
        quality_hd: None,
        quality_uhd: None,
        ntfy: Some(invalid_ntfy_url.to_string()), // Provide the invalid URL
        disable_autocrop: false, // Add the missing field
        preset: None, // Add the missing preset field
    };

    // --- Execute ---
    // run_encode handles creating the log dir and file
    let result = commands::encode::run_encode(args, false, vec![_dummy_mkv.clone()], input_dir.path().to_path_buf()); // Pass dummy file list and input dir

    // We expect run_encode to succeed even if ntfy fails (it just logs warnings)
    // However, the underlying Handbrake call might fail if HandbrakeCLI isn't installed
    // or if the dummy file isn't processable. We primarily care about the log content here.
    // Let's proceed assuming run_encode itself doesn't panic.
    println!("run_encode result: {:?}", result); // Log result for debugging

    // --- Verification ---
    // Find the log file (name includes timestamp, so find the most recent .log)
    let log_files = fs::read_dir(&log_dir_path)
        .expect("Failed to read log directory")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "log"))
        .collect::<Vec<_>>();

    assert_eq!(log_files.len(), 1, "Expected exactly one log file");
    let log_file_path = log_files[0].path();

    let mut log_content = String::new();
    File::open(&log_file_path)
        .expect("Failed to open log file")
        .read_to_string(&mut log_content)
        .expect("Failed to read log content");

    println!("--- Log Content ---");
    println!("{}", log_content);
    println!("-------------------");

    // Check for the specific warning messages indicating ntfy calls were attempted and failed
    // Check that the start notification failure was logged
    assert!(
        log_content.contains("Warning: Failed to send ntfy start notification"),
        "Log should contain ntfy start failure warning"
    );

    // NOTE: In this specific test setup, the dummy MKV is invalid, causing ffprobe
    // to fail and the processing loop to 'continue' before Handbrake (and thus
    // the success/error notifications) are attempted. Therefore, we *only* expect
    // the 'start' notification warning here. A more complex test with a valid
    // dummy file or mocking would be needed to test the success/error paths.
    assert!(
        !log_content.contains("Warning: Failed to send ntfy success notification"),
        "Log should NOT contain ntfy success warning in this specific failure case"
    );
     assert!(
        !log_content.contains("Warning: Failed to send ntfy error notification"),
        "Log should NOT contain ntfy error warning in this specific failure case"
    );

    // Optional: Check that the main processing didn't completely fail *because* of ntfy
    // (This depends on HandbrakeCLI's behavior with the dummy file)
    // assert!(result.is_ok(), "run_encode should ideally finish, even with ntfy errors");
}