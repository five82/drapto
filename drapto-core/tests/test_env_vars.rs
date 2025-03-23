use drapto_core::config::Config;
use std::env;

#[test]
fn test_env_var_overrides() {
    // First clear any existing env vars
    env::remove_var("DRAPTO_SCENE_THRESHOLD");
    env::remove_var("DRAPTO_MEMORY_THRESHOLD");
    env::remove_var("DRAPTO_PRESET");
    
    // Set environment variables
    env::set_var("DRAPTO_SCENE_THRESHOLD", "35.0");
    env::set_var("DRAPTO_MEMORY_THRESHOLD", "0.8");
    env::set_var("DRAPTO_PRESET", "8");
    
    // Create new config which should pick up env vars
    let config = Config::new();
    
    // Test if env vars were applied
    assert_eq!(config.scene_detection.scene_threshold, 35.0);
    assert_eq!(config.resources.memory_threshold, 0.8);
    assert_eq!(config.video.preset, 8);
    
    // Clean up
    env::remove_var("DRAPTO_SCENE_THRESHOLD");
    env::remove_var("DRAPTO_MEMORY_THRESHOLD");
    env::remove_var("DRAPTO_PRESET");
}