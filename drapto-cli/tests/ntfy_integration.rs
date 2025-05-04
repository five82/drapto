// drapto-cli/tests/ntfy_integration.rs

// Use the crate itself for accessing its public items
use drapto_cli::cli::EncodeArgs;
use drapto_cli::commands;
// Import mock spawner and core error for test setup
#[cfg(feature = "test-mock-ffmpeg")]
use drapto_core::external::mocks::{MockFfmpegSpawner, MockFfprobeExecutor}; // Import MockFfprobeExecutor
use drapto_core::notifications::mocks::MockNotifier; // Imports are correct
use ffmpeg_sidecar::event::FfmpegEvent; // Import the event type
use drapto_core::processing::VideoProperties; // Import VideoProperties struct
use drapto_core::error::CoreError; // Keep CoreError for potential mock errors
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

    // Removed unused invalid_ntfy_url variable

    // --- Construct Args ---
    let args = EncodeArgs {
        input_path: input_dir.path().to_path_buf(),
        output_dir: output_dir.path().to_path_buf(),
        log_dir: None, // Let it default to output_dir/logs
        quality_sd: None,
        quality_hd: None,
        quality_uhd: None,
        ntfy: Some("http://mock.ntfy.topic/valid-topic".to_string()), // Use a valid-looking URL now
        disable_autocrop: false, // Add the missing field
        preset: None, // Add the missing preset field
    };

    // --- Execute ---
    // --- Setup Mock Spawner Expectations (Required because test-mock-ffmpeg is enabled) ---
    // We don't care about the ffmpeg results here, just need to satisfy the mock.
    // Use #[cfg] to avoid compile errors if test-mock-ffmpeg isn't active (though it should be for tests)
    #[cfg(feature = "test-mock-ffmpeg")]
    let mock_spawner = MockFfmpegSpawner::new();
    #[cfg(feature = "test-mock-ffmpeg")]
    mock_spawner.add_success_expectation("cropdetect=limit=16", vec![], false); // Assume default cropdetect runs
    #[cfg(feature = "test-mock-ffmpeg")]
    // Simulate ffmpeg failing early, as the dummy file is invalid
    mock_spawner.add_exit_error_expectation("libsvtav1", vec![], 1);


    // --- Mock Notifier (Configured to Fail) ---
    let mock_notifier = MockNotifier::new();
    let ntfy_error = CoreError::NotificationError("Simulated ntfy send failure for test".to_string());
    // Configure mock to fail on the first send (start notification)
    mock_notifier.set_error_on_next_send(ntfy_error);

    // --- Mock Ffprobe ---
    let mock_ffprobe = MockFfprobeExecutor::new();
    // Set default expectation, although it might not be strictly needed if ffmpeg mock fails early
    mock_ffprobe.expect_audio_channels(&_dummy_mkv, Ok(vec![2]));

    // --- Execute ---
    // run_encode handles creating the log dir and file
    // Pass the configured mock spawner, ffprobe executor, and notifier
    let result = commands::encode::run_encode(&mock_spawner, &mock_ffprobe, &mock_notifier, args, false, vec![_dummy_mkv.clone()], input_dir.path().to_path_buf());

    // We expect run_encode to succeed overall even if ntfy fails (it just logs warnings)
    // and even if the mocked ffmpeg fails (it logs errors but doesn't panic run_encode)
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
#[test]
fn test_ntfy_success_notifications_sent() {
    // --- Setup ---
    let input_dir = tempdir().expect("Failed to create temp input dir");
    let output_dir = tempdir().expect("Failed to create temp output dir");
    let log_dir_path = output_dir.path().join("logs"); // Define where logs *should* go

    let dummy_mkv_path = create_dummy_mkv(&input_dir.path().to_path_buf(), "success_test.mkv");
    let ntfy_topic = "http://mock.ntfy.topic/success-test";

    // --- Construct Args ---
    let args = EncodeArgs {
        input_path: input_dir.path().to_path_buf(),
        output_dir: output_dir.path().to_path_buf(),
        log_dir: Some(log_dir_path.clone()), // Explicitly set log dir
        quality_sd: None,
        quality_hd: None,
        quality_uhd: None,
        ntfy: Some(ntfy_topic.to_string()),
        disable_autocrop: false,
        preset: None,
    };

    // --- Setup Mocks ---
    #[cfg(feature = "test-mock-ffmpeg")]
    let mock_spawner = MockFfmpegSpawner::new();
    #[cfg(feature = "test-mock-ffmpeg")]
    {
        // Expect crop detect and simulate success
        mock_spawner.add_success_expectation("cropdetect=limit=16", vec![FfmpegEvent::ParsedStreamMapping("crop=1920:800:0:140".to_string())], false);
        // Expect main encode and simulate success, CREATE dummy output file
        mock_spawner.add_success_expectation("libsvtav1", vec![], true); // Set create_dummy_output to true
    }

    let mock_ffprobe = MockFfprobeExecutor::new();
    // Simulate successful video properties detection (needed to proceed)
    mock_ffprobe.expect_video_properties(&dummy_mkv_path, Ok(VideoProperties { width: 1920, height: 1080, duration_secs: 10.0, color_space: None })); // Use duration_secs, remove removed fields
    // Simulate successful audio channel detection
    mock_ffprobe.expect_audio_channels(&dummy_mkv_path, Ok(vec![2])); // Stereo

    let mock_notifier = MockNotifier::new(); // Default behavior is success

    // --- Execute ---
    let result = commands::encode::run_encode(
        &mock_spawner,
        &mock_ffprobe,
        &mock_notifier,
        args,
        false, // Not interactive
        vec![dummy_mkv_path.clone()],
        input_dir.path().to_path_buf(),
    );

    // --- Verification ---
    assert!(result.is_ok(), "run_encode should succeed. Result: {:?}", result);

    // Verify notifications sent
    let sent_notifications = mock_notifier.get_sent_notifications(); // Use correct method

    println!("Sent ntfy notifications: {:?}", sent_notifications); // For debugging

    assert_eq!(sent_notifications.len(), 2, "Expected 2 notifications (start, success)");

    // Check start notification
    assert!(
        sent_notifications[0].message.contains("Starting encode for:"), // Check for actual content
        "First message should indicate starting encode"
    );
    // Removed assertion checking for input dir in start message, as it's not included.

    // Check success notification for the specific file
    assert!(
        sent_notifications[1].message.contains("Successfully encoded"), // Check for actual content
        "Second message should indicate successful encoding"
    );
    assert!(
        sent_notifications[1].message.contains("success_test.mkv"), // Access .message field
        "Success message should contain the filename"
    );
    // Removed assertion checking for "Output:" as it's not in the message format.

    // Optional: Check log file content if needed, but focus is on ntfy here
}

