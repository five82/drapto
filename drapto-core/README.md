# drapto-core

Core library for the Drapto video encoding system, providing functionality for:

- Media information analysis
- Scene detection and video segmentation 
- Parallel encoding management
- Validation of encoded media
- Configuration management

This crate contains the core functionality of Drapto without the command-line interface, 
making it suitable for integration into other Rust applications.

## Features

- **Media Analysis**: Extract detailed information from media files using FFmpeg
- **Scene Detection**: Identify scene changes for optimal segmentation
- **Parallel Encoding**: Memory-aware parallel encoding of video segments
- **Content Validation**: Verify encoded content meets quality standards
- **Flexible Configuration**: Layered configuration through files, environment variables, and code

## Usage

Add `drapto-core` to your `Cargo.toml`:

```toml
[dependencies]
drapto-core = "0.1.0"
```

### Basic Example

```rust
use drapto_core::config::Config;
use drapto_core::encoding::pipeline::EncodingPipeline;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration
    let config = Config::new()
        .with_input("input.mkv")
        .with_output("output.mp4");
    
    // Validate configuration
    config.validate()?;
    
    // Create and run the encoding pipeline
    let pipeline = EncodingPipeline::new(config);
    pipeline.run()?;
    
    println!("Encoding complete!");
    Ok(())
}
```

### Media Information Example

```rust
use drapto_core::media::info::MediaInfo;

fn example() -> drapto_core::error::Result<()> {
    let media_info = MediaInfo::from_path("video.mp4")?;
    println!("Duration: {} seconds", media_info.duration().unwrap_or(0.0));
    
    // Get video details
    if let Some(video) = media_info.primary_video_stream() {
        println!("Video codec: {}", video.codec_name);
        println!("Resolution: {}x{}", video.width, video.height);
    }
    
    Ok(())
}
```

### Configuration

The crate supports a flexible configuration system:

```rust
use drapto_core::config::Config;
use std::path::PathBuf;

// Load from TOML file
let config = Config::from_file("drapto.toml")?;

// Create configuration programmatically
let mut config = Config::default();
config.video.target_quality = Some(90.0);
config.scene_detection.scene_threshold = 35.0;
config.resources.parallel_jobs = 4;

// Environment variables (e.g., DRAPTO_SCENE_THRESHOLD=35.0) are 
// automatically applied when creating a new configuration
```

For more detailed examples, see the `examples/` directory:
- `config_example.rs`: Demonstrates configuration loading and management
- `parallel_encoding.rs`: Shows how to use the parallel encoding system

## Configuration

See the main [Configuration Guide](../docs/configuration.md) for details on all available configuration options.