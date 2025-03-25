//! Detection configuration module
//!
//! Defines the configuration structures for scene detection,
//! format detection, and crop detection parameters.

use serde::{Deserialize, Serialize};
use super::utils::*;

/// Scene detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDetectionConfig {
    //
    // Threshold settings
    //
    
    /// Scene detection threshold for SDR content (0-100)
    pub scene_threshold: f32,

    /// Scene detection threshold for HDR content (0-100)
    pub hdr_scene_threshold: f32,
    
    /// Scene validation tolerance in seconds
    /// Used when validating if segment boundaries align with scenes
    pub scene_tolerance: f32,
    
    //
    // Segment length constraints
    //

    /// Minimum segment length in seconds
    pub min_segment_length: f32,

    /// Maximum segment length in seconds
    pub max_segment_length: f32,
}

/// Crop detection configuration for black bar detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CropDetectionConfig {
    //
    // Threshold settings
    //
    
    /// Base crop detection threshold for SDR content
    pub sdr_threshold: i32,
    
    /// Base crop detection threshold for HDR content
    pub hdr_threshold: i32,
    
    /// Multiplier applied to analyzed black levels in HDR content
    pub hdr_black_level_multiplier: f32,
    
    /// Minimum allowed crop threshold
    pub min_threshold: i32,
    
    /// Maximum allowed crop threshold
    pub max_threshold: i32,
    
    //
    // Detection sensitivity
    //
    
    /// Minimum percentage of height that black bars must occupy to be cropped
    pub min_black_bar_percent: u32,
    
    /// Minimum height in pixels for a cropped frame to be considered valid
    pub min_height: u32,
    
    //
    // Sampling parameters
    //
    
    /// Sampling interval in seconds between analyzed frames
    pub sampling_interval: f32,
    
    /// Minimum number of samples to analyze regardless of duration
    pub min_sample_count: i32,
    
    /// Frame selection pattern for ffmpeg select filter
    pub frame_selection: String,
    
    //
    // Credits skip parameters
    //
    
    /// Skip duration for movies (content > 1 hour)
    pub credits_skip_movie: f64,
    
    /// Skip duration for TV episodes (content > 20 minutes)
    pub credits_skip_episode: f64,
    
    /// Skip duration for short content (content > 5 minutes)
    pub credits_skip_short: f64,
}

impl Default for SceneDetectionConfig {
    fn default() -> Self {
        Self {
            // Threshold settings
            
            // Scene detection threshold for SDR content (0-100)
            // Higher values result in fewer scene changes detected
            // Lower values are more sensitive to small changes
            scene_threshold: get_env_f32("DRAPTO_SCENE_THRESHOLD", 40.0),
            
            // Scene detection threshold for HDR content (0-100)
            // HDR content typically needs a lower threshold due to higher dynamic range
            hdr_scene_threshold: get_env_f32("DRAPTO_HDR_SCENE_THRESHOLD", 30.0),
            
            // Scene validation tolerance in seconds
            // Controls the allowed time difference when validating scene boundaries
            // Smaller values are more strict, requiring more precise alignment
            scene_tolerance: get_env_f32("DRAPTO_SCENE_TOLERANCE", 0.5),
            
            // Segment length constraints
            
            // Minimum segment length in seconds
            // Prevents creating segments that are too short, which could be inefficient
            min_segment_length: get_env_f32("DRAPTO_MIN_SEGMENT_LENGTH", 5.0),
            
            // Maximum segment length in seconds
            // Prevents creating segments that are too long, ensuring parallelism
            max_segment_length: get_env_f32("DRAPTO_MAX_SEGMENT_LENGTH", 15.0),
        }
    }
}

impl Default for CropDetectionConfig {
    fn default() -> Self {
        Self {
            // Threshold settings
            
            // Base crop detection threshold for SDR content
            // Low values are more sensitive to detecting blacks
            sdr_threshold: get_env_i32("DRAPTO_CROP_SDR_THRESHOLD", 16),
            
            // Base crop detection threshold for HDR content
            // Higher due to elevated black levels in HDR content
            hdr_threshold: get_env_i32("DRAPTO_CROP_HDR_THRESHOLD", 128),
            
            // Multiplier applied to analyzed black levels in HDR content
            // Compensates for varied HDR mastering approaches
            hdr_black_level_multiplier: get_env_f32("DRAPTO_HDR_BLACK_MULTIPLIER", 1.5),
            
            // Minimum allowed crop threshold
            // Prevents setting extremely low thresholds causing false positives
            min_threshold: get_env_i32("DRAPTO_CROP_MIN_THRESHOLD", 16),
            
            // Maximum allowed crop threshold
            // Prevents setting thresholds too high that miss actual black bars
            max_threshold: get_env_i32("DRAPTO_CROP_MAX_THRESHOLD", 256),
            
            // Detection sensitivity
            
            // Minimum percentage of height that black bars must occupy to be cropped
            // Prevents cropping small letterboxing or artifacts
            min_black_bar_percent: get_env_u32("DRAPTO_MIN_BLACK_BAR_PERCENT", 1),
            
            // Minimum height in pixels for a cropped frame
            // Prevents excessive cropping that removes too much height
            min_height: get_env_u32("DRAPTO_CROP_MIN_HEIGHT", 100),
            
            // Sampling parameters
            
            // Sampling interval in seconds between analyzed frames
            // Controls how frequently to take frame samples for analysis
            sampling_interval: get_env_f32("DRAPTO_CROP_SAMPLING_INTERVAL", 5.0),
            
            // Minimum number of samples to analyze regardless of duration
            // Ensures enough data points are collected for accurate detection
            min_sample_count: get_env_i32("DRAPTO_CROP_MIN_SAMPLES", 20),
            
            // Frame selection pattern for ffmpeg select filter
            // Defines which frames to select for analysis
            frame_selection: get_env_string(
                "DRAPTO_CROP_FRAME_SELECTION", 
                "not(mod(n,30))".to_string()
            ),
            
            // Credits skip parameters
            
            // Skip duration for movies (content > 1 hour)
            // Avoids analyzing ending credits in longer content
            credits_skip_movie: get_env_f64("DRAPTO_CREDITS_SKIP_MOVIE", 180.0),  // 3 minutes
            
            // Skip duration for TV episodes (content > 20 minutes)
            // Avoids analyzing ending credits in medium-length content
            credits_skip_episode: get_env_f64("DRAPTO_CREDITS_SKIP_EPISODE", 60.0), // 1 minute
            
            // Skip duration for short content (content > 5 minutes)
            // Avoids analyzing ending credits in shorter content
            credits_skip_short: get_env_f64("DRAPTO_CREDITS_SKIP_SHORT", 30.0),   // 30 seconds
        }
    }
}