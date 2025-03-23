use std::path::Path;
use std::env;

use drapto_core::encoding::segmentation::segment_video;
use drapto_core::config::Config;
use drapto_core::detection::scene::detect_scenes_with_config;

/// Integration test for video segmentation
///
/// Note: This test requires a sample video file to be present
/// at the path specified by the TEST_VIDEO_PATH environment variable.
/// The test will be skipped if the environment variable is not set.
#[test]
fn test_video_segmentation() {
    // Check if test video path is set in environment
    let test_video_path = match env::var("TEST_VIDEO_PATH") {
        Ok(path) => path,
        Err(_) => {
            // Skip test if environment variable not set
            println!("Skipping segmentation test as TEST_VIDEO_PATH not set");
            return;
        }
    };
    
    let video_path = Path::new(&test_video_path);
    if !video_path.exists() {
        // Skip test if file doesn't exist
        println!("Skipping segmentation test as test video not found at {}", test_video_path);
        return;
    }
    
    // Create test config with standard values
    let mut config = Config::default();
    config.scene_detection.scene_threshold = 25.0;
    config.scene_detection.hdr_scene_threshold = 15.0;
    config.scene_detection.min_segment_length = 1.0;
    config.scene_detection.max_segment_length = 30.0;
    
    // Create temp directory for segments
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let segments_dir = temp_dir.path();
    
    // First test scene detection
    let scenes = detect_scenes_with_config(video_path, &config)
        .expect("Scene detection failed");
    
    // We can't assert exact scene count as it depends on the test video,
    // but we can check that some scenes were detected
    println!("Detected {} scenes", scenes.len());
    assert!(!scenes.is_empty(), "No scenes detected in test video");
    
    // Test segmentation
    let segments = segment_video(video_path, segments_dir, &config)
        .expect("Video segmentation failed");
    
    // Check that segments were created
    assert!(!segments.is_empty(), "No segments were created");
    println!("Created {} segments", segments.len());
    
    // Validation of segments is done internally by segment_video
    
    // Cleanup temp directory happens automatically when temp_dir is dropped
}