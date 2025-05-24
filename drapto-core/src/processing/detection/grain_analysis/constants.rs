// ============================================================================
// drapto-core/src/processing/detection/grain_analysis/constants.rs
// ============================================================================
//
// GRAIN ANALYSIS CONSTANTS: Configuration Values for Grain Analysis
//
// This file defines constants used throughout the grain analysis module,
// including denoising parameters for different grain levels, sampling parameters,
// and analysis thresholds.
//
// AI-ASSISTANT-INFO: Constants for grain analysis configuration

// ---- Internal module imports ----
use super::types::GrainLevel;

// ============================================================================
// DENOISING PARAMETERS
// ============================================================================

/// Maps GrainLevel to specific hqdn3d parameter strings for testing and final application.
///
/// These parameters define the strength of the hqdn3d denoising filter for each grain level.
/// The parameters are in the format "hqdn3d=y:cb:cr:strength" where:
/// - y: Luma spatial strength (higher values = more denoising)
/// - cb: Chroma spatial strength
/// - cr: Temporal strength
/// - strength: Temporal chroma strength
///
/// The array is ordered by increasing strength for iteration in analysis logic.
/// Note: GrainLevel::Baseline is not included as it corresponds to no denoising.
pub(super) const HQDN3D_PARAMS: [(GrainLevel, &str); 4] = [
    // Very light denoising for barely noticeable grain
    (GrainLevel::VeryLight, "hqdn3d=0.5:0.3:3:3"),
    // Light denoising for light grain
    (GrainLevel::Light, "hqdn3d=1:0.7:4:4"),
    // Spatially-focused denoising for noticeable grain (higher spatial values)
    (GrainLevel::Moderate, "hqdn3d=1.5:1.0:6:6"),
    // Temporally-focused denoising for medium grain (higher temporal values)
    (GrainLevel::Elevated, "hqdn3d=2:1.3:8:8"),
];

// ============================================================================
// SAMPLING PARAMETERS
// ============================================================================

// Default sample duration is now defined in config/mod.rs as DEFAULT_GRAIN_SAMPLE_DURATION

/// Minimum number of samples to extract from a video for reliable analysis.
/// At least 3 samples are needed for robust median calculation.
pub(super) const MIN_SAMPLES: usize = 3;

/// Maximum number of samples to extract from a video.
/// Limiting the number of samples prevents excessive processing time for long videos.
pub(super) const MAX_SAMPLES: usize = 9;

/// Target number of seconds of video per sample.
/// Used to calculate the number of samples based on video duration.
/// For example, a 1-hour video (3600s) would use 3600/600 = 6 samples.
pub(super) const SECS_PER_SAMPLE_TARGET: f64 = 600.0;

// ============================================================================
// ANALYSIS PARAMETERS
// ============================================================================

// Knee threshold is now defined in config/mod.rs as DEFAULT_GRAIN_KNEE_THRESHOLD
