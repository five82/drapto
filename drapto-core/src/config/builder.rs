//! Builder pattern implementation for `CoreConfig`.
//!
//! This module provides a fluent API for creating and configuring `CoreConfig` instances
//! with sensible defaults and validation.


use std::path::PathBuf;

use super::CoreConfig;
use crate::processing::grain_types::GrainLevel;

/// Builder for creating `CoreConfig` instances.
///
/// This struct implements the builder pattern for `CoreConfig`, providing a
/// fluent API for creating and configuring `CoreConfig` instances with sensible
/// defaults and validation.
///
/// # Examples
///
/// ```rust
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
///     .film_grain_max_level(GrainLevel::Moderate)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct CoreConfigBuilder {
    input_dir: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    log_dir: Option<PathBuf>,
    temp_dir: Option<PathBuf>,
    enable_denoise: bool,
    encoder_preset: u8,
    quality_sd: u8,
    quality_hd: u8,
    quality_uhd: u8,
    crop_mode: String,
    ntfy_topic: Option<String>,
    film_grain_sample_duration: u32,
    film_grain_knee_threshold: f64,
    film_grain_max_level: GrainLevel,
    film_grain_refinement_points_count: usize,
}

impl Default for CoreConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CoreConfigBuilder {
    /// Creates a new `CoreConfigBuilder` with default values.
    #[must_use] pub fn new() -> Self {
        Self {
            input_dir: None,
            output_dir: None,
            log_dir: None,
            temp_dir: None,
            enable_denoise: true,
            encoder_preset: super::DEFAULT_ENCODER_PRESET,
            quality_sd: super::DEFAULT_CORE_QUALITY_SD,
            quality_hd: super::DEFAULT_CORE_QUALITY_HD,
            quality_uhd: super::DEFAULT_CORE_QUALITY_UHD,
            crop_mode: super::DEFAULT_CROP_MODE.to_string(),
            ntfy_topic: None,
            film_grain_sample_duration: super::DEFAULT_GRAIN_SAMPLE_DURATION,
            film_grain_knee_threshold: super::DEFAULT_GRAIN_KNEE_THRESHOLD,
            film_grain_max_level: super::DEFAULT_GRAIN_MAX_LEVEL,
            film_grain_refinement_points_count: super::DEFAULT_GRAIN_REFINEMENT_POINTS,
        }
    }

    /// Sets the input directory.
    #[must_use] pub fn input_dir(mut self, input_dir: PathBuf) -> Self {
        self.input_dir = Some(input_dir);
        self
    }

    /// Sets the output directory.
    #[must_use] pub fn output_dir(mut self, output_dir: PathBuf) -> Self {
        self.output_dir = Some(output_dir);
        self
    }

    /// Sets the log directory.
    #[must_use] pub fn log_dir(mut self, log_dir: PathBuf) -> Self {
        self.log_dir = Some(log_dir);
        self
    }

    /// Sets the temporary files directory.
    #[must_use] pub fn temp_dir(mut self, temp_dir: PathBuf) -> Self {
        self.temp_dir = Some(temp_dir);
        self
    }

    /// Sets whether to enable denoising.
    #[must_use] pub fn enable_denoise(mut self, enable: bool) -> Self {
        self.enable_denoise = enable;
        self
    }

    /// Sets the encoder preset (0-13, lower is slower/better quality).
    #[must_use] pub fn encoder_preset(mut self, preset: u8) -> Self {
        self.encoder_preset = preset;
        self
    }

    /// Sets the CRF quality for Standard Definition videos (0-63, lower is higher quality).
    #[must_use] pub fn quality_sd(mut self, quality: u8) -> Self {
        self.quality_sd = quality;
        self
    }

    /// Sets the CRF quality for High Definition videos (0-63, lower is higher quality).
    #[must_use] pub fn quality_hd(mut self, quality: u8) -> Self {
        self.quality_hd = quality;
        self
    }

    /// Sets the CRF quality for Ultra High Definition videos (0-63, lower is higher quality).
    #[must_use] pub fn quality_uhd(mut self, quality: u8) -> Self {
        self.quality_uhd = quality;
        self
    }

    /// Sets the crop mode ("auto", "none", etc.).
    #[must_use] pub fn crop_mode(mut self, mode: &str) -> Self {
        self.crop_mode = mode.to_string();
        self
    }

    /// Sets the ntfy.sh topic URL for sending notifications.
    #[must_use] pub fn ntfy_topic(mut self, topic: &str) -> Self {
        self.ntfy_topic = Some(topic.to_string());
        self
    }

    /// Sets the sample duration for grain analysis in seconds.
    #[must_use] pub fn film_grain_sample_duration(mut self, duration: u32) -> Self {
        self.film_grain_sample_duration = duration;
        self
    }

    /// Sets the knee threshold for grain analysis (0.1-1.0).
    #[must_use] pub fn film_grain_knee_threshold(mut self, threshold: f64) -> Self {
        self.film_grain_knee_threshold = threshold;
        self
    }

    /// Sets the maximum grain level for grain analysis.
    #[must_use] pub fn film_grain_max_level(mut self, level: GrainLevel) -> Self {
        self.film_grain_max_level = level;
        self
    }

    /// Sets the number of refinement points for grain analysis.
    #[must_use] pub fn film_grain_refinement_points_count(mut self, count: usize) -> Self {
        self.film_grain_refinement_points_count = count;
        self
    }

    /// Builds a `CoreConfig` instance from the builder.
    ///
    /// # Panics
    ///
    /// Panics if `input_dir`, `output_dir`, or `log_dir` are not set.
    #[must_use] pub fn build(self) -> CoreConfig {
        let input_dir = self.input_dir.expect("input_dir is required");
        let output_dir = self.output_dir.expect("output_dir is required");
        let log_dir = self.log_dir.expect("log_dir is required");

        CoreConfig {
            input_dir,
            output_dir,
            log_dir,
            temp_dir: self.temp_dir,
            enable_denoise: self.enable_denoise,
            encoder_preset: self.encoder_preset,
            quality_sd: self.quality_sd,
            quality_hd: self.quality_hd,
            quality_uhd: self.quality_uhd,
            crop_mode: self.crop_mode,
            ntfy_topic: self.ntfy_topic,
            film_grain_sample_duration: self.film_grain_sample_duration,
            film_grain_knee_threshold: self.film_grain_knee_threshold,
            film_grain_max_level: self.film_grain_max_level,
            film_grain_refinement_points_count: self.film_grain_refinement_points_count,
        }
    }
}
