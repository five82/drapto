use drapto_core::config::Config;
use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example 1: Use default configuration with overrides via environment variables
    println!("Example 1: Default configuration with environment variables");
    env::set_var("DRAPTO_SCENE_THRESHOLD", "35.0");
    env::set_var("DRAPTO_MEMORY_THRESHOLD", "0.8");
    
    let config = Config::new()
        .with_input("input.mp4")
        .with_output("output.mp4");
    
    println!("Scene threshold: {}", config.scene_detection.scene_threshold);
    println!("Memory threshold: {}", config.resources.memory_threshold);
    
    // Example 2: Load from a configuration file
    println!("\nExample 2: Load configuration from file");
    
    // For this example, we'll just create a temporary example file
    let example_config_path = PathBuf::from("drapto_example.toml");
    if !example_config_path.exists() {
        let example_config = r#"
[scene_detection]
scene_threshold = 30.0
min_segment_length = 3.0

[video]
target_quality = 90.0
disable_crop = true
"#;
        std::fs::write(&example_config_path, example_config)?;
    }
    
    let file_config = Config::from_file(&example_config_path)?;
    
    println!("Scene threshold: {}", file_config.scene_detection.scene_threshold);
    println!("Min segment length: {}", file_config.scene_detection.min_segment_length);
    println!("Target quality: {:?}", file_config.video.target_quality);
    println!("Disable crop: {}", file_config.video.disable_crop);
    
    // Clean up temporary file
    std::fs::remove_file(example_config_path)?;
    
    // Example 3: Create a config from code and save to file
    println!("\nExample 3: Create and save configuration");
    
    let _custom_config = Config::new()
        .with_input("my_video.mkv")
        .with_output("output.mp4");
    
    // We would normally save it
    // _custom_config.save_to_file("drapto_custom.toml")?;
    
    println!("Configuration created and can be saved to file");
    
    // Example 4: Show layered config resolution (default -> file -> env vars -> cmd args)
    println!("\nExample 4: Layered configuration resolution");
    
    // 1. Start with defaults
    let mut layered_config = Config::new();
    println!("Default scene threshold: {}", layered_config.scene_detection.scene_threshold);
    
    // 2. Apply file config if it exists (only simulated here)
    // In real code, this would be: if file_path.exists() { layered_config = Config::from_file(file_path)?; }
    layered_config.scene_detection.scene_threshold = 25.0;
    println!("After file config: {}", layered_config.scene_detection.scene_threshold);
    
    // 3. Environment variables are already applied via the Default implementations
    // But we can simulate a command-line argument override
    layered_config.scene_detection.scene_threshold = 20.0;
    println!("After CLI args: {}", layered_config.scene_detection.scene_threshold);
    
    Ok(())
}