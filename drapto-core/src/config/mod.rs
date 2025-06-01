//! Configuration structures and constants for the drapto-core library.
//! 
//! This module provides the configuration system for video processing behavior,
//! including encoding parameters, quality settings, and analysis options.

use std::path::PathBuf;
use crate::error::CoreError;

// Default constants

/// Default CRF (Constant Rate Factor) quality value for Standard Definition videos (<1920 width).
/// Lower values produce higher quality but larger files.
/// Range: 0-63, with 0 being lossless.
pub const DEFAULT_CORE_QUALITY_SD: u8 = 25;

/// Default CRF quality value for High Definition videos (>=1920 width, <3840 width).
/// Higher than SD to maintain reasonable file sizes for HD content.
pub const DEFAULT_CORE_QUALITY_HD: u8 = 27;

/// Default CRF quality value for Ultra High Definition videos (>=3840 width).
/// Same as HD by default, but can be overridden separately.
pub const DEFAULT_CORE_QUALITY_UHD: u8 = 27;

/// Default encoder preset (0-13, lower is slower/better quality)
/// Value 6 provides a good balance between speed and quality.
pub const DEFAULT_ENCODER_PRESET: u8 = 6;

/// Default crop mode for the main encode.
pub const DEFAULT_CROP_MODE: &str = "auto";

/// Fixed denoising parameters for hqdn3d filter.
/// Format: spatial_luma:spatial_chroma:temporal_luma:temporal_chroma
pub const FIXED_HQDN3D_PARAMS: &str = "0.5:0.4:2:2";

/// Fixed film grain synthesis value for SVT-AV1.
/// Range: 0-50, where 0 is no grain and 50 is maximum grain.
pub const FIXED_FILM_GRAIN_VALUE: u8 = 4;

/// Width threshold for Ultra High Definition (4K) videos.
/// Videos with width >= this value are considered UHD.
pub const UHD_WIDTH_THRESHOLD: u32 = 3840;

/// Width threshold for High Definition videos.
/// Videos with width >= this value (but < UHD threshold) are considered HD.
pub const HD_WIDTH_THRESHOLD: u32 = 1920;

/// HDR color spaces used for HDR detection.
/// Videos with these color spaces are considered HDR.
pub const HDR_COLOR_SPACES: &[&str] = &["bt2020nc", "bt2020c"];



/// Configuration for video processing including paths and encoding settings.
#[derive(Debug, Clone)]
pub struct CoreConfig {
    /// Directory containing input video files to process
    pub input_dir: PathBuf,

    /// Directory where encoded output files will be saved
    pub output_dir: PathBuf,

    /// Directory for log files and temporary files
    pub log_dir: PathBuf,

    /// Optional directory for temporary files (defaults to `output_dir`)
    pub temp_dir: Option<PathBuf>,

    /// Encoder preset (0-13, lower is slower/better quality)
    /// Default: 6 for balanced speed/quality
    pub encoder_preset: u8,

    /// CRF quality for Standard Definition videos (<1920 width)
    /// Lower values produce higher quality but larger files
    pub quality_sd: u8,

    /// CRF quality for High Definition videos (>=1920 width, <3840 width)
    pub quality_hd: u8,

    /// CRF quality for Ultra High Definition videos (>=3840 width)
    pub quality_uhd: u8,

    /// Crop mode for the main encode ("auto", "none", etc.)
    pub crop_mode: String,

    /// Optional ntfy.sh topic URL for sending notifications
    pub ntfy_topic: Option<String>,

    /// Whether to enable light video denoising (hqdn3d)
    /// When true, applies fixed VeryLight denoising with film grain synthesis
    pub enable_denoise: bool,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            input_dir: PathBuf::from("."),
            output_dir: PathBuf::from("."),
            log_dir: PathBuf::from("."),
            temp_dir: None,
            encoder_preset: DEFAULT_ENCODER_PRESET,
            quality_sd: DEFAULT_CORE_QUALITY_SD,
            quality_hd: DEFAULT_CORE_QUALITY_HD,
            quality_uhd: DEFAULT_CORE_QUALITY_UHD,
            crop_mode: DEFAULT_CROP_MODE.to_string(),
            ntfy_topic: None,
            enable_denoise: true,
        }
    }
}

impl CoreConfig {
    /// Creates config with required paths. Other fields use defaults.
    pub fn new(input_dir: PathBuf, output_dir: PathBuf, log_dir: PathBuf) -> Self {
        Self {
            input_dir,
            output_dir,
            log_dir,
            ..Default::default()
        }
    }
    
    /// Validates encoder_preset (0-13) and quality values (0-63).
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.encoder_preset > 13 {
            return Err(CoreError::Config(
                format!("encoder_preset must be 0-13, got {}", self.encoder_preset)
            ));
        }
        
        if self.quality_sd > 63 {
            return Err(CoreError::Config(
                format!("quality_sd must be 0-63, got {}", self.quality_sd)
            ));
        }
        
        if self.quality_hd > 63 {
            return Err(CoreError::Config(
                format!("quality_hd must be 0-63, got {}", self.quality_hd)
            ));
        }
        
        if self.quality_uhd > 63 {
            return Err(CoreError::Config(
                format!("quality_uhd must be 0-63, got {}", self.quality_uhd)
            ));
        }
        
        Ok(())
    }
}
