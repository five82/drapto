//! Example demonstrating segment merger functionality
//!
//! This example shows how to:
//! 1. Create multiple video segments from a source file
//! 2. Merge those segments back into a single file
//! 3. Validate the merged output

use std::path::{Path, PathBuf};
use std::env;
use log::{info, error, LevelFilter};
use serde_json::Value;

use drapto_core::error::Result;
use drapto_core::encoding::{segmentation, merger};
use drapto_core::config::Config;
use drapto_core::media::MediaInfo;
use drapto_core::util::logging;

fn main() -> Result<()> {
    // Initialize logging
    logging::init_logger(LevelFilter::Info);
    
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: segment_merger <input_video_file>");
        std::process::exit(1);
    }
    
    let input_file = PathBuf::from(&args[1]);
    if !input_file.exists() {
        error!("Input file not found: {:?}", input_file);
        std::process::exit(1);
    }
    
    // Create temp directory for segments
    let temp_dir = std::env::temp_dir().join("drapto_segments");
    let output_dir = std::env::temp_dir().join("drapto_output");
    
    // Ensure directories exist
    if !temp_dir.exists() {
        std::fs::create_dir_all(&temp_dir)?;
    }
    if !output_dir.exists() {
        std::fs::create_dir_all(&output_dir)?;
    }
    
    // Print input file information
    print_media_info(&input_file)?;
    
    // 1. Segment the video
    info!("Segmenting video: {:?}", input_file);
    let segments = segment_video(&input_file, &temp_dir)?;
    
    // Get the merged output filename
    let output_filename = input_file.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "output.mkv".to_string());
    
    let merged_output = output_dir.join(format!("merged_{}", output_filename));
    
    // 2. Merge the segments
    info!("Merging {} segments into: {:?}", segments.len(), merged_output);
    
    // Create merger with custom options for this example
    let options = merger::MergeOptions {
        copy_streams: true,
        faststart: true,
        generate_pts: true,
        copy_metadata: true,
        expected_codec: None,  // Don't check codec for this example
        duration_tolerance: 1.0,
        start_time_tolerance: 0.5,
    };
    
    let segment_merger = merger::SegmentMerger::with_options(options);
    let merged_file = segment_merger.merge_segments(&segments, &merged_output)?;
    
    // 3. Print final output information
    info!("Successfully merged segments. Final output: {:?}", merged_file);
    print_media_info(&merged_file)?;
    
    // Compare durations of input and output
    let input_info = MediaInfo::from_path(&input_file)?;
    let output_info = MediaInfo::from_path(&merged_file)?;
    
    let input_duration = input_info.duration().unwrap_or(0.0);
    let output_duration = output_info.duration().unwrap_or(0.0);
    
    info!("Input duration: {:.2}s", input_duration);
    info!("Output duration: {:.2}s", output_duration);
    info!("Difference: {:.2}s", (output_duration - input_duration).abs());
    
    Ok(())
}

/// Segment a video file into multiple parts
fn segment_video(input_file: &Path, temp_dir: &Path) -> Result<Vec<PathBuf>> {
    // Create simple config for segmentation
    let config = Config::default();
    
    // Force some scene times for this example
    // In a real application, you would use scene detection
    let input_info = MediaInfo::from_path(input_file)?;
    let total_duration = input_info.duration().unwrap_or(60.0);
    
    // Create segments at 25% intervals of the total duration
    let segment_count = 4;
    let segment_duration = total_duration / segment_count as f64;
    
    let mut scene_times = Vec::new();
    for i in 1..segment_count {
        scene_times.push(i as f64 * segment_duration);
    }
    
    info!("Creating {} segments at times: {:?}", segment_count, scene_times);
    
    // Build segmentation command
    let mut cmd = segmentation::build_segment_command(
        input_file,
        temp_dir,
        &scene_times,
        None,
    );
    
    // Execute segmentation
    drapto_core::util::command::run_command(&mut cmd)?;
    
    // Get segment list
    let mut segments = Vec::new();
    for entry in std::fs::read_dir(temp_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "mkv") {
            segments.push(path);
        }
    }
    
    // Sort segments by name
    segments.sort();
    
    info!("Created {} segments", segments.len());
    
    Ok(segments)
}

/// Print media file information
fn print_media_info(file_path: &Path) -> Result<()> {
    let info = MediaInfo::from_path(file_path)?;
    
    println!("\nMedia Information for {:?}:", file_path.file_name().unwrap_or_default());
    println!("-------------------------");
    
    // Format information
    if let Some(format) = &info.format {
        println!("Format: {} ({})", format.format_name, format.format_long_name);
        println!("Duration: {:.2} seconds", info.duration().unwrap_or(0.0));
        println!("Size: {:.2} MB", format.size as f64 / (1024.0 * 1024.0));
        println!("Bit rate: {} kb/s", format.bit_rate.unwrap_or(0) / 1000);
    }
    
    // Video stream
    if let Some(video) = info.primary_video_stream() {
        println!("\nVideo Stream:");
        println!("  Codec: {} ({})", 
                 video.codec_name, 
                 video.codec_long_name.as_deref().unwrap_or("unknown"));
        println!("  Resolution: {}x{}", 
                 get_property_as_string(&video.properties, "width"),
                 get_property_as_string(&video.properties, "height"));
        if let Some(fps) = video.properties.get("r_frame_rate") {
            println!("  Frame rate: {}", fps);
        }
        if let Some(pixel_fmt) = video.properties.get("pix_fmt") {
            println!("  Pixel format: {}", pixel_fmt);
        }
    }
    
    // Audio streams
    for (i, audio) in info.audio_streams().iter().enumerate() {
        println!("\nAudio Stream #{}:", i);
        println!("  Codec: {} ({})", 
                 audio.codec_name, 
                 audio.codec_long_name.as_deref().unwrap_or("unknown"));
        if let Some(channels) = audio.properties.get("channels") {
            println!("  Channels: {}", channels);
        }
        if let Some(sample_rate) = audio.properties.get("sample_rate") {
            println!("  Sample rate: {} Hz", sample_rate);
        }
        if let Some(bit_rate) = audio.properties.get("bit_rate") {
            if let Some(bit_rate_str) = bit_rate.as_str() {
                if let Ok(bit_rate_val) = bit_rate_str.parse::<i64>() {
                    println!("  Bit rate: {} kb/s", bit_rate_val / 1000);
                }
            }
        }
    }
    
    println!("-------------------------\n");
    
    Ok(())
}

/// Helper to safely get a property value as a string
fn get_property_as_string(properties: &std::collections::HashMap<String, Value>, key: &str) -> String {
    match properties.get(key) {
        Some(value) => value.to_string().trim_matches('"').to_string(), 
        None => "?".to_string()
    }
}