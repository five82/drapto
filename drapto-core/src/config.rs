// drapto-core/src/config.rs
//
// This module defines the core configuration structures and related types
// used throughout the `drapto-core` library.
//
// It includes:
// - `FilmGrainMetricType`: An enum defining the different strategies available
//   for determining the optimal film grain value during optimization.
// - `CoreConfig`: The main configuration struct holding all parameters required
//   for the core processing logic. This includes input/output paths, logging paths,
//   optional overrides for Handbrake defaults (like preset, quality, crop), and
//   detailed settings for the film grain optimization process (e.g., enabling
//   optimization, sample duration/count, initial values, fallback value, metric type,
//   thresholds, refinement parameters).
//
// Instances of `CoreConfig` are typically created by consumers of the library
// (like `drapto-cli`) and passed into the main processing functions.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)] // Added PartialEq, Eq for comparison
pub enum FilmGrainMetricType {
    KneePoint,
    // PercentMaxReduction, // Keep commented out for now as we focus on KneePoint
    // OriginalEfficiency, // Keep commented out for now
}

// --- Core Default Values ---
// These are used by the core logic if specific values are not provided in CoreConfig.
pub const DEFAULT_CORE_QUALITY_SD: u8 = 28;
pub const DEFAULT_CORE_QUALITY_HD: u8 = 29;
pub const DEFAULT_CORE_QUALITY_UHD: u8 = 30;
// Add other core defaults here if needed (e.g., default preset)

#[derive(Debug, Clone)] // Configuration for the core processing
pub struct CoreConfig {
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    pub log_dir: PathBuf,
    // --- Optional Handbrake Defaults ---
    pub default_encoder_preset: Option<u8>,
    // pub default_quality: Option<u8>, // Replaced by resolution-specific qualities
    pub quality_sd: Option<u8>, // Quality for Standard Definition (e.g., < 1920 width)
    pub quality_hd: Option<u8>, // Quality for High Definition (e.g., >= 1920 width)
    pub quality_uhd: Option<u8>, // Quality for Ultra High Definition (e.g., >= 3840 width)
    pub default_crop_mode: Option<String>,
    // --- Film Grain Optimization ---
    /// Enable automatic film grain optimization (default: false)
    pub optimize_film_grain: bool,
    /// Duration in seconds for each sample clip (default: 10)
    pub film_grain_sample_duration: Option<u32>,
    /// Number of sample points to extract (default: 3)
    pub film_grain_sample_count: Option<usize>,
    /// Initial grain values to test (default: [0, 8, 20])
    pub film_grain_initial_values: Option<Vec<u8>>,
    /// Fallback grain value if optimization fails or is disabled (default: 0)
    pub film_grain_fallback_value: Option<u8>,
    /// Which metric to use for determining optimal grain (default: KneePoint)
    pub film_grain_metric_type: Option<FilmGrainMetricType>,
    /// Threshold for KneePoint metric (percentage of max efficiency, e.g., 0.8 for 80%)
    pub film_grain_knee_threshold: Option<f64>,
    /// +/- range around the Phase 2 median estimate for Phase 3 refinement (default: 3)
    pub film_grain_refinement_range_delta: Option<u8>,
    /// Maximum allowed film grain value for the final result (default: 20)
    pub film_grain_max_value: Option<u8>,
    /// Number of refinement points to test in Phase 3 (default: 3) - Moved from film_grain.rs constants
    pub film_grain_refinement_points_count: Option<usize>,
}