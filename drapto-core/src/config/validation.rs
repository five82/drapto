//! Validation configuration module
//!
//! Defines the configuration structure for validation parameters
//! including sync, duration, audio, and video validation settings.

use serde::{Deserialize, Serialize};
use super::utils::*;

/// Video validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoValidationConfig {
    //
    // Video quality validation
    //
    
    /// Minimum acceptable video bitrate in kbps
    pub min_video_bitrate: u32,
    
    /// Minimum acceptable quality score (VMAF) for validation
    pub min_quality_score: f32,
    
    //
    // Video dimension validation
    //
    
    /// Minimum acceptable video width in pixels
    pub min_width: u32,
    
    /// Minimum acceptable video height in pixels
    pub min_height: u32,
    
    //
    // Video framerate validation
    //
    
    /// Minimum acceptable video framerate
    pub min_framerate: f32,
    
    /// Maximum acceptable video framerate
    pub max_framerate: f32,
}

/// Audio validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioValidationConfig {
    //
    // Audio duration validation
    //
    
    /// Threshold below which audio streams are considered too short
    pub short_audio_threshold: f64,
    
    //
    // Audio sample rate validation
    //
    
    /// Standard acceptable audio sample rates
    pub standard_sample_rates: Vec<u32>,
    
    //
    // Audio codec validation
    //
    
    /// List of preferred audio codecs
    pub preferred_codecs: Vec<String>,
    
    /// List of acceptable audio codecs
    pub acceptable_codecs: Vec<String>,
}

/// Validation configuration for media quality and sync checking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    //
    // Audio/Video sync validation
    //
    
    /// Maximum allowed audio/video sync difference in milliseconds
    pub sync_threshold_ms: i64,
    
    //
    // Duration validation
    //
    
    /// Absolute tolerance for duration differences in seconds
    pub duration_tolerance: f64,
    
    /// Relative tolerance for duration differences as a fraction (0.0-1.0)
    pub duration_relative_tolerance: f32,
    
    //
    // Audio validation
    //
    
    /// Audio validation configuration
    #[serde(default)]
    pub audio: AudioValidationConfig,
    
    //
    // Video validation
    //
    
    /// Video validation configuration
    #[serde(default)]
    pub video: VideoValidationConfig,
}

impl Default for VideoValidationConfig {
    fn default() -> Self {
        Self {
            // Video quality validation
            
            // Minimum acceptable video bitrate in kbps
            // Below this threshold, video quality is likely to be poor
            min_video_bitrate: get_env_u32("DRAPTO_MIN_VIDEO_BITRATE", 500),
            
            // Minimum acceptable quality score (VMAF) for validation
            // Below this threshold, quality is considered unacceptable
            min_quality_score: get_env_f32("DRAPTO_MIN_QUALITY_SCORE", 80.0),
            
            // Video dimension validation
            
            // Minimum acceptable video width in pixels
            // Videos narrower than this may have scaling issues
            min_width: get_env_u32("DRAPTO_MIN_VIDEO_WIDTH", 16),
            
            // Minimum acceptable video height in pixels
            // Videos shorter than this may have scaling issues
            min_height: get_env_u32("DRAPTO_MIN_VIDEO_HEIGHT", 16),
            
            // Video framerate validation
            
            // Minimum acceptable video framerate
            // Lower framerates may cause visible judder
            min_framerate: get_env_f32("DRAPTO_MIN_FRAMERATE", 10.0),
            
            // Maximum acceptable video framerate
            // Higher framerates may indicate issues or be inefficient for encoding
            max_framerate: get_env_f32("DRAPTO_MAX_FRAMERATE", 120.0),
        }
    }
}

impl Default for AudioValidationConfig {
    fn default() -> Self {
        Self {
            // Audio duration validation
            
            // Threshold below which audio streams are considered too short (seconds)
            // Audio segments shorter than this may indicate truncation issues
            short_audio_threshold: get_env_f64("DRAPTO_SHORT_AUDIO_THRESHOLD", 0.5),
            
            // Audio sample rate validation
            
            // Standard acceptable audio sample rates
            // Most common sample rates used in digital audio systems
            // Can be overridden via DRAPTO_STANDARD_SAMPLE_RATES (comma-separated list)
            standard_sample_rates: get_env_sample_rates(
                "DRAPTO_STANDARD_SAMPLE_RATES", 
                vec![8000, 16000, 22050, 24000, 32000, 44100, 48000, 96000]
            ),
            
            // Audio codec validation
            
            // List of preferred audio codecs
            // These codecs are considered optimal for the output
            preferred_codecs: vec!["opus".to_string()],
            
            // List of acceptable audio codecs
            // These codecs are acceptable but not optimal
            acceptable_codecs: vec!["aac".to_string(), "vorbis".to_string()],
        }
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            // Audio/Video sync validation
            
            // Maximum allowed audio/video sync difference in milliseconds
            // A/V sync issues are noticeable at around 80-100ms
            sync_threshold_ms: get_env_i64("DRAPTO_SYNC_THRESHOLD_MS", 100),
            
            // Duration validation
            
            // Absolute tolerance for duration differences in seconds
            // Small differences in duration are not perceptible to viewers
            duration_tolerance: get_env_f64("DRAPTO_DURATION_TOLERANCE", 0.2),
            
            // Relative tolerance for duration differences as a fraction (0.0-1.0)
            // For longer content, we allow a bit more flexibility proportional to length
            duration_relative_tolerance: get_env_f32("DRAPTO_DURATION_RELATIVE_TOLERANCE", 0.05),
            
            // Audio validation settings
            audio: AudioValidationConfig::default(),
            
            // Video validation settings
            video: VideoValidationConfig::default(),
        }
    }
}