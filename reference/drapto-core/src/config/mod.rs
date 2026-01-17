//! Configuration structures and constants for the drapto-core library.
//!
//! This module provides the configuration system for video processing behavior,
//! including encoding parameters, quality settings, and analysis options.

use crate::error::CoreError;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

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
pub const DEFAULT_CORE_QUALITY_UHD: u8 = 29;

/// Default SVT-AV1 preset (0-13, lower is slower/better quality)
/// Value 6 provides a good balance between speed and quality.
pub const DEFAULT_SVT_AV1_PRESET: u8 = 6;

/// Default SVT-AV1 tune parameter
/// Different SVT-AV1 forks may use this value differently
pub const DEFAULT_SVT_AV1_TUNE: u8 = 0;

/// Default SVT-AV1 ac-bias parameter (controls adaptive quantization bias)
pub const DEFAULT_SVT_AV1_AC_BIAS: f32 = 0.1;

/// Default SVT-AV1 variance boost toggle (1 = enabled)
pub const DEFAULT_SVT_AV1_ENABLE_VARIANCE_BOOST: bool = false;

/// Default SVT-AV1 variance boost strength parameter
pub const DEFAULT_SVT_AV1_VARIANCE_BOOST_STRENGTH: u8 = 0;

/// Default SVT-AV1 variance octile parameter
pub const DEFAULT_SVT_AV1_VARIANCE_OCTILE: u8 = 0;

/// Default crop mode for the main encode.
pub const DEFAULT_CROP_MODE: &str = "auto";

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

/// Drapto preset groupings for encoding-related defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraptoPreset {
    /// Placeholder for future film-grain tuning; currently matches defaults.
    Grain,
    /// Settings tuned for clean, low-noise sources.
    Clean,
    /// Fast, non-archival encodes that favor turnaround time.
    Quick,
}

impl DraptoPreset {
    /// Machine-friendly identifier for this preset.
    pub const fn as_str(self) -> &'static str {
        match self {
            DraptoPreset::Grain => "grain",
            DraptoPreset::Clean => "clean",
            DraptoPreset::Quick => "quick",
        }
    }

    /// Returns the supported preset identifiers for error messages.
    pub const fn variants() -> &'static [&'static str] {
        &["grain", "clean", "quick"]
    }

    pub const fn variants_display() -> &'static str {
        "grain, clean, quick"
    }

    /// Bundled parameter defaults for this preset.
    pub const fn values(self) -> DraptoPresetValues {
        match self {
            DraptoPreset::Grain => DRAPTO_PRESET_GRAIN_VALUES,
            DraptoPreset::Clean => DRAPTO_PRESET_CLEAN_VALUES,
            DraptoPreset::Quick => DRAPTO_PRESET_QUICK_VALUES,
        }
    }
}

impl fmt::Display for DraptoPreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when parsing preset names from strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraptoPresetParseError {
    invalid_value: String,
}

impl DraptoPresetParseError {
    pub fn new<S: Into<String>>(value: S) -> Self {
        Self {
            invalid_value: value.into(),
        }
    }
}

impl fmt::Display for DraptoPresetParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Unknown Drapto preset '{}'. Valid options: {}",
            self.invalid_value,
            DraptoPreset::variants_display()
        )
    }
}

impl std::error::Error for DraptoPresetParseError {}

impl FromStr for DraptoPreset {
    type Err = DraptoPresetParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("grain") {
            Ok(DraptoPreset::Grain)
        } else if s.eq_ignore_ascii_case("clean") {
            Ok(DraptoPreset::Clean)
        } else if s.eq_ignore_ascii_case("quick") {
            Ok(DraptoPreset::Quick)
        } else {
            Err(DraptoPresetParseError::new(s))
        }
    }
}

/// Bundled parameter values for a Drapto preset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DraptoPresetValues {
    pub quality_sd: u8,
    pub quality_hd: u8,
    pub quality_uhd: u8,
    pub svt_av1_preset: u8,
    pub svt_av1_tune: u8,
    pub svt_av1_ac_bias: f32,
    pub svt_av1_enable_variance_boost: bool,
    pub svt_av1_variance_boost_strength: u8,
    pub svt_av1_variance_octile: u8,
    pub video_denoise_filter: Option<&'static str>,
    pub svt_av1_film_grain: Option<u8>,
    pub svt_av1_film_grain_denoise: Option<bool>,
}

