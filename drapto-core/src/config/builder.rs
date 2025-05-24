// ============================================================================
// drapto-core/src/config/builder.rs
// ============================================================================
//
// CONFIGURATION BUILDER: Builder Pattern for CoreConfig
//
// This module implements the builder pattern for the CoreConfig structure,
// providing a fluent API for creating and configuring CoreConfig instances.
// It allows for more readable and maintainable configuration code, with
// sensible defaults and validation.
//
// KEY COMPONENTS:
// - CoreConfigBuilder: Builder struct for creating CoreConfig instances
// - Validation methods for configuration parameters
// - Default values for optional parameters
//
// DESIGN PHILOSOPHY:
// The builder pattern provides a more flexible and readable way to create
// complex configuration objects, especially when many parameters are optional.
// It also allows for validation of parameters during construction.
//
// AI-ASSISTANT-INFO: Builder pattern implementation for CoreConfig

// ---- Standard library imports ----
use std::path::PathBuf;

// ---- Internal crate imports ----
use super::CoreConfig;
use crate::processing::detection::grain_analysis::GrainLevel;

/// Builder for creating CoreConfig instances.
///
/// This struct implements the builder pattern for CoreConfig, providing a
/// fluent API for creating and configuring CoreConfig instances with sensible
/// defaults and validation.
///
/// # Examples
///
/// ```rust
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
pub struct CoreConfigBuilder {
    // Required fields
    input_dir: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    log_dir: Option<PathBuf>,

    // Optional directory fields
    temp_dir: Option<PathBuf>,

    // Optional fields with defaults
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
    /// Creates a new CoreConfigBuilder with default values.
    ///
    /// # Returns
    ///
    /// * A new CoreConfigBuilder instance
    pub fn new() -> Self {
        Self {
            // Required fields
            input_dir: None,
            output_dir: None,
            log_dir: None,

            // Optional directory fields
            temp_dir: None,

            // Optional fields with defaults
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
    ///
    /// # Arguments
    ///
    /// * `input_dir` - The directory containing input video files
    ///
    /// # Returns
    ///
    /// * Self for method chaining
    pub fn input_dir(mut self, input_dir: PathBuf) -> Self {
        self.input_dir = Some(input_dir);
        self
    }

    /// Sets the output directory.
    ///
    /// # Arguments
    ///
    /// * `output_dir` - The directory where encoded output files will be saved
    ///
    /// # Returns
    ///
    /// * Self for method chaining
    pub fn output_dir(mut self, output_dir: PathBuf) -> Self {
        self.output_dir = Some(output_dir);
        self
    }

    /// Sets the log directory.
    ///
    /// # Arguments
    ///
    /// * `log_dir` - The directory for log files and temporary files
    ///
    /// # Returns
    ///
    /// * Self for method chaining
    pub fn log_dir(mut self, log_dir: PathBuf) -> Self {
        self.log_dir = Some(log_dir);
        self
    }

    /// Sets the temporary files directory.
    ///
    /// # Arguments
    ///
    /// * `temp_dir` - The directory for temporary files
    ///
    /// # Returns
    ///
    /// * Self for method chaining
    pub fn temp_dir(mut self, temp_dir: PathBuf) -> Self {
        self.temp_dir = Some(temp_dir);
        self
    }

    /// Sets whether to enable denoising.
    ///
    /// # Arguments
    ///
    /// * `enable` - Whether to enable denoising
    ///
    /// # Returns
    ///
    /// * Self for method chaining
    pub fn enable_denoise(mut self, enable: bool) -> Self {
        self.enable_denoise = enable;
        self
    }

    /// Sets the encoder preset.
    ///
    /// # Arguments
    ///
    /// * `preset` - The encoder preset (0-13, lower is slower/better quality)
    ///
    /// # Returns
    ///
    /// * Self for method chaining
    pub fn encoder_preset(mut self, preset: u8) -> Self {
        self.encoder_preset = preset;
        self
    }


    /// Sets the CRF quality for Standard Definition videos.
    ///
    /// # Arguments
    ///
    /// * `quality` - The CRF quality value (0-63, lower is higher quality)
    ///
    /// # Returns
    ///
    /// * Self for method chaining
    pub fn quality_sd(mut self, quality: u8) -> Self {
        self.quality_sd = quality;
        self
    }

    /// Sets the CRF quality for High Definition videos.
    ///
    /// # Arguments
    ///
    /// * `quality` - The CRF quality value (0-63, lower is higher quality)
    ///
    /// # Returns
    ///
    /// * Self for method chaining
    pub fn quality_hd(mut self, quality: u8) -> Self {
        self.quality_hd = quality;
        self
    }

    /// Sets the CRF quality for Ultra High Definition videos.
    ///
    /// # Arguments
    ///
    /// * `quality` - The CRF quality value (0-63, lower is higher quality)
    ///
    /// # Returns
    ///
    /// * Self for method chaining
    pub fn quality_uhd(mut self, quality: u8) -> Self {
        self.quality_uhd = quality;
        self
    }

    /// Sets the crop mode.
    ///
    /// # Arguments
    ///
    /// * `mode` - The crop mode ("auto", "none", etc.)
    ///
    /// # Returns
    ///
    /// * Self for method chaining
    pub fn crop_mode(mut self, mode: &str) -> Self {
        self.crop_mode = mode.to_string();
        self
    }


    /// Sets the ntfy.sh topic URL for sending notifications.
    ///
    /// # Arguments
    ///
    /// * `topic` - The ntfy.sh topic URL
    ///
    /// # Returns
    ///
    /// * Self for method chaining
    pub fn ntfy_topic(mut self, topic: &str) -> Self {
        self.ntfy_topic = Some(topic.to_string());
        self
    }

    /// Sets the sample duration for grain analysis in seconds.
    ///
    /// # Arguments
    ///
    /// * `duration` - The sample duration in seconds
    ///
    /// # Returns
    ///
    /// * Self for method chaining
    pub fn film_grain_sample_duration(mut self, duration: u32) -> Self {
        self.film_grain_sample_duration = duration;
        self
    }

    /// Sets the knee threshold for grain analysis.
    ///
    /// # Arguments
    ///
    /// * `threshold` - The knee threshold (0.1-1.0)
    ///
    /// # Returns
    ///
    /// * Self for method chaining
    pub fn film_grain_knee_threshold(mut self, threshold: f64) -> Self {
        self.film_grain_knee_threshold = threshold;
        self
    }

    /// Sets the maximum grain level for grain analysis.
    ///
    /// # Arguments
    ///
    /// * `level` - The maximum grain level
    ///
    /// # Returns
    ///
    /// * Self for method chaining
    pub fn film_grain_max_level(mut self, level: GrainLevel) -> Self {
        self.film_grain_max_level = level;
        self
    }

    /// Sets the number of refinement points for grain analysis.
    ///
    /// # Arguments
    ///
    /// * `count` - The number of refinement points
    ///
    /// # Returns
    ///
    /// * Self for method chaining
    pub fn film_grain_refinement_points_count(mut self, count: usize) -> Self {
        self.film_grain_refinement_points_count = count;
        self
    }


    /// Builds a CoreConfig instance from the builder.
    ///
    /// # Returns
    ///
    /// * A new CoreConfig instance
    ///
    /// # Panics
    ///
    /// * If any required fields are missing
    pub fn build(self) -> CoreConfig {
        // Validate required fields
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
