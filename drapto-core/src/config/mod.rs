// ============================================================================
// drapto-core/src/config/mod.rs
// ============================================================================
//
// CONFIGURATION: Core Configuration Structures and Constants
//
// This module defines the configuration structures and constants used throughout
// the drapto-core library. It provides a flexible way to configure the video
// processing behavior, including encoding parameters, quality settings, and
// analysis options.
//
// KEY COMPONENTS:
// - CoreConfig: Main configuration structure for the library
// - CoreConfigBuilder: Builder pattern for creating CoreConfig instances
// - FilmGrainMetricType: Enum for different grain analysis strategies
// - Default constants: Predefined values for common settings
//
// USAGE:
// Instances of CoreConfig are created by consumers of the library (like drapto-cli)
// and passed to the process_videos function to control encoding behavior.
//
// DESIGN PHILOSOPHY:
// The configuration is designed to be flexible and extensible, with sensible
// defaults for most parameters. Optional fields allow for fine-tuning specific
// aspects of the encoding process when needed.
//
// AI-ASSISTANT-INFO: Configuration structures and constants for the drapto-core library

// ---- Module declarations ----
mod builder;

// ---- Standard library imports ----
use std::path::PathBuf;

// ---- Internal module imports ----
use crate::processing::detection::grain_analysis::GrainLevel;

// ---- Re-exports ----
pub use builder::CoreConfigBuilder;

// ============================================================================
// DEFAULT CONSTANTS
// ============================================================================

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

/// Default knee point threshold for grain analysis (0.0-1.0).
/// This represents the point of diminishing returns in denoising strength.
pub const DEFAULT_GRAIN_KNEE_THRESHOLD: f64 = 0.8;

/// Default maximum allowed grain level for any analysis result.
pub const DEFAULT_GRAIN_MAX_LEVEL: GrainLevel = GrainLevel::Elevated;

/// Default number of refinement points to test during adaptive refinement.
pub const DEFAULT_GRAIN_REFINEMENT_POINTS: usize = 5;

// ============================================================================
// CORE CONFIGURATION
// ============================================================================

/// Main configuration structure for the drapto-core library.
///
/// This structure holds all the parameters required for video processing,
/// including paths, encoding settings, and analysis options. It is typically
/// created by the consumer of the library (e.g., drapto-cli) and passed to
/// the process_videos function.
///
/// All fields have sensible defaults, so only the required path fields need to be set.
/// The builder pattern provides a convenient way to create and configure instances.
///
/// # Examples
///
/// ```rust,no_run
/// use drapto_core::config::CoreConfigBuilder;
/// use drapto_core::processing::detection::grain_analysis::GrainLevel;
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
///     .film_grain_max_level(GrainLevel::Moderate)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct CoreConfig {
    // ---- Path Configuration ----
    /// Directory containing input video files to process
    pub input_dir: PathBuf,

    /// Directory where encoded output files will be saved
    pub output_dir: PathBuf,

    /// Directory for log files and temporary files
    pub log_dir: PathBuf,

    /// Optional directory for temporary files (defaults to output_dir)
    pub temp_dir: Option<PathBuf>,

    // ---- Encoder Settings ----
    /// Encoder preset (0-13, lower is slower/better quality)
    /// Default value is 6, which provides a good balance between speed and quality
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

    // ---- Notification Settings ----
    /// Optional ntfy.sh topic URL for sending notifications
    pub ntfy_topic: Option<String>,

    // ---- Processing Options ----
    /// Whether to enable light video denoising (hqdn3d)
    /// When true, grain analysis will be performed to determine optimal parameters
    pub enable_denoise: bool,

    // ---- Grain Analysis Configuration ----
    /// Sample duration for grain analysis in seconds
    /// Shorter samples process faster but may be less representative
    pub film_grain_sample_duration: u32,

    /// Knee point threshold for grain analysis (0.0-1.0)
    /// This represents the point of diminishing returns in denoising strength
    /// A value of 0.8 means we look for the point where we achieve 80% of the
    /// maximum possible file size reduction
    pub film_grain_knee_threshold: f64,

    /// Maximum allowed grain level for any analysis result
    /// This prevents excessive denoising even if analysis suggests it
    pub film_grain_max_level: GrainLevel,

    /// Number of refinement points to test during adaptive refinement
    /// More points provide more accurate results but increase processing time
    pub film_grain_refinement_points_count: usize,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            // Path Configuration
            input_dir: PathBuf::from("."),
            output_dir: PathBuf::from("."),
            log_dir: PathBuf::from("."),
            temp_dir: None,

            // Encoder Settings
            encoder_preset: DEFAULT_ENCODER_PRESET,
            quality_sd: DEFAULT_CORE_QUALITY_SD,
            quality_hd: DEFAULT_CORE_QUALITY_HD,
            quality_uhd: DEFAULT_CORE_QUALITY_UHD,
            crop_mode: DEFAULT_CROP_MODE.to_string(),

            // Notification Settings
            ntfy_topic: None,

            // Processing Options
            enable_denoise: true,

            // Grain Analysis Configuration
            film_grain_sample_duration: DEFAULT_GRAIN_SAMPLE_DURATION,
            film_grain_knee_threshold: DEFAULT_GRAIN_KNEE_THRESHOLD,
            film_grain_max_level: DEFAULT_GRAIN_MAX_LEVEL,
            film_grain_refinement_points_count: DEFAULT_GRAIN_REFINEMENT_POINTS,
        }
    }
}
