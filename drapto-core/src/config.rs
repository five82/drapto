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
// Removed unused import: use crate::external::ffmpeg::HardwareAccel;

#[derive(Debug, Clone, PartialEq, Eq)] // Added PartialEq, Eq for comparison
pub enum FilmGrainMetricType {
    KneePoint,
    // PercentMaxReduction, // Keep commented out for now as we focus on KneePoint
    // OriginalEfficiency, // Keep commented out for now
}

// --- Core Default Values ---
// These are used by the core logic if specific values are not provided in CoreConfig.
pub const DEFAULT_CORE_QUALITY_SD: u8 = 25;
pub const DEFAULT_CORE_QUALITY_HD: u8 = 27;
pub const DEFAULT_CORE_QUALITY_UHD: u8 = 27;
// Add other core defaults here if needed (e.g., default preset)

#[derive(Debug, Clone)] // Configuration for the core processing
pub struct CoreConfig {
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    pub log_dir: PathBuf,
    // --- Optional Handbrake Defaults ---
    pub default_encoder_preset: Option<u8>, // Keep this as the primary way to set default
    pub preset: Option<u8>, // New field for CLI override (numeric)
    // pub default_quality: Option<u8>, // Replaced by resolution-specific qualities
    pub quality_sd: Option<u8>, // Quality for Standard Definition (e.g., < 1920 width)
    pub quality_hd: Option<u8>, // Quality for High Definition (e.g., >= 1920 width)
    pub quality_uhd: Option<u8>, // Quality for Ultra High Definition (e.g., >= 3840 width)
    pub default_crop_mode: Option<String>, // Crop mode for the main encode
    // --- Notifications ---
    /// Optional ntfy.sh topic URL for notifications
    pub ntfy_topic: Option<String>,
/// Enable light video denoising (hqdn3d) by default.
    pub enable_denoise: bool,
    // Hardware acceleration field removed as it's no longer supported.
}