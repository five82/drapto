//! Configuration structures and constants for the drapto-core library.
//!
//! This module provides the configuration system for video processing behavior,
//! including encoding parameters, quality settings, and analysis options.

use crate::error::CoreError;
use std::path::PathBuf;

// Default constants

/// Default CRF (Constant Rate Factor) quality value for Standard Definition videos (<1920 width).
/// Lower values produce higher quality but larger files.
/// Range: 0-63, with 0 being lossless.
pub const DEFAULT_CORE_QUALITY_SD: u8 = 23;

/// Default CRF quality value for High Definition videos (>=1920 width, <3840 width).
/// Higher than SD to maintain reasonable file sizes for HD content.
pub const DEFAULT_CORE_QUALITY_HD: u8 = 25;

/// Default CRF quality value for Ultra High Definition videos (>=3840 width).
/// Same as HD by default, but can be overridden separately.
pub const DEFAULT_CORE_QUALITY_UHD: u8 = 27;

/// Default SVT-AV1 preset (0-13, lower is slower/better quality)
/// Value 6 provides a good balance between speed and quality.
pub const DEFAULT_SVT_AV1_PRESET: u8 = 4;

/// Default SVT-AV1 tune parameter
/// Different SVT-AV1 forks may use this value differently
pub const DEFAULT_SVT_AV1_TUNE: u8 = 0;

/// Default crop mode for the main encode.
pub const DEFAULT_CROP_MODE: &str = "auto";

/// Minimum film grain synthesis value for SVT-AV1.
/// Range: 0-50, where 0 is no grain and 50 is maximum grain.
pub const MIN_FILM_GRAIN_VALUE: u8 = 4;

/// Maximum film grain synthesis value for SVT-AV1.
/// Used for high noise content with strong denoising.
pub const MAX_FILM_GRAIN_VALUE: u8 = 16;

/// Width threshold for Ultra High Definition (4K) videos.
/// Videos with width >= this value are considered UHD.
pub const UHD_WIDTH_THRESHOLD: u32 = 3840;

/// Width threshold for High Definition videos.
/// Videos with width >= this value (but < UHD threshold) are considered HD.
pub const HD_WIDTH_THRESHOLD: u32 = 1920;

/// Default cooldown period between encodes in seconds.
/// This helps ensure notifications arrive in order when processing multiple files.
pub const DEFAULT_ENCODE_COOLDOWN_SECS: u64 = 3;