/// Tweak these constants to customize the built-in Drapto presets.
pub const DRAPTO_PRESET_GRAIN_VALUES: DraptoPresetValues = DraptoPresetValues {
    quality_sd: DEFAULT_CORE_QUALITY_SD,
    quality_hd: DEFAULT_CORE_QUALITY_HD,
    quality_uhd: DEFAULT_CORE_QUALITY_UHD,
    svt_av1_preset: DEFAULT_SVT_AV1_PRESET,
    svt_av1_tune: DEFAULT_SVT_AV1_TUNE,
    svt_av1_ac_bias: DEFAULT_SVT_AV1_AC_BIAS,
    svt_av1_enable_variance_boost: DEFAULT_SVT_AV1_ENABLE_VARIANCE_BOOST,
    svt_av1_variance_boost_strength: DEFAULT_SVT_AV1_VARIANCE_BOOST_STRENGTH,
    svt_av1_variance_octile: DEFAULT_SVT_AV1_VARIANCE_OCTILE,
    video_denoise_filter: None,
    svt_av1_film_grain: None,
    svt_av1_film_grain_denoise: None,
};

pub const DRAPTO_PRESET_CLEAN_VALUES: DraptoPresetValues = DraptoPresetValues {
    quality_sd: 27,
    quality_hd: 29,
    quality_uhd: 31,
    svt_av1_preset: DEFAULT_SVT_AV1_PRESET,
    svt_av1_tune: DEFAULT_SVT_AV1_TUNE,
    svt_av1_ac_bias: 0.05,
    svt_av1_enable_variance_boost: false,
    svt_av1_variance_boost_strength: 0,
    svt_av1_variance_octile: 0,
    video_denoise_filter: None,
    svt_av1_film_grain: None,
    svt_av1_film_grain_denoise: None,
};

