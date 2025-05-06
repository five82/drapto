// drapto-core/src/processing/detection/grain_analysis/types.rs
use serde::{Deserialize, Serialize};

/// Represents the detected level of grain in the video, determined by relative encode comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum GrainLevel {
    VeryClean, // Corresponds to no significant benefit from denoising
    VeryLight, // Corresponds to benefit from very light denoising
    Light,     // Corresponds to benefit from light denoising
    Visible,   // Corresponds to benefit from medium denoising
    Heavy,     // Corresponds to benefit from heavy denoising
}

/// Holds the final result of the grain analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrainAnalysisResult {
    /// The final detected grain level based on median of sample analyses.
    pub detected_level: GrainLevel,
}