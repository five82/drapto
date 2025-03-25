//! Encoding configuration module
//!
//! Defines the configuration structures for video and audio encoding
//! parameters used by the encoding pipeline.

use serde::{Deserialize, Serialize};
use super::utils::*;

/// Video encoding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoEncodingConfig {
    //
    // General encoding options
    //
    
    /// Encoder preset (0-13, lower = slower/better quality)
    pub preset: u8,
    
    /// SVT-AV1 encoder parameters
    pub svt_params: String,
    
    /// Encoder name to use
    pub encoder: String,
    
    /// Keyframe interval (e.g. "10s")
    pub keyframe_interval: String,
    
    /// Pixel format (e.g. "yuv420p10le")
    pub pixel_format: String,
    
    //
    // Video processing options
    //
    
    /// Disable automatic crop detection
    pub disable_crop: bool,
    
    /// Use scene-based segmentation and parallel encoding
    pub use_segmentation: bool,
    
    //
    // Quality settings
    //
    
    /// Target VMAF score (0-100) for SDR content
    pub target_vmaf: f32,
    
    /// Target VMAF score (0-100) for HDR content
    pub target_vmaf_hdr: f32,
    
    //
    // VMAF analysis options
    //
    
    /// Number of samples to use for VMAF analysis
    pub vmaf_sample_count: u8,
    
    /// Duration of each VMAF sample in seconds
    pub vmaf_sample_duration: f32,
    
    /// VMAF analysis options
    pub vmaf_options: String,
    
    //
    // Hardware acceleration
    //
    
    /// Enable hardware acceleration for decoding if available
    pub hardware_acceleration: bool,
    
    /// Hardware acceleration options for FFmpeg
    pub hw_accel_option: String,
    
    //
    // Encoding retry options
    //
    
    /// Maximum number of retries for failed encoding
    pub max_retries: usize,
    
    /// Quality score for final retry attempt
    pub force_quality_score: f32,
}

/// Audio encoding configuration for Opus codec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioEncodingConfig {
    //
    // Opus encoder settings
    //
    
    /// Opus encoding compression level (0-10)
    pub compression_level: u32,
    
    /// Opus frame duration in milliseconds
    pub frame_duration: u32,
    
    /// Use variable bitrate
    pub vbr: bool,
    
    /// Application type (voip, audio, lowdelay)
    pub application: String,
    
    //
    // Bitrate configuration
    //
    
    /// Channel-specific bitrates override
    #[serde(default)]
    pub bitrates: AudioBitrates,
}

/// Audio bitrate configuration for different channel layouts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioBitrates {
    //
    // Standard layouts
    //
    
    /// Bitrate for mono audio (1 channel) in kbps
    pub mono: Option<u32>,

    /// Bitrate for stereo audio (2 channels) in kbps
    pub stereo: Option<u32>,
    
    //
    // Surround sound layouts
    //

    /// Bitrate for 5.1 surround (6 channels) in kbps
    pub surround_5_1: Option<u32>,

    /// Bitrate for 7.1 surround (8 channels) in kbps
    pub surround_7_1: Option<u32>,
    
    //
    // Custom layouts
    //

    /// Bitrate per channel for other configurations in kbps
    pub per_channel: Option<u32>,
}

impl Default for AudioBitrates {
    fn default() -> Self {
        Self {
            // Standard layouts
            
            // Bitrate for mono audio (1 channel) in kbps
            // Default: 64 kbps - Good quality for speech
            mono: Some(64),
            
            // Bitrate for stereo audio (2 channels) in kbps
            // Default: 128 kbps - Good quality for music
            stereo: Some(128),
            
            // Surround sound layouts
            
            // Bitrate for 5.1 surround (6 channels) in kbps
            // Default: 256 kbps - Standard for 5.1 surround
            surround_5_1: Some(256),
            
            // Bitrate for 7.1 surround (8 channels) in kbps
            // Default: 384 kbps - Standard for 7.1 surround
            surround_7_1: Some(384),
            
            // Custom layouts
            
            // Bitrate per channel for other configurations in kbps
            // Used for channel counts not covered by standard layouts
            // Default: 48 kbps per channel
            per_channel: Some(48),
        }
    }
}

