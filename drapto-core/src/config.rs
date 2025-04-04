// drapto-core/src/config.rs
// Responsibility: Define core configuration structures and related constants/defaults.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)] // Added PartialEq, Eq for comparison
pub enum FilmGrainMetricType {
    KneePoint,
    // PercentMaxReduction, // Keep commented out for now as we focus on KneePoint
    // OriginalEfficiency, // Keep commented out for now
}

#[derive(Debug, Clone)] // Configuration for the core processing
pub struct CoreConfig {
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    pub log_dir: PathBuf,
    // --- Optional Handbrake Defaults ---
    pub default_encoder_preset: Option<u8>,
    pub default_quality: Option<u8>,
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