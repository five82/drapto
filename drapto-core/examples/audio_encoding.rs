//! Audio Encoding with Opus Example
//!
//! This example demonstrates high-quality audio encoding capabilities:
//! 1. Analyzing input files to detect audio streams and properties
//! 2. Configuring the Opus encoder with specific quality settings
//! 3. Encoding all audio tracks from an input file to Opus format
//! 4. Verifying and reporting on encoded audio quality and size
//!
//! The example shows how to access stream properties, configure encoding
//! parameters, and process multiple audio tracks from a single input file.
//!
//! Run with:
//! ```
//! cargo run --example audio_encoding <input_file>
//! ```

use std::path::PathBuf;
use log::{info, error};
use drapto_core::encoding::audio::{OpusEncoder, AudioEncoderConfig};
use drapto_core::media::info::MediaInfo;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <input_file>", args[0]);
        std::process::exit(1);
    }
    
    let input_file = PathBuf::from(&args[1]);
    
    if !input_file.exists() {
        eprintln!("Input file not found: {:?}", input_file);
        std::process::exit(1);
    }
    
    // Get information about the input file
    info!("Analyzing input file: {:?}", input_file);
    let media_info = MediaInfo::from_path(&input_file)?;
    
    let audio_streams = media_info.audio_streams();
    if audio_streams.is_empty() {
        error!("No audio streams found in the input file");
        std::process::exit(1);
    }
    
    info!("Found {} audio streams:", audio_streams.len());
    for (i, stream) in audio_streams.iter().enumerate() {
        let channels = stream.properties.get("channels")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
            
        let sample_rate = stream.properties.get("sample_rate")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
            
        info!(
            "Stream #{}: codec={}, channels={}, sample_rate={}",
            i, stream.codec_name, channels, sample_rate
        );
    }
    
    // Create a temporary directory for encoded files
    let temp_dir = std::env::temp_dir().join("drapto_audio_example");
    std::fs::create_dir_all(&temp_dir)?;
    info!("Created temporary directory: {:?}", temp_dir);
    
    // Configure and create the audio encoder
    let config = AudioEncoderConfig {
        compression_level: 10,
        frame_duration: 20,
        vbr: true,
        application: "audio".to_string(),
        temp_dir: temp_dir.clone(),
    };
    
    let encoder = OpusEncoder::with_config(config);
    
    // Encode all audio tracks
    info!("Starting audio encoding");
    let encoded_tracks = encoder.encode_audio_tracks(&input_file)?;
    
    if encoded_tracks.is_empty() {
        info!("No audio tracks were encoded");
        return Ok(());
    }
    
    // Verify the encoded files
    info!("Successfully encoded {} audio tracks:", encoded_tracks.len());
    for (i, track_path) in encoded_tracks.iter().enumerate() {
        let track_info = MediaInfo::from_path(track_path)?;
        let audio_stream = &track_info.audio_streams()[0];
        
        let bitrate = audio_stream.properties.get("bit_rate")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
            
        info!(
            "Track #{}: codec={}, size={:.2} MB, bitrate={}",
            i, 
            audio_stream.codec_name,
            track_path.metadata()?.len() as f64 / (1024.0 * 1024.0),
            bitrate
        );
    }
    
    info!("Encoded files are located in: {:?}", temp_dir);
    info!("Example completed successfully");
    
    Ok(())
}