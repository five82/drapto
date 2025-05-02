// drapto-core/tests/process_videos_success_tests.rs

use drapto_core::config::CoreConfig;
use drapto_core::external::mocks::{MockFfmpegSpawner, MockFfprobeExecutor}; // Import MockFfprobeExecutor
use drapto_core::notifications::mocks::MockNotifier;
use drapto_core::processing::video::process_videos;
use ffmpeg_sidecar::event::{FfmpegEvent, FfmpegProgress};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

// Helper to create a dummy file with some content
fn create_dummy_file(dir: &Path, filename: &str) -> PathBuf {
    let file_path = dir.join(filename);
    let mut file = File::create(&file_path).expect("Failed to create dummy file");
    file.write_all(b"dummy content").expect("Failed to write dummy content");
    file_path
}

#[test]
fn test_process_videos_mock_ffmpeg_success() -> Result<(), Box<dyn std::error::Error>> {
    let input_dir = tempdir()?;
    let output_dir = tempdir()?;
    let log_dir = tempdir()?; // Separate log dir

    let dummy_video = create_dummy_file(input_dir.path(), "test_video.mkv");
    let files_to_process = vec![dummy_video.clone()]; // Clone dummy_video

    // Initialize CoreConfig correctly
    let config = CoreConfig {
         input_dir: input_dir.path().to_path_buf(),
         output_dir: output_dir.path().to_path_buf(),
         log_dir: log_dir.path().to_path_buf(),
         default_encoder_preset: None,
         preset: Some(6),
         quality_sd: Some(30),
         quality_hd: None,
         quality_uhd: None,
         default_crop_mode: None, // No crop override
         ntfy_topic: Some("http://localhost:1234/mock-topic".to_string()), // Add a mock topic
     };

    // --- Mock Spawner Setup ---
    let mock_spawner = MockFfmpegSpawner::new(); // Use new constructor
    let mock_events = vec![
        // Simulate some progress
        FfmpegEvent::Progress(FfmpegProgress { frame: 100, fps: 30.0, size_kb: 1024, time: "00:00:03.333".to_string(), bitrate_kbps: 2457.6, speed: 1.0, q: 0.0, raw_log_message: String::new() }),
        FfmpegEvent::Progress(FfmpegProgress { frame: 200, fps: 30.0, size_kb: 2048, time: "00:00:06.666".to_string(), bitrate_kbps: 2457.6, speed: 1.0, q: 0.0, raw_log_message: String::new() }),
    ];
    // Expect the cropdetect command first (non-HDR path, default threshold 16)
    mock_spawner.add_success_expectation("cropdetect=limit=16:round=2:reset=1", vec![], false); // Exact filter string
    // Expect the main encode command
    mock_spawner.add_success_expectation("libsvtav1", mock_events, true); // Match codec value

    // --- Mock Notifier ---
    let mock_notifier = MockNotifier::new();

    // --- Mock Ffprobe ---
    let mock_ffprobe = MockFfprobeExecutor::new();
    let default_props = drapto_core::processing::detection::VideoProperties { // Need to import or use full path
        width: 1920, height: 1080, duration: 120.0, ..Default::default()
    };
    mock_ffprobe.expect_audio_channels(&dummy_video, Ok(vec![2]));
    mock_ffprobe.expect_video_properties(&dummy_video, Ok(default_props)); // Add expectation for video props

    // --- Log Callback Setup ---
    let log_messages = Arc::new(Mutex::new(Vec::new()));
    let log_messages_clone = log_messages.clone();
    let log_callback = move |msg: &str| {
        log_messages_clone.lock().unwrap().push(msg.to_string());
        println!("LOG_CALLBACK: {}", msg); // Print for visibility during test run
    };

    // --- Execute ---
    let target_override: Option<PathBuf> = None; // Add missing argument
    let results = process_videos(&mock_spawner, &mock_ffprobe, &mock_notifier, &config, &files_to_process, target_override, log_callback); // Pass mock_ffprobe


    // --- Assertions ---
    // Expect Ok because the mock creates the output file, allowing get_file_size to succeed.
    assert!(results.is_ok(), "process_videos should succeed, but got Err: {:?}", results.err());
    let processed_files = results.unwrap();
    assert_eq!(processed_files.len(), 1, "Expected 1 file to be processed successfully");
    let result = &processed_files[0];
    assert_eq!(result.filename, "test_video.mkv");
    // Check for non-negative duration, as mock might be too fast for > 0
    assert!(result.duration.as_secs_f64() >= 0.0, "Expected non-negative processing duration");
    // Input size should be non-zero (dummy content written by helper)
    assert!(result.input_size > 0, "Expected non-zero input size");
    // Output size should be zero (mock spawner creates an empty file)
    assert_eq!(result.output_size, 0, "Expected zero output size for mock's empty dummy file");

    // Verify mock interactions: Check arguments passed to ffmpeg
    let calls = mock_spawner.get_received_calls();
    assert_eq!(calls.len(), 2, "Expected 2 ffmpeg calls (cropdetect, encode)");

    // Check cropdetect call args (example: check for input file)
    let cropdetect_args = &calls[0];
    assert!(cropdetect_args.iter().any(|a| a.contains("test_video.mkv")), "Cropdetect args should contain input file"); // Correct filename
    assert!(cropdetect_args.iter().any(|a| a == "cropdetect=limit=16:round=2:reset=1"), "Cropdetect args should contain correct filter");


    // Check main encode call args
    let encode_args = &calls[1];
    let expected_output_path = config.output_dir.join("test_video.mkv"); // Reconstruct expected output path
    assert!(encode_args.iter().any(|a| a == "-i"), "Encode args should contain -i");
    assert!(encode_args.iter().any(|a| a.contains("test_video.mkv")), "Encode args should contain input file");
    assert!(encode_args.iter().position(|a| a == "-preset").map_or(false, |i| encode_args.get(i+1) == Some(&"6".to_string())), "Encode args should contain -preset 6");
    assert!(encode_args.iter().position(|a| a == "-crf").map_or(false, |i| encode_args.get(i+1) == Some(&"27".to_string())), "Encode args should contain -crf 27 (HD default)"); // Correct expected CRF
    assert!(encode_args.iter().any(|a| a == expected_output_path.to_str().unwrap()), "Encode args should contain correct output path");
    assert!(encode_args.iter().any(|a| a == "libsvtav1"), "Encode args should contain codec");


    // Verify notifications
    let sent_notifications = mock_notifier.get_sent_notifications();
    assert_eq!(sent_notifications.len(), 2, "Expected 2 notifications (start, success)");
    // Start Notification
    let start_notification = &sent_notifications[0];
    assert_eq!(start_notification.topic_url, "http://localhost:1234/mock-topic");
    assert!(start_notification.message.contains("Starting encode for: test_video.mkv"));
    assert_eq!(start_notification.title.as_deref(), Some("Drapto Encode Start"));
    // Success Notification
    let success_notification = &sent_notifications[1];
    assert_eq!(success_notification.topic_url, "http://localhost:1234/mock-topic");
    assert!(success_notification.message.contains("Successfully encoded test_video.mkv"));
    // Since output size is 0, reduction should be 100%
    assert!(success_notification.message.contains("(Reduced by 100%)"), "Expected 100% reduction message");
    assert_eq!(success_notification.title.as_deref(), Some("Drapto Encode Success"));
    assert_eq!(success_notification.priority, Some(4));

    // Verify log messages
    let logs = log_messages.lock().unwrap();
    assert!(logs.iter().any(|m| m.contains("Processing:")), "Should log start message"); // Match actual log message
    assert!(logs.iter().any(|m| m.contains("Encoding progress")), "Should log progress");
    assert!(logs.iter().any(|m| m.contains("✅ Encode finished successfully")), "Should log success message");
    // Add more log assertions as needed

    Ok(())
}