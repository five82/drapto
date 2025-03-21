# drapto-core

Core library for the Drapto video encoding tool, providing reusable components for video processing, encoding, and validation.

## Features

- Media information extraction and analysis
- Video and audio validation
- Format detection (HDR, Dolby Vision, etc.)
- Scene detection algorithms
- Encoding pipeline abstractions

## Usage

This library is primarily used by the `drapto-cli` crate for the command-line interface, but can be used independently in other applications requiring video processing capabilities.

```rust
use drapto_core::media::probe::MediaInfo;

fn example() -> drapto_core::error::Result<()> {
    let media_info = MediaInfo::from_path("video.mp4")?;
    println!("Duration: {} seconds", media_info.duration().unwrap_or(0.0));
    Ok(())
}
```