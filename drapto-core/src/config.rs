// ============================================================================
// drapto-core/src/config.rs
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

// ---- Standard library imports ----
use std::path::PathBuf;

// ---- Internal module imports ----
use crate::processing::detection::grain_analysis::GrainLevel;

// ============================================================================
// FILM GRAIN ANALYSIS TYPES
// ============================================================================

/// Strategy for determining the optimal film grain value during analysis.
///
/// This enum defines different algorithms that can be used to analyze
/// film grain in a video and determine the optimal denoising parameters.
///
/// # Variants
///
/// * `KneePoint` - Uses knee point detection to find the optimal balance
///   between file size reduction and visual quality preservation.
///
/// # Examples
///
/// ```rust,no_run
/// use drapto_core::config::FilmGrainMetricType;
///
/// // Configure grain analysis to use knee point detection
/// let metric_type = FilmGrainMetricType::KneePoint;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilmGrainMetricType {
    /// Knee point detection algorithm that finds the point of diminishing returns
    /// in the denoising curve, balancing file size reduction and quality preservation.
    KneePoint,

    // Future strategies (currently disabled):
    // PercentMaxReduction - Uses a percentage of maximum reduction
    // OriginalEfficiency - Uses the original efficiency algorithm
}

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
/// # Examples
///
/// ```rust,no_run
/// use drapto_core::CoreConfig;
/// use drapto_core::processing::detection::GrainLevel;
/// use std::path::PathBuf;
///
/// let config = CoreConfig {
///     input_dir: PathBuf::from("/path/to/input"),
///     output_dir: PathBuf::from("/path/to/output"),
///     log_dir: PathBuf::from("/path/to/logs"),
///     default_encoder_preset: Some(6),
///     preset: None,
///     quality_sd: Some(24),
///     quality_hd: Some(26),
///     quality_uhd: Some(28),
///     default_crop_mode: Some("auto".to_string()),
///     ntfy_topic: Some("https://ntfy.sh/my-topic".to_string()),
///     enable_denoise: true,
///     film_grain_sample_duration: Some(5),
///     film_grain_knee_threshold: Some(0.8),
///     film_grain_fallback_level: Some(GrainLevel::Baseline),
///     film_grain_max_level: Some(GrainLevel::Moderate),
///     film_grain_refinement_points_count: Some(5),
/// };
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

    // ---- Encoder Settings ----

    /// Default encoder preset (0-13, lower is slower/better quality)
    /// This is the primary way to set the default preset
    pub default_encoder_preset: Option<u8>,

    /// Override for encoder preset (takes precedence over default_encoder_preset)
    pub preset: Option<u8>,

    /// CRF quality for Standard Definition videos (<1920 width)
    /// Lower values produce higher quality but larger files
    pub quality_sd: Option<u8>,

    /// CRF quality for High Definition videos (>=1920 width, <3840 width)
    pub quality_hd: Option<u8>,

    /// CRF quality for Ultra High Definition videos (>=3840 width)
    pub quality_uhd: Option<u8>,

    /// Crop mode for the main encode ("auto", "none", etc.)
    pub default_crop_mode: Option<String>,

    // ---- Notification Settings ----

    /// Optional ntfy.sh topic URL for sending notifications
    pub ntfy_topic: Option<String>,

    // ---- Processing Options ----

    /// Whether to enable light video denoising (hqdn3d)
    /// When true, grain analysis will be performed to determine optimal parameters
    pub enable_denoise: bool,

    // Note: Hardware acceleration field was removed as it's no longer supported

    // ---- Grain Analysis Configuration ----

    /// Sample duration for grain analysis in seconds
    /// Shorter samples process faster but may be less representative
    pub film_grain_sample_duration: Option<u32>,

    /// Knee point threshold for grain analysis (0.0-1.0)
    /// This represents the point of diminishing returns in denoising strength
    /// A value of 0.8 means we look for the point where we achieve 80% of the
    /// maximum possible file size reduction
    pub film_grain_knee_threshold: Option<f64>,

    /// Fallback grain level if analysis fails
    /// This is used when grain analysis cannot be performed or fails
    pub film_grain_fallback_level: Option<GrainLevel>,

    /// Maximum allowed grain level for any analysis result
    /// This prevents excessive denoising even if analysis suggests it
    pub film_grain_max_level: Option<GrainLevel>,

    /// Number of refinement points to test during adaptive refinement
    /// More points provide more accurate results but increase processing time
    pub film_grain_refinement_points_count: Option<usize>,
}