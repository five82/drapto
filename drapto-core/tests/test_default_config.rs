//! Tests for default configuration values
//!
//! These tests verify:
//! - Proper initialization of default configuration values
//! - Core default settings for encoding, scene detection, and resources
//! - Consistency of default values across implementations
//! - Correct initialization of configuration without external influences

use drapto_core::config::Config;
use std::env;

#[test]
fn test_default_config() {
    // Clear any environment variables that might interfere with defaults
    env::remove_var("DRAPTO_SCENE_THRESHOLD");
    env::remove_var("DRAPTO_HDR_SCENE_THRESHOLD");
    env::remove_var("DRAPTO_MEMORY_THRESHOLD");
    env::remove_var("DRAPTO_PRESET");
    
    let config = Config::new();
    
    // Test some defaults
    assert_eq!(config.scene_detection.scene_threshold, 40.0);
    assert_eq!(config.scene_detection.hdr_scene_threshold, 30.0);
    assert_eq!(config.resources.memory_threshold, 0.7);
    assert_eq!(config.video.preset, 6);
}