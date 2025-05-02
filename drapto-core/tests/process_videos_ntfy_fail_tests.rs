// drapto-core/tests/process_videos_ntfy_fail_tests.rs

use drapto_core::config::CoreConfig;
use drapto_core::error::CoreError;
use drapto_core::external::mocks::{MockFfmpegSpawner, MockFfprobeExecutor};
use drapto_core::notifications::mocks::MockNotifier;
use drapto_core::processing::video::process_videos;
// Removed unused VideoProperties import (using full path below)
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

// Helper to create a dummy file with some content
// (Copied - consider moving to a shared test utils module later)
fn create_dummy_file(dir: &Path, filename: &str) -> PathBuf {
    let file_path = dir.join(filename);
    let mut file = File::create(&file_path).expect("Failed to create dummy file");
    file.write_all(b"dummy content").expect("Failed to write dummy content");
    file_path
}

#[test]
fn test_process_videos_mock_ntfy_fail() -> Result<(), Box<dyn std::error::Error>> {
    let input_dir = tempdir()?;
    let output_dir = tempdir()?;
    let log_dir = tempdir()?;

    let dummy_video = create_dummy_file(input_dir.path(), "ntfy_fail.mkv");
    let files_to_process = vec![dummy_video.clone()]; // Clone dummy_video

    let config = CoreConfig {
         input_dir: input_dir.path().to_path_buf(),
         output_dir: output_dir.path().to_path_buf(),
         log_dir: log_dir.path().to_path_buf(),
         default_encoder_preset: None,
         preset: Some(6),
         quality_sd: Some(30),
         quality_hd: None,
         quality_uhd: None,
         default_crop_mode: None,
         ntfy_topic: Some("http://localhost:1234/mock-topic-ntfy-fail".to_string()), // Need topic for ntfy attempt
     };

    // --- Mock Spawner Setup (Success) ---
    let mock_spawner = MockFfmpegSpawner::new(); // Use new constructor
    // Expect the cropdetect command first
    mock_spawner.add_success_expectation("cropdetect=limit=16:round=2:reset=1", vec![], false); // Exact filter string
    // Expect the main encode command, return success
    mock_spawner.add_success_expectation("libsvtav1", vec![], true); // Match codec value

    // --- Mock Notifier Setup (Failure) ---
    let mock_notifier = MockNotifier::new();
    let ntfy_error = CoreError::NotificationError("Simulated ntfy send failure".to_string());
    // Configure mock to fail on the *next* send call (which will be the success notification)
    mock_notifier.set_error_on_next_send(ntfy_error);

    // --- Mock Ffprobe ---
    let mock_ffprobe = MockFfprobeExecutor::new();
    let default_props = drapto_core::processing::detection::VideoProperties {
        width: 1920, height: 1080, duration_secs: 120.0, ..Default::default() // Use renamed field
    };
    mock_ffprobe.expect_audio_channels(&dummy_video, Ok(vec![2]));
    mock_ffprobe.expect_video_properties(&dummy_video, Ok(default_props)); // Add expectation

    // --- Log Callback Setup ---
    let log_messages = Arc::new(Mutex::new(Vec::new()));
    let log_messages_clone = log_messages.clone();
    let log_callback = move |msg: &str| {
        log_messages_clone.lock().unwrap().push(msg.to_string());
        println!("LOG_CALLBACK: {}", msg);
    };

    // --- Execute ---
    let target_override: Option<PathBuf> = None;
    let results = process_videos(&mock_spawner, &mock_ffprobe, &mock_notifier, &config, &files_to_process, target_override, log_callback); // Pass mock_ffprobe

    // --- Assertions ---
    // Expect Ok result, as ntfy failure only logs a warning, doesn't fail the whole process.
    assert!(results.is_ok(), "process_videos should return Ok even if ntfy fails, but got Err: {:?}", results.err());
    let processed_files = results.unwrap();
    assert_eq!(processed_files.len(), 1, "Expected 1 file to be processed despite ntfy failure");

    // Verify notifications (mock was set to fail on first send (start), so only success should be captured)
    let sent_notifications = mock_notifier.get_sent_notifications();
    println!("Captured notifications: {:?}", sent_notifications); // Keep debug print for now
    assert_eq!(sent_notifications.len(), 1, "Expected only success notification to be captured");
    let success_notification = &sent_notifications[0];
    assert_eq!(success_notification.topic_url, "http://localhost:1234/mock-topic-ntfy-fail");
    assert!(success_notification.message.contains("Successfully encoded ntfy_fail.mkv"));
    assert_eq!(success_notification.title.as_deref(), Some("Drapto Encode Success"));

    // Verify log messages
    let logs = log_messages.lock().unwrap();
    assert!(logs.iter().any(|m| m.contains("Processing: ntfy_fail.mkv")), "Should log start message");
    assert!(logs.iter().any(|m| m.contains("Completed: ntfy_fail.mkv")), "Should log completion message");
    // Check for the START notification failure warning
    assert!(logs.iter().any(|m| m.contains("Warning: Failed to send ntfy start notification")), "Should log ntfy START failure warning");

    Ok(())
}