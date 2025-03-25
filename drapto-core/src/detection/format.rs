//! Media format detection module
//!
//! Responsibilities:
//! - Detect HDR/SDR video content
//! - Detect black bars for cropping
//! - Analyze color spaces and bit depths
//!
//! This module provides tools to analyze media files for format-specific
//! characteristics that impact encoding decisions.

use std::path::Path;
use std::collections::HashMap;
use std::process::Command;
use std::str::FromStr;
use regex::Regex;
use log::{info, error};

use crate::error::{DraptoError, Result};
use crate::media::MediaInfo;
use crate::util::command;

/// Video properties structure
#[derive(Debug, Clone)]
pub struct VideoProperties {
    /// Color properties (transfer, primaries, space)
    pub color_props: HashMap<String, String>,
    /// Video dimensions (width, height)
    pub dimensions: (u32, u32),
    /// Video duration in seconds
    pub duration: f64,
}

/// Determine crop threshold based on color properties
/// Returns a tuple of (crop_threshold, is_hdr)
fn determine_crop_threshold(
    color_transfer: &str, 
    color_primaries: &str, 
    color_space: &str,
    config: &crate::config::Config
) -> (i32, bool) {
    let mut crop_threshold = config.crop_detection.sdr_threshold;
    let mut is_hdr = false;
    
    let hdr_transfer_regex = regex::Regex::new(r"^(smpte2084|arib-std-b67|smpte428|bt2020-10|bt2020-12)$").unwrap();
    
    if hdr_transfer_regex.is_match(color_transfer)
        || color_primaries == "bt2020"
        || color_space == "bt2020nc"
        || color_space == "bt2020c"
    {
        is_hdr = true;
        crop_threshold = config.crop_detection.hdr_threshold;
        info!("HDR content detected, adjusting detection sensitivity");
    }
    
    (crop_threshold, is_hdr)
}

/// Run a set of ffmpeg commands to sample black levels for HDR content
/// Returns an updated crop threshold based on black level analysis
fn run_hdr_blackdetect<P: AsRef<Path>>(
    input_file: P, 
    crop_threshold: i32, 
    config: &crate::config::Config
) -> i32 {
    let input_path = input_file.as_ref();
    
    let mut cmd = Command::new("ffmpeg");
    cmd.args([
        "-hide_banner",
        "-i", input_path.to_str().unwrap_or_default(),
        "-vf", "select='eq(n,0)+eq(n,100)+eq(n,200)',blackdetect=d=0:pic_th=0.1",
        "-f", "null", "-"
    ]);
    
    match command::run_command(&mut cmd) {
        Ok(output) => {
            let output_stderr = String::from_utf8_lossy(&output.stderr);
            let regex = Regex::new(r"black_level:\s*([0-9.]+)").unwrap();
            
            let mut black_levels = Vec::new();
            for cap in regex.captures_iter(&output_stderr) {
                if let Some(level) = cap.get(1) {
                    if let Ok(value) = f32::from_str(level.as_str()) {
                        black_levels.push(value);
                    }
                }
            }
            
            if !black_levels.is_empty() {
                let avg_black_level = black_levels.iter().sum::<f32>() / black_levels.len() as f32;
                let black_level = avg_black_level as i32;
                return ((black_level as f32) * config.crop_detection.hdr_black_level_multiplier) as i32;
            }
            
            crop_threshold
        },
        Err(e) => {
            error!("Error during HDR black level analysis: {}", e);
            crop_threshold
        }
    }
}


/// Get video properties from media info
///
/// # Returns
///
/// * `Result<VideoProperties>` - Structured video properties data
fn get_video_properties<P: AsRef<Path>>(input_file: P) -> Result<VideoProperties> {
    let media_info = MediaInfo::from_path(input_file)?;
    
    // Get primary video stream
    let video_stream = media_info.primary_video_stream()
        .ok_or_else(|| DraptoError::MediaFile("No video stream found".to_string()))?;
    
    // Get color properties
    let mut color_props = HashMap::new();
    color_props.insert("transfer".to_string(), video_stream.properties.get("color_transfer")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string());
    
    color_props.insert("primaries".to_string(), video_stream.properties.get("color_primaries")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string());
    
    color_props.insert("space".to_string(), video_stream.properties.get("color_space")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string());
    
    // Get dimensions
    let dimensions = media_info.video_dimensions()
        .ok_or_else(|| DraptoError::MediaFile("Unable to determine video dimensions".to_string()))?;
    
    // Get duration
    let duration = media_info.duration()
        .ok_or_else(|| DraptoError::MediaFile("Unable to determine video duration".to_string()))?;
    
    Ok(VideoProperties {
        color_props,
        dimensions,
        duration,
    })
}

