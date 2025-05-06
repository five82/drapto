// drapto-core/src/processing/detection/grain_analysis/constants.rs
use super::types::GrainLevel;

// Map GrainLevel to specific hqdn3d parameter strings for testing and final application
// Ordered by strength for iteration in analysis logic
pub(super) const HQDN3D_PARAMS: [(GrainLevel, &str); 4] = [
    (GrainLevel::VeryLight, "hqdn3d=0.5:0.3:3:3"),
    (GrainLevel::Light, "hqdn3d=1:0.7:4:4"),
    (GrainLevel::Visible, "hqdn3d=1.5:1.0:6:6"),
    (GrainLevel::Heavy, "hqdn3d=2:1.3:8:8"),
];

// --- Constants for Sampling (Adapted from reference/mod.rs) ---
pub(super) const DEFAULT_SAMPLE_DURATION_SECS: u32 = 10;
pub(super) const MIN_SAMPLES: usize = 3; // Updated to match reference
pub(super) const MAX_SAMPLES: usize = 9; // Updated to match reference
pub(super) const SECS_PER_SAMPLE_TARGET: f64 = 600.0;

// --- Constants for Analysis ---
pub(super) const KNEE_THRESHOLD: f64 = 0.8; // 80% efficiency threshold (from reference code)