impl Default for VideoEncodingConfig {
    fn default() -> Self {
        Self {
            // General encoding options
            
            // Encoder preset (0-13, lower = slower/better quality)
            // Lower values produce better quality but slower encoding
            preset: get_env_u8("DRAPTO_PRESET", 6),
            
            // SVT-AV1 encoder parameters
            // Controls various encoder-specific options as a string
            svt_params: get_env_string(
                "DRAPTO_SVT_PARAMS",
                "tune=0:enable-qm=1:enable-overlays=1:film-grain=0:film-grain-denoise=0"
                    .to_string(),
            ),
            
            // Encoder name to use
            // Determines which encoder to use with ab-av1
            encoder: get_env_string("DRAPTO_ENCODER", "libsvtav1".to_string()),
            
            // Keyframe interval (e.g. "10s")
            // Controls how frequently keyframes are inserted
            keyframe_interval: get_env_string("DRAPTO_KEYFRAME_INTERVAL", "10s".to_string()),
            
            // Pixel format (e.g. "yuv420p10le")
            // Determines color depth and chroma subsampling
            pixel_format: get_env_string("DRAPTO_PIXEL_FORMAT", "yuv420p10le".to_string()),
            
            // Video processing options
            
            // Disable automatic crop detection
            // When true, no automatic cropping will be applied
            disable_crop: get_env_bool("DRAPTO_DISABLE_CROP", false),
            
            // Use scene-based segmentation and parallel encoding
            // When true, video will be split at scene changes for parallel encoding
            use_segmentation: get_env_bool("DRAPTO_USE_SEGMENTATION", true),
            
            // Quality settings
            
            // Target VMAF score (0-100) for SDR content
            // Higher values produce better quality but larger file sizes
            target_vmaf: get_env_f32("DRAPTO_TARGET_VMAF", 93.0),
            
            // Target VMAF score (0-100) for HDR content
            // HDR content typically requires higher VMAF for similar perceptual quality
            target_vmaf_hdr: get_env_f32("DRAPTO_TARGET_VMAF_HDR", 95.0),
            
            // VMAF analysis options
            
            // Number of samples to use for VMAF analysis
            // More samples improve accuracy but increase analysis time
            vmaf_sample_count: get_env_u8("DRAPTO_VMAF_SAMPLE_COUNT", 3),
            
            // Duration of each VMAF sample in seconds
            // Longer samples may be more representative but increase analysis time
            vmaf_sample_duration: get_env_f32("DRAPTO_VMAF_SAMPLE_DURATION", 1.0),
            
            // VMAF analysis options
            // Controls model and subsample settings for VMAF
            vmaf_options: get_env_string(
                "DRAPTO_VMAF_OPTIONS",
                "n_subsample=8:pool=perc5_min".to_string(),
            ),
            
            // Hardware acceleration
            
            // Enable hardware acceleration for decoding if available
            // Can significantly improve performance on supported systems
            hardware_acceleration: get_env_bool("DRAPTO_HARDWARE_ACCELERATION", true),
            
            // Hardware acceleration options for FFmpeg
            // Used to specify the type of hardware acceleration (vaapi, nvenc, etc.)
            hw_accel_option: get_env_string("DRAPTO_HW_ACCEL_OPTION", String::new()),
            
            // Encoding retry options
            
            // Maximum number of retries for failed encoding
            // Controls how many times to retry encoding if it fails
            max_retries: get_env_usize("DRAPTO_MAX_RETRIES", 2),
            
            // Quality score for final retry attempt
            // Higher value forces better quality for last attempt
            force_quality_score: get_env_f32("DRAPTO_FORCE_QUALITY_SCORE", 95.0),
        }
    }
}

impl Default for AudioEncodingConfig {
    fn default() -> Self {
        Self {
            // Opus encoder settings
            
            // Opus encoding compression level (0-10)
            // Higher values provide better quality at the expense of encoding speed
            compression_level: get_env_u32("DRAPTO_AUDIO_COMPRESSION_LEVEL", 10),
            
            // Opus frame duration in milliseconds
            // Typical values: 2.5, 5, 10, 20, 40, 60, 80, 100, 120
            frame_duration: get_env_u32("DRAPTO_AUDIO_FRAME_DURATION", 20),
            
            // Use variable bitrate
            // VBR allows more efficient encoding by varying bitrate based on content complexity
            vbr: get_env_bool("DRAPTO_AUDIO_VBR", true),
            
            // Application type (voip, audio, lowdelay)
            // - "voip" - Optimized for voice, low delay
            // - "audio" - Optimized for music, standard delay
            // - "lowdelay" - Optimized for lowest possible delay
            application: get_env_string("DRAPTO_AUDIO_APPLICATION", "audio".to_string()),
            
            // Bitrate configuration
            
            // Channel-specific bitrates override
            // When not specified, default values are used (64k for mono, 128k for stereo, etc.)
            // Set specific bitrates based on channel count for better control
            bitrates: AudioBitrates::default(),
        }
    }
}