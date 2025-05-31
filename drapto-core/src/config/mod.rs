//! Configuration structures and constants for the drapto-core library.
//! 
//! This module provides the configuration system for video processing behavior,
//! including encoding parameters, quality settings, and analysis options.

mod builder;

use std::path::PathBuf;

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

/// Fixed denoising parameters for hqdn3d filter.
/// Format: spatial_luma:spatial_chroma:temporal_luma:temporal_chroma
pub const FIXED_HQDN3D_PARAMS: &str = "0.5:0.4:2:2";

/// Fixed film grain synthesis value for SVT-AV1.
/// Range: 0-50, where 0 is no grain and 50 is maximum grain.
pub const FIXED_FILM_GRAIN_VALUE: u8 = 4;



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