/// Calculate how much time to skip at the end for credits
fn calculate_credits_skip(duration: f64, config: &crate::config::Config) -> f64 {
    if duration > 3600.0 {
        config.crop_detection.credits_skip_movie  // Skip for movies > 1 hour
    } else if duration > 1200.0 {
        config.crop_detection.credits_skip_episode  // Skip for content > 20 minutes
    } else if duration > 300.0 {
        config.crop_detection.credits_skip_short  // Skip for content > 5 minutes
    } else {
        0.0
    }
}

/// Run ffmpeg cropdetect and analyze results
fn run_cropdetect<P: AsRef<Path>>(
    input_file: P,
    crop_threshold: i32,
    dimensions: (u32, u32),
    duration: f64,
    config: &crate::config::Config
) -> Result<Option<String>> {
    let (orig_width, orig_height) = dimensions;
    
    // Calculate sampling parameters
    let interval = config.crop_detection.sampling_interval;
    let mut total_samples = (duration / interval as f64) as i32;
    
    if total_samples < config.crop_detection.min_sample_count {
        total_samples = config.crop_detection.min_sample_count;
    }
    
    let cropdetect_filter = format!("select='{}',cropdetect=limit={}:round=2:reset=1", 
                                  config.crop_detection.frame_selection, crop_threshold);
    let frames = total_samples * 2;
    
    let mut cmd = Command::new("ffmpeg");
    cmd.args([
        "-hide_banner",
        "-i", input_file.as_ref().to_str().unwrap_or_default(),
        "-vf", &cropdetect_filter,
        "-frames:v", &frames.to_string(),
        "-f", "null", "-"
    ]);
    
    match command::run_command(&mut cmd) {
        Ok(output) => {
            let output_stderr = String::from_utf8_lossy(&output.stderr);
            let regex = Regex::new(r"crop=(\d+):(\d+):(\d+):(\d+)").unwrap();
            
            // Parse crop values
            let mut valid_crops = Vec::new();
            for cap in regex.captures_iter(&output_stderr) {
                if cap.len() >= 5 {
                    if let (Ok(w), Ok(h), Ok(x), Ok(y)) = (
                        u32::from_str(cap.get(1).unwrap().as_str()),
                        u32::from_str(cap.get(2).unwrap().as_str()),
                        u32::from_str(cap.get(3).unwrap().as_str()),
                        u32::from_str(cap.get(4).unwrap().as_str())
                    ) {
                        if w == orig_width {
                            valid_crops.push((w, h, x, y));
                        }
                    }
                }
            }
            
            if valid_crops.is_empty() {
                info!("No crop values detected, using full dimensions");
                return Ok(Some(format!("crop={}:{}:0:0", orig_width, orig_height)));
            }
            
            // Analyze crop heights
            let crop_heights: Vec<u32> = valid_crops.iter()
                .map(|(_, h, _, _)| *h)
                .filter(|h| *h >= config.crop_detection.min_height)
                .collect();
            
            if crop_heights.is_empty() {
                return Ok(Some(format!("crop={}:{}:0:0", orig_width, orig_height)));
            }
            
            // Find most common height
            let mut height_counts = HashMap::new();
            for height in crop_heights {
                *height_counts.entry(height).or_insert(0) += 1;
            }
            
            let most_common_height = height_counts
                .iter()
                .max_by_key(|(_, &count)| count)
                .map(|(height, _)| *height)
                .unwrap_or(orig_height);
            
            // Calculate black bars
            let black_bar_size = (orig_height - most_common_height) / 2;
            let black_bar_percent = (black_bar_size * 100) / orig_height;
            
            if black_bar_size > 0 {
                info!("Found black bars: {} pixels ({}% of height)",
                      black_bar_size, black_bar_percent);
            } else {
                info!("No significant black bars detected");
            }
            
            if black_bar_percent > config.crop_detection.min_black_bar_percent {
                Ok(Some(format!("crop={}:{}:0:{}", orig_width, most_common_height, black_bar_size)))
            } else {
                Ok(Some(format!("crop={}:{}:0:0", orig_width, orig_height)))
            }
        },
        Err(e) => {
            error!("Error during crop detection: {}", e);
            Ok(None)
        }
    }
}