/// Progress logging interval in percent.
/// Progress will be logged to file at this percentage interval (e.g., 5 = every 5%).
pub const PROGRESS_LOG_INTERVAL_PERCENT: u8 = 5;

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

    /// SVT-AV1 preset (0-13, lower is slower/better quality)
    /// Default: 6 for balanced speed/quality
    pub svt_av1_preset: u8,

    /// SVT-AV1 tune parameter
    /// Different SVT-AV1 forks may use this value differently
    pub svt_av1_tune: u8,

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

    /// Whether to enable adaptive video denoising (hqdn3d)
    /// When true, analyzes video noise and applies appropriate denoising with film grain synthesis
    pub enable_denoise: bool,

    /// Whether to reserve CPU threads for improved system responsiveness.
    pub responsive_encoding: bool,

    /// Cooldown period in seconds between encodes when processing multiple files.
    /// Helps ensure notifications arrive in order.
    pub encode_cooldown_secs: u64,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            input_dir: PathBuf::from("."),
            output_dir: PathBuf::from("."),
            log_dir: PathBuf::from("."),
            temp_dir: None,
            svt_av1_preset: DEFAULT_SVT_AV1_PRESET,
            svt_av1_tune: DEFAULT_SVT_AV1_TUNE,
            quality_sd: DEFAULT_CORE_QUALITY_SD,
            quality_hd: DEFAULT_CORE_QUALITY_HD,
            quality_uhd: DEFAULT_CORE_QUALITY_UHD,
            crop_mode: DEFAULT_CROP_MODE.to_string(),
            ntfy_topic: None,
            enable_denoise: true,
            responsive_encoding: false,
            encode_cooldown_secs: DEFAULT_ENCODE_COOLDOWN_SECS,
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

    /// Validates svt_av1_preset (0-13) and quality values (0-63).
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.svt_av1_preset > 13 {
            return Err(CoreError::Config(format!(
                "svt_av1_preset must be 0-13, got {}",
                self.svt_av1_preset
            )));
        }

        if self.quality_sd > 63 {
            return Err(CoreError::Config(format!(
                "quality_sd must be 0-63, got {}",
                self.quality_sd
            )));
        }

        if self.quality_hd > 63 {
            return Err(CoreError::Config(format!(
                "quality_hd must be 0-63, got {}",
                self.quality_hd
            )));
        }

        if self.quality_uhd > 63 {
            return Err(CoreError::Config(format!(
                "quality_uhd must be 0-63, got {}",
                self.quality_uhd
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CoreConfig::default();

        // Check default values
        assert_eq!(config.svt_av1_preset, DEFAULT_SVT_AV1_PRESET);
        assert_eq!(config.quality_sd, DEFAULT_CORE_QUALITY_SD);
        assert_eq!(config.quality_hd, DEFAULT_CORE_QUALITY_HD);
        assert_eq!(config.quality_uhd, DEFAULT_CORE_QUALITY_UHD);
        assert_eq!(config.crop_mode, DEFAULT_CROP_MODE);
        assert_eq!(config.encode_cooldown_secs, DEFAULT_ENCODE_COOLDOWN_SECS);
        assert!(config.enable_denoise);
        assert!(!config.responsive_encoding);
        assert!(config.ntfy_topic.is_none());
        assert!(config.temp_dir.is_none());

        // Validate default config should pass
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_new_config() {
        let input = PathBuf::from("/input");
        let output = PathBuf::from("/output");
        let log = PathBuf::from("/log");

        let config = CoreConfig::new(input.clone(), output.clone(), log.clone());

        // Check paths are set correctly
        assert_eq!(config.input_dir, input);
        assert_eq!(config.output_dir, output);
        assert_eq!(config.log_dir, log);

        // Check other fields use defaults
        assert_eq!(config.svt_av1_preset, DEFAULT_SVT_AV1_PRESET);
        assert_eq!(config.quality_sd, DEFAULT_CORE_QUALITY_SD);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_svt_av1_preset() {
        let mut config = CoreConfig::default();

        // Valid presets
        for preset in 0..=13 {
            config.svt_av1_preset = preset;
            assert!(config.validate().is_ok());
        }

        // Invalid presets
        config.svt_av1_preset = 14;
        assert!(config.validate().is_err());

        config.svt_av1_preset = 255;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_quality_values() {
        let mut config = CoreConfig::default();

        // Valid quality values
        for quality in 0..=63 {
            config.quality_sd = quality;
            config.quality_hd = quality;
            config.quality_uhd = quality;
            assert!(config.validate().is_ok());
        }

        // Invalid SD quality
        config = CoreConfig::default();
        config.quality_sd = 64;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("quality_sd"));

        // Invalid HD quality
        config = CoreConfig::default();
        config.quality_hd = 64;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("quality_hd"));

        // Invalid UHD quality
        config = CoreConfig::default();
        config.quality_uhd = 64;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("quality_uhd"));

        // Multiple invalid values (should fail on first)
        config = CoreConfig::default();
        config.svt_av1_preset = 14;
        config.quality_sd = 64;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("svt_av1_preset"));
    }

    #[test]
    fn test_constants() {
        // Verify constants are reasonable values
        assert!(DEFAULT_CORE_QUALITY_SD <= 63);
        assert!(DEFAULT_CORE_QUALITY_HD <= 63);
        assert!(DEFAULT_CORE_QUALITY_UHD <= 63);
        assert!(DEFAULT_SVT_AV1_PRESET <= 13);
        assert!(MIN_FILM_GRAIN_VALUE <= 50);
        assert!(MAX_FILM_GRAIN_VALUE <= 50);
        assert!(MIN_FILM_GRAIN_VALUE < MAX_FILM_GRAIN_VALUE);
        assert!(HD_WIDTH_THRESHOLD < UHD_WIDTH_THRESHOLD);
        assert!(DEFAULT_ENCODE_COOLDOWN_SECS > 0);
    }
}
