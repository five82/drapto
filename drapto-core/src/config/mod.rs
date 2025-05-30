//! Configuration structures and constants for the drapto-core library.
//! 
//! This module provides the configuration system for video processing behavior,
//! including encoding parameters, quality settings, and analysis options.

mod builder;

use std::path::PathBuf;
use crate::processing::grain_types::GrainLevel;

pub use builder::CoreConfigBuilder;

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

/// Default sample duration for grain analysis in seconds.
pub const DEFAULT_GRAIN_SAMPLE_DURATION: u32 = 10;

/// Default maximum allowed grain level for any analysis result.
pub const DEFAULT_GRAIN_MAX_LEVEL: GrainLevel = GrainLevel::Elevated;

/// Default XPSNR threshold for grain analysis in decibels.
/// This represents the Just Noticeable Difference (JND) threshold.
pub const DEFAULT_XPSNR_THRESHOLD: f64 = 1.2;

/// Default minimum number of samples for grain analysis.
pub const DEFAULT_GRAIN_MIN_SAMPLES: usize = 3;

/// Default maximum number of samples for grain analysis.
pub const DEFAULT_GRAIN_MAX_SAMPLES: usize = 7;

/// Default target seconds per sample for calculating sample count.
pub const DEFAULT_GRAIN_SECS_PER_SAMPLE: f64 = 1200.0; // 20 minutes

/// Default start boundary for sample extraction (as fraction of video duration).
pub const DEFAULT_GRAIN_SAMPLE_START_BOUNDARY: f64 = 0.15; // 15%

/// Default end boundary for sample extraction (as fraction of video duration).
pub const DEFAULT_GRAIN_SAMPLE_END_BOUNDARY: f64 = 0.85; // 85%


/// Main configuration structure for the drapto-core library.
///
/// This structure holds all the parameters required for video processing,
/// including paths, encoding settings, and analysis options. It is typically
/// created by the consumer of the library (e.g., drapto-cli) and passed to
/// the `process_videos` function.
///
/// All fields have sensible defaults, so only the required path fields need to be set.
/// The builder pattern provides a convenient way to create and configure instances.
///
/// # Examples
///
/// ```rust,no_run
/// use drapto_core::config::CoreConfigBuilder;
/// use drapto_core::processing::grain_types::GrainLevel;
/// use std::path::PathBuf;
///
/// let config = CoreConfigBuilder::new()
///     .input_dir(PathBuf::from("/path/to/input"))
///     .output_dir(PathBuf::from("/path/to/output"))
///     .log_dir(PathBuf::from("/path/to/logs"))
///     .enable_denoise(true)
///     .encoder_preset(6)
///     .quality_sd(24)
///     .quality_hd(26)
///     .quality_uhd(28)
///     .crop_mode("auto")
///     .ntfy_topic("https://ntfy.sh/my-topic")
///     .film_grain_sample_duration(10)
///     .film_grain_max_level(GrainLevel::Moderate)
///     .build();
/// ```
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
    /// When true, grain analysis will be performed to determine optimal parameters
    pub enable_denoise: bool,

    /// Sample duration for grain analysis in seconds
    /// Shorter samples process faster but may be less representative
    pub film_grain_sample_duration: u32,

    /// Maximum allowed grain level for any analysis result
    /// This prevents excessive denoising even if analysis suggests it
    pub film_grain_max_level: GrainLevel,

    /// XPSNR threshold for grain analysis in decibels
    /// Represents the Just Noticeable Difference (JND) threshold
    pub xpsnr_threshold: f64,

    /// Minimum number of samples for grain analysis
    pub grain_min_samples: usize,

    /// Maximum number of samples for grain analysis
    pub grain_max_samples: usize,

    /// Target seconds per sample for calculating sample count
    pub grain_secs_per_sample: f64,

    /// Start boundary for sample extraction (fraction of video duration)
    pub grain_sample_start_boundary: f64,

    /// End boundary for sample extraction (fraction of video duration)
    pub grain_sample_end_boundary: f64,
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
            film_grain_sample_duration: DEFAULT_GRAIN_SAMPLE_DURATION,
            film_grain_max_level: DEFAULT_GRAIN_MAX_LEVEL,
            xpsnr_threshold: DEFAULT_XPSNR_THRESHOLD,
            grain_min_samples: DEFAULT_GRAIN_MIN_SAMPLES,
            grain_max_samples: DEFAULT_GRAIN_MAX_SAMPLES,
            grain_secs_per_sample: DEFAULT_GRAIN_SECS_PER_SAMPLE,
            grain_sample_start_boundary: DEFAULT_GRAIN_SAMPLE_START_BOUNDARY,
            grain_sample_end_boundary: DEFAULT_GRAIN_SAMPLE_END_BOUNDARY,
        }
    }
}