/// Detect black bars and return an ffmpeg crop filter string
///
/// # Arguments
///
/// * `input_file` - Path to input video file
/// * `disable_crop` - If true, skip crop detection
/// * `config` - Optional configuration, if not provided will use default values
///
/// # Returns
///
/// * `Result<(Option<String>, bool)>` - A tuple containing:
///   - Optional crop filter string (like "crop=1920:800:0:140"), None if disabled or failed
///   - Boolean indicating if the content is HDR
pub fn detect_crop<P: AsRef<Path>>(
    input_file: P, 
    disable_crop: Option<bool>,
    config: Option<&crate::config::Config>
) -> Result<(Option<String>, bool)> {
    let disable_crop = disable_crop.unwrap_or(false);
    let default_config = crate::config::Config::default();
    let config = config.unwrap_or(&default_config);
    
    if disable_crop {
        info!("Crop detection disabled");
        return Ok((None, false));
    }
    
    info!("Analyzing video for black bars...");
    
    // Get video properties
    let props = get_video_properties(&input_file)?;
    
    if props.dimensions.0 == 0 || props.dimensions.1 == 0 || props.duration <= 0.0 {
        return Ok((None, false));
    }
    
    // Determine crop threshold and HDR status
    let (mut crop_threshold, is_hdr) = determine_crop_threshold(
        props.color_props.get("transfer").unwrap_or(&String::new()),
        props.color_props.get("primaries").unwrap_or(&String::new()),
        props.color_props.get("space").unwrap_or(&String::new()),
        config
    );
    
    // For HDR content, analyze black levels
    if is_hdr {
        crop_threshold = run_hdr_blackdetect(&input_file, crop_threshold, config);
        crop_threshold = crop_threshold.clamp(
            config.crop_detection.min_threshold, 
            config.crop_detection.max_threshold
        );
    }
    
    // Adjust duration for credits
    let credits_skip = calculate_credits_skip(props.duration, config);
    let adjusted_duration = if credits_skip > 0.0 {
        props.duration - credits_skip
    } else {
        props.duration
    };
    
    // Run crop detection
    let crop_filter = run_cropdetect(
        &input_file, 
        crop_threshold, 
        props.dimensions, 
        adjusted_duration,
        config
    )?;
    
    Ok((crop_filter, is_hdr))
}

/// Check if media has HDR content
///
/// # Arguments
///
/// * `media_info` - Media information
///
/// # Returns
///
/// * `bool` - True if HDR content is detected
pub fn has_hdr(media_info: &MediaInfo) -> bool {
    if let Some(video_stream) = media_info.primary_video_stream() {
        // Check color transfer
        if let Some(color_transfer) = video_stream.properties.get("color_transfer").and_then(|v| v.as_str()) {
            let hdr_transfer_regex = regex::Regex::new(r"^(smpte2084|arib-std-b67|smpte428|bt2020-10|bt2020-12)$").unwrap();
            if hdr_transfer_regex.is_match(color_transfer) {
                return true;
            }
        }
        
        // Check color primaries
        if let Some(color_primaries) = video_stream.properties.get("color_primaries").and_then(|v| v.as_str()) {
            if color_primaries == "bt2020" {
                return true;
            }
        }
        
        // Check color space
        if let Some(color_space) = video_stream.properties.get("color_space").and_then(|v| v.as_str()) {
            if color_space == "bt2020nc" || color_space == "bt2020c" {
                return true;
            }
        }
    }
    
    false
}

