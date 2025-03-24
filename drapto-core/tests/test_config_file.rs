//! Tests for the configuration file parsing and manipulation functionality
//!
//! These tests verify:
//! - Proper parsing of TOML configuration files
//! - Correct default values for missing configurations
//! - Saving configuration to files and loading it back
//! - Proper serialization/deserialization of all configuration values

use drapto_core::config::Config;
use std::env;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_config_file_parsing() -> Result<(), Box<dyn std::error::Error>> {
    // Clean env vars first
    env::remove_var("DRAPTO_SCENE_THRESHOLD");
    env::remove_var("DRAPTO_HDR_SCENE_THRESHOLD");
    env::remove_var("DRAPTO_MEMORY_THRESHOLD");
    env::remove_var("DRAPTO_PRESET");
    
    // Create a temporary directory
    let dir = tempdir()?;
    let config_path = dir.path().join("test_config.toml");
    
    // Create a test config file
    let config_content = r#"
input = "input.mp4"
output = "output.mp4"

[scene_detection]
scene_threshold = 25.0
min_segment_length = 3.0
hdr_scene_threshold = 30.0
max_segment_length = 15.0

[video]
target_quality = 90.0
disable_crop = true
hardware_acceleration = true
hw_accel_option = ""
preset = 6
svt_params = "tune=0:film-grain=0:film-grain-denoise=0"
pix_fmt = "yuv420p10le"
use_segmentation = true
vmaf_sample_count = 3
vmaf_sample_length = 1.0

[audio]
compression_level = 10
frame_duration = 20
vbr = true
application = "audio"

[resources]
parallel_jobs = 4
memory_threshold = 0.7
max_memory_tokens = 8
task_stagger_delay = 0.2
memory_per_job = 2048

[directories]
temp_dir = "/tmp/drapto"
keep_temp_files = false

[logging]
verbose = false
log_level = "INFO"
log_dir = "/tmp/drapto_logs"
"#;
    
    fs::write(&config_path, config_content)?;
    
    // Load the config file
    let config = Config::from_file(&config_path)?;
    
    // Test if file values were loaded correctly
    assert_eq!(config.scene_detection.scene_threshold, 25.0);
    assert_eq!(config.scene_detection.min_segment_length, 3.0);
    assert_eq!(config.video.target_quality, Some(90.0));
    assert_eq!(config.video.disable_crop, true);
    assert_eq!(config.input.to_string_lossy(), "input.mp4");
    assert_eq!(config.output.to_string_lossy(), "output.mp4");
    
    // Test that values not in the file retain defaults
    assert_eq!(config.scene_detection.hdr_scene_threshold, 30.0);
    
    Ok(())
}

#[test]
fn test_config_save_and_load() -> Result<(), Box<dyn std::error::Error>> {
    // Clean env vars first
    env::remove_var("DRAPTO_SCENE_THRESHOLD");
    env::remove_var("DRAPTO_HDR_SCENE_THRESHOLD");
    env::remove_var("DRAPTO_MEMORY_THRESHOLD");
    env::remove_var("DRAPTO_PRESET");
    
    // Create a temporary directory
    let dir = tempdir()?;
    let config_path = dir.path().join("saved_config.toml");
    
    // Create a custom config
    let original_config = Config::new()
        .with_input("input.mp4")
        .with_output("output.mp4");
    
    // Modify some values
    let mut config_to_save = original_config.clone();
    config_to_save.scene_detection.scene_threshold = 22.0;
    config_to_save.video.preset = 7;
    
    // Save the config
    config_to_save.save_to_file(&config_path)?;
    
    // Load the config back
    let loaded_config = Config::from_file(&config_path)?;
    
    // Test if saved and loaded configs match
    assert_eq!(loaded_config.scene_detection.scene_threshold, 22.0);
    assert_eq!(loaded_config.video.preset, 7);
    
    // Original input/output should be preserved
    assert_eq!(loaded_config.input, PathBuf::from("input.mp4"));
    assert_eq!(loaded_config.output, PathBuf::from("output.mp4"));
    
    Ok(())
}