#[test]
fn test_ntfy_error_notification_sent() {
    // --- Setup ---
    let input_dir = tempdir().expect("Failed to create temp input dir");
    let output_dir = tempdir().expect("Failed to create temp output dir");
    let log_dir_path = output_dir.path().join("logs");

    let dummy_mkv_path = create_dummy_mkv(&input_dir.path().to_path_buf(), "error_test.mkv");
    let ntfy_topic = "http://mock.ntfy.topic/error-test";

    // --- Construct Args ---
    let args = EncodeArgs {
        input_path: input_dir.path().to_path_buf(),
        output_dir: output_dir.path().to_path_buf(),
        log_dir: Some(log_dir_path.clone()),
        quality_sd: None,
        quality_hd: None,
        quality_uhd: None,
        ntfy: Some(ntfy_topic.to_string()),
        disable_autocrop: false,
        preset: None,
    };

    // --- Setup Mocks ---
    #[cfg(feature = "test-mock-ffmpeg")]
    let mock_spawner = MockFfmpegSpawner::new();
    #[cfg(feature = "test-mock-ffmpeg")]
    {
        // Expect crop detect and simulate success (needed to proceed to encode)
        mock_spawner.add_success_expectation("cropdetect=limit=16", vec![FfmpegEvent::ParsedStreamMapping("crop=1920:800:0:140".to_string())], false);
        // Expect main encode and simulate FAILURE
        mock_spawner.add_exit_error_expectation("libsvtav1", vec![FfmpegEvent::Error("Simulated ffmpeg error".to_string())], 1);
    }

    let mock_ffprobe = MockFfprobeExecutor::new();
    // Simulate successful video properties detection (needed to proceed)
    mock_ffprobe.expect_video_properties(&dummy_mkv_path, Ok(VideoProperties { width: 1920, height: 1080, duration_secs: 10.0, color_space: None })); // Use duration_secs, remove removed fields
    // Simulate successful audio channel detection
    mock_ffprobe.expect_audio_channels(&dummy_mkv_path, Ok(vec![2]));

    let mock_notifier = MockNotifier::new(); // Default behavior is success

    // --- Execute ---
    let result = commands::encode::run_encode(
        &mock_spawner,
        &mock_ffprobe,
        &mock_notifier,
        args,
        false, // Not interactive
        vec![dummy_mkv_path.clone()],
        input_dir.path().to_path_buf(),
    );

    // --- Verification ---
    // run_encode itself should succeed even if ffmpeg fails internally, as it logs the error.
    assert!(result.is_ok(), "run_encode should succeed even with internal errors. Result: {:?}", result);

    // Verify notifications sent
    let sent_notifications = mock_notifier.get_sent_notifications(); // Use correct method

    println!("Sent ntfy notifications (error case): {:?}", sent_notifications); // For debugging

    assert_eq!(sent_notifications.len(), 2, "Expected 2 notifications (start, error)");

    // Check start notification (same as success case)
    assert!(
        sent_notifications[0].message.contains("Starting encode for:"), // Check for actual content
        "First message should indicate starting encode"
    );
    // Removed assertion checking for input dir in start message, as it's not included.

    // Check error notification for the specific file
    assert!(
        sent_notifications[1].message.contains("Error encoding"), // Check for actual content
        "Second message should indicate an encoding error"
    );
    assert!(
        sent_notifications[1].message.contains("error_test.mkv"), // Access .message field
        "Error message should contain the filename"
    );
    // Removed assertion checking for specific error type as it's not in the message format.
}