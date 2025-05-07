// ============================================================================
// drapto-core/src/processing/detection/grain_analysis/types.rs
// ============================================================================
//
// GRAIN ANALYSIS TYPES: Type Definitions for Grain Analysis
//
// This file defines the types used in the grain analysis module, including
// the GrainLevel enum that represents different levels of grain/noise in a video
// and the GrainAnalysisResult structure that holds the final analysis result.
//
// AI-ASSISTANT-INFO: Type definitions for grain analysis results

// ---- External crate imports ----
use serde::{Deserialize, Serialize};

/// Represents the detected level of grain/noise in a video.
///
/// This enum categorizes the amount of film grain or noise present in a video,
/// which is determined through comparative encoding tests. Each level corresponds
/// to a different recommended denoising strength.
///
/// The levels are ordered from least grain (VeryClean) to most grain (Heavy),
/// and this ordering is reflected in the derived PartialOrd and Ord traits.
///
/// # Examples
///
/// ```rust
/// use drapto_core::processing::detection::grain_analysis::GrainLevel;
///
/// // Compare grain levels
/// assert!(GrainLevel::Heavy > GrainLevel::Light);
/// assert!(GrainLevel::VeryClean < GrainLevel::VeryLight);
///
/// // Use in match statements
/// let level = GrainLevel::Visible;
/// let description = match level {
///     GrainLevel::VeryClean => "No visible grain",
///     GrainLevel::VeryLight => "Barely noticeable grain",
///     GrainLevel::Light => "Light grain",
///     GrainLevel::Visible => "Noticeable grain",
///     GrainLevel::Heavy => "Heavy grain",
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum GrainLevel {
    /// Very little to no grain - minimal benefit from denoising
    VeryClean,

    /// Very light grain - benefits from very light denoising
    VeryLight,

    /// Light grain - benefits from light denoising
    Light,

    /// Noticeable grain - benefits from medium denoising
    Visible,

    /// Heavy grain - benefits from strong denoising
    Heavy,
}

/// Holds the final result of the grain analysis process.
///
/// This structure contains the detected grain level determined through
/// the multi-phase analysis process. It represents the consensus result
/// across multiple video samples and test encodings.
///
/// # Examples
///
/// ```rust
/// use drapto_core::processing::detection::grain_analysis::{GrainAnalysisResult, GrainLevel};
///
/// // Create a result with a detected level
/// let result = GrainAnalysisResult {
///     detected_level: GrainLevel::Light,
/// };
///
/// // Use the result to determine denoising parameters
/// let denoising_params = match result.detected_level {
///     GrainLevel::VeryClean => "0:0:0:0", // No denoising
///     GrainLevel::VeryLight => "1.5:1.5:1.0:1.0", // Very light denoising
///     GrainLevel::Light => "3.0:2.5:2.0:1.5", // Light denoising
///     GrainLevel::Visible => "6.0:4.5:3.0:2.5", // Medium denoising
///     GrainLevel::Heavy => "10.0:8.0:6.0:4.0", // Strong denoising
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrainAnalysisResult {
    /// The final detected grain level based on median of sample analyses.
    /// This represents the consensus grain level across multiple samples.
    pub detected_level: GrainLevel,
}