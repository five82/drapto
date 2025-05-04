// drapto-core/tests/detect_crop_tests.rs

use drapto_core::external::mocks::MockFfmpegSpawner;
use drapto_core::processing::detection; // Remove direct VideoProperties import
use ffmpeg_sidecar::event::{FfmpegEvent, LogLevel};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

// Helper to create a dummy file with some content
// (Copied from process_videos_tests.rs - consider moving to a shared test utils module later)
fn create_dummy_file(dir: &Path, filename: &str) -> PathBuf {
    let file_path = dir.join(filename);
    let mut file = File::create(&file_path).expect("Failed to create dummy file");
    file.write_all(b"dummy content").expect("Failed to write dummy content");
    file_path
}

#[test]
fn test_detect_crop_mocked() -> Result<(), Box<dyn std::error::Error>> {
    let input_dir = tempdir()?;
    let dummy_video_path = create_dummy_file(input_dir.path(), "crop_test.mkv");

    // Simulate non-HDR video properties
    let video_props = detection::VideoProperties {
        width: 1920,
        height: 1080,
        duration_secs: 600.0, // 10 minutes - Use renamed field
        color_space: Some("bt709".to_string()),
        // color_transfer and color_primaries removed
    };

    // --- Mock Spawner Setup ---
    let mock_spawner = MockFfmpegSpawner::new(); // Use new constructor
    let mock_events = vec![
        // Simulate ffmpeg outputting crop detection lines
        FfmpegEvent::Log(LogLevel::Info, "[Parsed_cropdetect_0 @ 0x...] crop=1920:800:0:140".to_string()),
        FfmpegEvent::Log(LogLevel::Info, "[Parsed_cropdetect_0 @ 0x...] crop=1920:800:0:140".to_string()),
        FfmpegEvent::Log(LogLevel::Info, "[Parsed_cropdetect_0 @ 0x...] crop=1920:800:0:140".to_string()),
        FfmpegEvent::Log(LogLevel::Info, "[Parsed_cropdetect_0 @ 0x...] crop=1920:1080:0:0".to_string()),
    ];
    // Expect the cropdetect command (identified by the exact filter string)
    mock_spawner.add_success_expectation("cropdetect=limit=16:round=2:reset=1", mock_events, false); // Exact filter string

    // --- Execute detect_crop ---
    let result = detection::detect_crop(&mock_spawner, &dummy_video_path, &video_props, false); // disable_crop = false

    // --- Assertions ---
    assert!(result.is_ok(), "detect_crop should succeed");
    let (crop_filter_opt, is_hdr) = result.unwrap();

    assert!(!is_hdr, "Expected is_hdr to be false for non-HDR properties");
    assert!(crop_filter_opt.is_some(), "Expected a crop filter string");
    assert_eq!(crop_filter_opt.unwrap(), "crop=1920:800:0:140", "Expected the most frequent crop value");

    // Verify mock spawner was called (optional, more useful for arg verification)
    // let calls = mock_spawner.get_received_calls();
    // assert_eq!(calls.len(), 1);

    Ok(())
}

#[test]
fn test_detect_crop_mocked_hdr() -> Result<(), Box<dyn std::error::Error>> {
    let input_dir = tempdir()?;
    let dummy_video_path = create_dummy_file(input_dir.path(), "crop_hdr_test.mkv");

    // Simulate HDR video properties (e.g., using bt2020)
    let video_props = detection::VideoProperties {
        width: 3840,
        height: 2160,
        duration_secs: 600.0, // 10 minutes - Use renamed field
        color_space: Some("bt2020nc".to_string()), // HDR indicator
        // color_transfer and color_primaries removed
    };

    // --- Mock Spawner Setup ---
    let mock_spawner = MockFfmpegSpawner::new();

    // 1. Expectation for run_hdr_blackdetect
    let blackdetect_events = vec![
        // Simulate ffmpeg outputting black level lines
        FfmpegEvent::Log(LogLevel::Info, "[blackdetect @ 0x...] black_level: 64".to_string()),
        FfmpegEvent::Log(LogLevel::Info, "[blackdetect @ 0x...] black_level: 70".to_string()), // Example different value
    ];
    // Use a pattern unique to the blackdetect command
    mock_spawner.add_success_expectation("blackdetect=d=0", blackdetect_events, false);

    // 2. Expectation for run_cropdetect (using refined threshold)
    // Calculation based on mock blackdetect output: avg = (64+70)/2 = 67. Refined = round(67*1.5) = 101. Clamped = 101.
    let expected_cropdetect_threshold = 101;
    let cropdetect_pattern = format!("cropdetect=limit={}:round=2:reset=1", expected_cropdetect_threshold);
    let cropdetect_events = vec![
        // Simulate cropdetect output after blackdetect refinement
        FfmpegEvent::Log(LogLevel::Info, format!("[Parsed_cropdetect_0 @ 0x...] crop=3840:1600:0:280 t:...", )), // Example crop
        FfmpegEvent::Log(LogLevel::Info, format!("[Parsed_cropdetect_0 @ 0x...] crop=3840:1600:0:280 t:...", )),
    ];
    mock_spawner.add_success_expectation(&cropdetect_pattern, cropdetect_events, false);


    // --- Execute detect_crop ---
    let result = detection::detect_crop(&mock_spawner, &dummy_video_path, &video_props, false); // disable_crop = false

    // --- Assertions ---
    assert!(result.is_ok(), "detect_crop should succeed for HDR path");
    let (crop_filter_opt, is_hdr) = result.unwrap();

    assert!(is_hdr, "Expected is_hdr to be true for HDR properties");
    assert!(crop_filter_opt.is_some(), "Expected a crop filter string for HDR");
    assert_eq!(crop_filter_opt.unwrap(), "crop=3840:1600:0:280", "Expected the crop value from mock");

    // Verify mock spawner calls (optional)
    let calls = mock_spawner.get_received_calls();
    assert_eq!(calls.len(), 2, "Expected two ffmpeg calls (blackdetect, cropdetect)");

    Ok(())
}

// TODO: Add test case for ffprobe failure (if mocking ffprobe separately)