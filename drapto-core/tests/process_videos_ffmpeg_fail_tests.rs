// drapto-core/tests/process_videos_ffmpeg_fail_tests.rs

use drapto_core::config::CoreConfig;
use drapto_core::error::CoreError;
use drapto_core::external::mocks::{MockFfmpegSpawner, MockFfprobeExecutor};
use drapto_core::notifications::mocks::MockNotifier;
use drapto_core::processing::video::process_videos;
// Removed unused VideoProperties import (using full path below)
use ffmpeg_sidecar::event::{FfmpegEvent, FfmpegProgress};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
// Removed unused ExitStatus import
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
fn test_process_videos_mock_ffmpeg_fail_exit_code() -> Result<(), Box<dyn std::error::Error>> {
    let input_dir = tempdir()?;
    let output_dir = tempdir()?;
    let log_dir = tempdir()?;

    let dummy_video = create_dummy_file(input_dir.path(), "fail_video.mkv");
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
         ntfy_topic: Some("http://localhost:1234/mock-topic-fail".to_string()), // Add topic
         enable_denoise: true, // Added field
     };

    // --- Mock Spawner Setup ---
    let mock_spawner = MockFfmpegSpawner::new(); // Use new constructor
    let mock_events = vec![
         FfmpegEvent::Progress(FfmpegProgress { frame: 50, fps: 30.0, size_kb: 512, time: "00:00:01.666".to_string(), bitrate_kbps: 2457.6, speed: 1.0, q: 0.0, raw_log_message: String::new() }),
         FfmpegEvent::Error("Simulated ffmpeg error line".to_string()),
    ];
    // Expect the cropdetect command first
    mock_spawner.add_success_expectation("cropdetect=limit=16:round=2:reset=1", vec![], false); // Exact filter string
    // Expect the main encode command, return exit error
    mock_spawner.add_exit_error_expectation("libsvtav1", mock_events, 1); // Match codec value

    // --- Mock Notifier ---
    let mock_notifier = MockNotifier::new();

    // --- Mock Ffprobe ---
    let mock_ffprobe = MockFfprobeExecutor::new();
    let default_props = drapto_core::processing::detection::VideoProperties {
        width: 1920, height: 1080, duration_secs: 120.0, ..Default::default() // Use renamed field
    };
    mock_ffprobe.expect_audio_channels(&dummy_video, Ok(vec![2]));
    mock_ffprobe.expect_video_properties(&dummy_video, Ok(default_props.clone())); // Add expectation

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
    // Expect Ok result, but the results vector should be empty because the encode failed for the only file.
    assert!(results.is_ok(), "process_videos should return Ok even if individual encodes fail, but got Err: {:?}", results.err());
    let processed_files = results.unwrap();
    assert!(processed_files.is_empty(), "Expected results vector to be empty after encode failure");

    // Verify notifications
    let sent_notifications = mock_notifier.get_sent_notifications();
    assert_eq!(sent_notifications.len(), 2, "Expected 2 notifications (start, error)");
    // Start Notification
    assert_eq!(sent_notifications[0].topic_url, "http://localhost:1234/mock-topic-fail");
    assert!(sent_notifications[0].message.contains("Starting encode for: fail_video.mkv"));
    assert_eq!(sent_notifications[0].title.as_deref(), Some("Drapto Encode Start"));
    // Error Notification
    assert_eq!(sent_notifications[1].topic_url, "http://localhost:1234/mock-topic-fail");
    assert!(sent_notifications[1].message.contains("Error encoding fail_video.mkv: ffmpeg failed."));
    assert_eq!(sent_notifications[1].title.as_deref(), Some("Drapto Encode Error"));
    assert_eq!(sent_notifications[1].priority, Some(5));
    assert!(sent_notifications[1].tags.as_deref().unwrap_or("").contains("x"));

    // Verify log messages
    let logs = log_messages.lock().unwrap();
    assert!(logs.iter().any(|m| m.contains("Processing: fail_video.mkv")), "Should log start message");
    assert!(logs.iter().any(|m| m.contains("ERROR: ffmpeg encode failed for fail_video.mkv")), "Should log high-level error message"); // Match exact error log

    Ok(())
}

#[test]
fn test_process_videos_mock_ffmpeg_spawn_error() -> Result<(), Box<dyn std::error::Error>> {
    let input_dir = tempdir()?;
    let output_dir = tempdir()?;
    let log_dir = tempdir()?;

    let dummy_video = create_dummy_file(input_dir.path(), "spawn_fail.mkv");
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
         default_crop_mode: Some("off".to_string()), // Disable crop detection for this test
         ntfy_topic: Some("http://localhost:1234/mock-topic-spawn-fail".to_string()), // Add topic
         enable_denoise: true, // Added field
     };

     // --- Mock Spawner Setup ---
    let mock_spawner = MockFfmpegSpawner::new(); // Use new constructor
    let spawn_error = CoreError::CommandStart("ffmpeg (mock)".to_string(), std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Mock spawn permission denied"));
    // Expect the main encode command, return spawn error (crop detection is disabled in config for this test)
    mock_spawner.add_spawn_error_expectation("libsvtav1", spawn_error); // Match codec value

    // --- Mock Notifier ---
    let mock_notifier = MockNotifier::new();

    // --- Mock Ffprobe ---
    let mock_ffprobe = MockFfprobeExecutor::new();
    // Use the same default_props as above or redefine if needed
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
    // Expect Ok result, but the results vector should be empty because the encode failed to spawn.
    assert!(results.is_ok(), "process_videos should return Ok even if spawn fails, but got Err: {:?}", results.err());
    let processed_files = results.unwrap();
    assert!(processed_files.is_empty(), "Expected results vector to be empty after spawn failure");

    // Verify notifications
    let sent_notifications = mock_notifier.get_sent_notifications();
    assert_eq!(sent_notifications.len(), 2, "Expected 2 notifications (start, error)");
    // Start Notification
    assert_eq!(sent_notifications[0].topic_url, "http://localhost:1234/mock-topic-spawn-fail");
    assert!(sent_notifications[0].message.contains("Starting encode for: spawn_fail.mkv"));
    assert_eq!(sent_notifications[0].title.as_deref(), Some("Drapto Encode Start"));
    // Error Notification
    assert_eq!(sent_notifications[1].topic_url, "http://localhost:1234/mock-topic-spawn-fail");
    assert!(sent_notifications[1].message.contains("Error encoding spawn_fail.mkv: ffmpeg failed.")); // Check generic failure message
    assert_eq!(sent_notifications[1].title.as_deref(), Some("Drapto Encode Error"));
    assert_eq!(sent_notifications[1].priority, Some(5));
    assert!(sent_notifications[1].tags.as_deref().unwrap_or("").contains("x"));

    // Verify log messages
    let logs = log_messages.lock().unwrap();
    assert!(logs.iter().any(|m| m.contains("Processing: spawn_fail.mkv")), "Should log start message");
    assert!(logs.iter().any(|m| m.contains("ERROR: ffmpeg encode failed for spawn_fail.mkv")), "Should log high-level error message"); // Match exact error log

    Ok(())
}