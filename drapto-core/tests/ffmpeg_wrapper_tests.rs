// drapto-core/tests/ffmpeg_wrapper_tests.rs

use drapto_core::external::ffmpeg::{run_ffmpeg_encode, EncodeParams};
use drapto_core::external::mocks::MockFfmpegSpawner;
// Removed unused std::path::PathBuf import
use tempfile::tempdir;

#[test]
fn test_run_ffmpeg_encode_args_basic() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempdir()?;
    let input_path = tmp.path().join("input.mkv");
    let output_path = tmp.path().join("output.mkv");

    let params = EncodeParams {
        input_path: input_path.clone(),
        output_path: output_path.clone(),
        quality: 25,
        preset: 7,
        crop_filter: None,
        audio_channels: vec![2], // Stereo
        duration: 60.0,
        enable_denoise: true, // Added field
    };

    let mock_spawner = MockFfmpegSpawner::new();
    // Expect the encode call, return simple success
    mock_spawner.add_success_expectation("libsvtav1", vec![], false);

    let log_callback = |_msg: &str| {}; // No-op log callback

    // Execute the function under test
    let result = run_ffmpeg_encode(&mock_spawner, &params, log_callback);
    assert!(result.is_ok());

    // Verify arguments passed to the mock spawner
    let calls = mock_spawner.get_received_calls();
    assert_eq!(calls.len(), 1, "Expected one call to ffmpeg spawner");
    let args = &calls[0];

    // Basic checks - more detailed checks can be added
    assert!(args.iter().any(|a| a == "-i"));
    assert!(args.iter().any(|a| a == input_path.to_str().unwrap()));
    assert!(args.iter().any(|a| a == output_path.to_str().unwrap()));
    assert!(args.iter().position(|a| a == "-crf").map_or(false, |i| args.get(i+1) == Some(&"25".to_string())), "Should contain -crf 25");
    assert!(args.iter().position(|a| a == "-preset").map_or(false, |i| args.get(i+1) == Some(&"7".to_string())), "Should contain -preset 7");
    assert!(args.iter().any(|a| a == "libsvtav1"), "Should contain codec");
    assert!(args.iter().position(|a| a == "-b:a:0").map_or(false, |i| args.get(i+1) == Some(&"128k".to_string())), "Should contain audio bitrate for stream 0");
    // Check that crop filter is NOT present
    assert!(!args.iter().any(|a| a.contains("crop=")), "Should not contain crop filter");

    Ok(())
}

#[test]
fn test_run_ffmpeg_encode_args_with_crop() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempdir()?;
    let input_path = tmp.path().join("input_crop.mkv");
    let output_path = tmp.path().join("output_crop.mkv");
    let crop_filter = "crop=1920:800:0:140".to_string();

    let params = EncodeParams {
        input_path: input_path.clone(),
        output_path: output_path.clone(),
        quality: 27,
        preset: 6,
        crop_filter: Some(crop_filter.clone()),
        audio_channels: vec![6], // 5.1
        duration: 120.0,
        enable_denoise: true, // Added field
    };

    let mock_spawner = MockFfmpegSpawner::new();
    mock_spawner.add_success_expectation("libsvtav1", vec![], false);

    let log_callback = |_msg: &str| {};

    let result = run_ffmpeg_encode(&mock_spawner, &params, log_callback);
    assert!(result.is_ok());

    let calls = mock_spawner.get_received_calls();
    assert_eq!(calls.len(), 1);
    let args = &calls[0];

    // Check that crop filter IS present and correct
    // Expect crop AND denoise filter since enable_denoise is true by default
    let expected_filter_arg = format!("[0:v:0]{},hqdn3d[vout]", crop_filter);
    assert!(args.iter().position(|a| a == "-filter_complex").map_or(false, |i| args.get(i+1) == Some(&expected_filter_arg)), "Should contain correct -filter_complex arg");
    assert!(args.iter().any(|a| a == "-map"), "Should contain -map");
    assert!(args.iter().any(|a| a == "[vout]"), "Should map [vout]");
    assert!(args.iter().position(|a| a == "-b:a:0").map_or(false, |i| args.get(i+1) == Some(&"256k".to_string())), "Should contain correct audio bitrate for 5.1");


    Ok(())
}

// TODO: Add tests for different audio channel counts, edge cases, etc.