pub const DRAPTO_PRESET_QUICK_VALUES: DraptoPresetValues = DraptoPresetValues {
    quality_sd: 32,
    quality_hd: 35,
    quality_uhd: 36,
    svt_av1_preset: 8,
    svt_av1_tune: DEFAULT_SVT_AV1_TUNE,
    svt_av1_ac_bias: 0.0,
    svt_av1_enable_variance_boost: false,
    svt_av1_variance_boost_strength: 0,
    svt_av1_variance_octile: 0,
    video_denoise_filter: None,
    svt_av1_film_grain: None,
    svt_av1_film_grain_denoise: None,
};

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

    /// SVT-AV1 ac-bias parameter
    pub svt_av1_ac_bias: f32,

    /// Whether to enable variance boost in SVT-AV1
    pub svt_av1_enable_variance_boost: bool,

    /// SVT-AV1 variance boost strength parameter
    pub svt_av1_variance_boost_strength: u8,

    /// SVT-AV1 variance octile parameter
    pub svt_av1_variance_octile: u8,

    /// Optional denoise filter applied via `-vf` (e.g., `hqdn3d=1.5:1.5:3:3`).
    pub video_denoise_filter: Option<String>,

    /// Optional SVT-AV1 film grain synthesis strength (passed via `-svtav1-params film-grain=...`).
    pub svt_av1_film_grain: Option<u8>,

    /// Optional SVT-AV1 film grain denoise toggle (`false` -> `0`, `true` -> `1`).
    pub svt_av1_film_grain_denoise: Option<bool>,

    /// CRF quality for Standard Definition videos (<1920 width)
    /// Lower values produce higher quality but larger files
    pub quality_sd: u8,

    /// CRF quality for High Definition videos (>=1920 width, <3840 width)
    pub quality_hd: u8,

    /// CRF quality for Ultra High Definition videos (>=3840 width)
    pub quality_uhd: u8,

    /// Crop mode for the main encode ("auto", "none", etc.)
    pub crop_mode: String,

    /// Whether to reserve CPU threads for improved system responsiveness.
    pub responsive_encoding: bool,

    /// Cooldown period in seconds between encodes when processing multiple files.
    /// Helps ensure notifications arrive in order.
    pub encode_cooldown_secs: u64,

    /// Selected Drapto preset controlling grouped defaults.
    pub drapto_preset: Option<DraptoPreset>,
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
            svt_av1_ac_bias: DEFAULT_SVT_AV1_AC_BIAS,
            svt_av1_enable_variance_boost: DEFAULT_SVT_AV1_ENABLE_VARIANCE_BOOST,
            svt_av1_variance_boost_strength: DEFAULT_SVT_AV1_VARIANCE_BOOST_STRENGTH,
            svt_av1_variance_octile: DEFAULT_SVT_AV1_VARIANCE_OCTILE,
            video_denoise_filter: None,
            svt_av1_film_grain: None,
            svt_av1_film_grain_denoise: None,
            quality_sd: DEFAULT_CORE_QUALITY_SD,
            quality_hd: DEFAULT_CORE_QUALITY_HD,
            quality_uhd: DEFAULT_CORE_QUALITY_UHD,
            crop_mode: DEFAULT_CROP_MODE.to_string(),
            responsive_encoding: false,
            encode_cooldown_secs: DEFAULT_ENCODE_COOLDOWN_SECS,
            drapto_preset: None,
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

    /// Applies the provided Drapto preset to the config, overwriting the grouped values.
    pub fn apply_drapto_preset(&mut self, preset: DraptoPreset) {
        let values = preset.values();
        self.drapto_preset = Some(preset);
        self.quality_sd = values.quality_sd;
        self.quality_hd = values.quality_hd;
        self.quality_uhd = values.quality_uhd;
        self.svt_av1_preset = values.svt_av1_preset;
        self.svt_av1_tune = values.svt_av1_tune;
        self.svt_av1_ac_bias = values.svt_av1_ac_bias;
        self.svt_av1_enable_variance_boost = values.svt_av1_enable_variance_boost;
        self.svt_av1_variance_boost_strength = values.svt_av1_variance_boost_strength;
        self.svt_av1_variance_octile = values.svt_av1_variance_octile;
        self.video_denoise_filter = values.video_denoise_filter.map(str::to_string);
        self.svt_av1_film_grain = values.svt_av1_film_grain;
        self.svt_av1_film_grain_denoise = values.svt_av1_film_grain_denoise;
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

        if self.svt_av1_film_grain.is_none() && self.svt_av1_film_grain_denoise.is_some() {
            return Err(CoreError::Config(
                "svt_av1_film_grain_denoise set without svt_av1_film_grain".to_string(),
            ));
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
        assert_eq!(config.svt_av1_ac_bias, DEFAULT_SVT_AV1_AC_BIAS);
        assert_eq!(
            config.svt_av1_enable_variance_boost,
            DEFAULT_SVT_AV1_ENABLE_VARIANCE_BOOST
        );
        assert_eq!(
            config.svt_av1_variance_boost_strength,
            DEFAULT_SVT_AV1_VARIANCE_BOOST_STRENGTH
        );
        assert_eq!(
            config.svt_av1_variance_octile,
            DEFAULT_SVT_AV1_VARIANCE_OCTILE
        );
        assert!(config.video_denoise_filter.is_none());
        assert!(config.svt_av1_film_grain.is_none());
        assert!(config.svt_av1_film_grain_denoise.is_none());
        assert!(!config.responsive_encoding);
        assert!(config.temp_dir.is_none());
        assert!(config.drapto_preset.is_none());

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
    fn applying_drapto_preset_resets_grouped_fields() {
        let mut config = CoreConfig::default();
        config.quality_sd = 1;
        config.svt_av1_preset = 13;

        config.apply_drapto_preset(DraptoPreset::Grain);

        assert_eq!(config.drapto_preset, Some(DraptoPreset::Grain));
        assert_eq!(config.quality_sd, DRAPTO_PRESET_GRAIN_VALUES.quality_sd);
        assert_eq!(
            config.svt_av1_preset,
            DRAPTO_PRESET_GRAIN_VALUES.svt_av1_preset
        );
        assert_eq!(config.svt_av1_tune, DRAPTO_PRESET_GRAIN_VALUES.svt_av1_tune);
        assert_eq!(
            config.svt_av1_ac_bias,
            DRAPTO_PRESET_GRAIN_VALUES.svt_av1_ac_bias
        );
        assert_eq!(
            config.svt_av1_enable_variance_boost,
            DRAPTO_PRESET_GRAIN_VALUES.svt_av1_enable_variance_boost
        );
        assert_eq!(
            config.svt_av1_variance_boost_strength,
            DRAPTO_PRESET_GRAIN_VALUES.svt_av1_variance_boost_strength
        );
        assert_eq!(
            config.svt_av1_variance_octile,
            DRAPTO_PRESET_GRAIN_VALUES.svt_av1_variance_octile
        );
        assert_eq!(
            config.video_denoise_filter.as_deref(),
            DRAPTO_PRESET_GRAIN_VALUES.video_denoise_filter
        );
        assert_eq!(
            config.svt_av1_film_grain,
            DRAPTO_PRESET_GRAIN_VALUES.svt_av1_film_grain
        );
        assert_eq!(
            config.svt_av1_film_grain_denoise,
            DRAPTO_PRESET_GRAIN_VALUES.svt_av1_film_grain_denoise
        );
    }

    #[test]
    fn drapto_preset_from_str_is_case_insensitive() {
        assert_eq!(
            "GRAIN".parse::<DraptoPreset>().unwrap(),
            DraptoPreset::Grain
        );
        assert_eq!(
            "clean".parse::<DraptoPreset>().unwrap(),
            DraptoPreset::Clean
        );
        assert_eq!(
            "Quick".parse::<DraptoPreset>().unwrap(),
            DraptoPreset::Quick
        );
        assert!("unknown".parse::<DraptoPreset>().is_err());
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
        assert!(HD_WIDTH_THRESHOLD < UHD_WIDTH_THRESHOLD);
        assert!(DEFAULT_ENCODE_COOLDOWN_SECS > 0);
    }
}
