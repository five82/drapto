//! Media File Multiplexing Example
//!
//! This example demonstrates how to use the Muxer to combine separate video and audio tracks:
//! 1. Creating a muxer with custom synchronization options
//! 2. Handling multiple audio tracks with a single video track
//! 3. Verifying the muxing operation's success
//! 4. Using custom synchronization thresholds for more flexible A/V matching
//!
//! Run with:
//! ```
//! cargo run --example muxer <video_file> <audio_file> [additional_audio_files...] <output_file>
//! ```
//!
//! The example supports muxing one video track with multiple audio tracks.

use std::path::Path;
use std::fs;
use std::env;
use log::{info, error, LevelFilter};
use drapto_core::encoding::muxer::{Muxer, MuxOptions};
use drapto_core::util::logging;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    logging::init_with_level(LevelFilter::Debug, false);
    
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 4 {
        eprintln!("Usage: {} <video_file> <audio_file> <output_file>", args[0]);
        eprintln!("  Additional audio files can be specified after the first audio file");
        std::process::exit(1);
    }
    
    let video_file = &args[1];
    
    // Collect all audio files (args[2] and beyond, except the last which is output)
    let audio_files: Vec<&str> = args[2..args.len()-1].iter().map(|s| s.as_str()).collect();
    
    let output_file = &args[args.len()-1];
    
    // Create output directory if needed
    if let Some(parent) = Path::new(output_file).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    
    // Create muxer with custom options
    let mux_options = MuxOptions {
        sync_threshold: 0.2, // Slightly more tolerant
        allow_container_duration: true,
    };
    
    let muxer = Muxer::with_options(mux_options);
    
    // Mux the files
    info!("Muxing video: {}", video_file);
    for audio in &audio_files {
        info!("   with audio: {}", audio);
    }
    info!("   to output: {}", output_file);
    
    match muxer.mux_tracks(video_file, &audio_files, output_file, &muxer.options) {
        Ok(output_path) => {
            info!("Successfully muxed tracks to: {}", output_path.display());
            Ok(())
        },
        Err(e) => {
            error!("Failed to mux tracks: {}", e);
            Err(e.into())
        }
    }
}