//! Type definitions for grain analysis.
//!
//! This file defines the types used in the grain analysis module, including
//! the GrainLevel enum that represents different levels of grain/noise in a video
//! and the GrainAnalysisResult structure that holds the final analysis result.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Represents the detected level of grain/noise in a video.
///
/// This enum categorizes the amount of film grain or noise present in a video,
/// which is determined through comparative encoding tests. Each level corresponds
/// to a different recommended denoising strength.
///
/// The levels are ordered from least grain (Baseline) to most grain (Elevated),
/// and this ordering is reflected in the derived `PartialOrd` and Ord traits.
///
/// # Examples
///
/// ```rust
/// use drapto_core::processing::grain_types::GrainLevel;
///
/// // Compare grain levels
/// assert!(GrainLevel::Elevated > GrainLevel::Light);
/// assert!(GrainLevel::Baseline < GrainLevel::VeryLight);
///
/// // Use in match statements
/// let level = GrainLevel::Moderate;
/// let description = match level {
///     GrainLevel::Baseline => "No visible grain",
///     GrainLevel::VeryLight => "Barely noticeable grain",
///     GrainLevel::Light => "Light grain",
///     GrainLevel::LightModerate => "Light to moderate grain",
///     GrainLevel::Moderate => "Noticeable grain with spatial patterns",
///     GrainLevel::Elevated => "Medium grain with temporal fluctuations",
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum GrainLevel {
    /// Very little to no grain - minimal benefit from denoising
    Baseline,

    /// Very light grain - benefits from very light denoising
    VeryLight,

    /// Light grain - benefits from light denoising
    Light,

    /// Light to moderate grain - benefits from balanced denoising
    LightModerate,

    /// Noticeable grain - benefits from spatially-focused denoising (higher luma/chroma spatial strength)
    Moderate,

    /// Medium grain - benefits from temporally-focused denoising (higher temporal strength values)
    Elevated,
}

/// Error type for `GrainLevel` parsing failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrainLevelParseError {
    /// The invalid string that couldn't be parsed
    pub invalid_value: String,
}

impl std::fmt::Display for GrainLevelParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid grain level: {}", self.invalid_value)
    }
}

impl std::error::Error for GrainLevelParseError {}

impl FromStr for GrainLevel {
    type Err = GrainLevelParseError;

    /// Parses a string into a `GrainLevel`.
    ///
    /// # Arguments
    ///
    /// * `s` - The string to parse
    ///
    /// # Returns
    ///
    /// * `Ok(GrainLevel)` - If the string was successfully parsed
    /// * `Err(GrainLevelParseError)` - If the string couldn't be parsed
    ///
    /// # Examples
    ///
    /// ```rust
    /// use drapto_core::processing::grain_types::GrainLevel;
    /// use std::str::FromStr;
    ///
    /// // Parse grain level names
    /// assert_eq!(GrainLevel::from_str("baseline").unwrap(), GrainLevel::Baseline);
    /// assert_eq!(GrainLevel::from_str("moderate").unwrap(), GrainLevel::Moderate);
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "baseline" => Ok(GrainLevel::Baseline),
            "verylight" => Ok(GrainLevel::VeryLight),
            "light" => Ok(GrainLevel::Light),
            "lightmoderate" => Ok(GrainLevel::LightModerate),
            "moderate" => Ok(GrainLevel::Moderate),
            "elevated" => Ok(GrainLevel::Elevated),
            _ => Err(GrainLevelParseError {
                invalid_value: s.to_string(),
            }),
        }
    }
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
/// use drapto_core::processing::grain_types::{GrainAnalysisResult, GrainLevel};
///
/// // Create a result with a detected level
/// let result = GrainAnalysisResult {
///     detected_level: GrainLevel::Light,
/// };
///
/// // Use the result to determine denoising parameters (example values)
/// let denoising_params = match result.detected_level {
///     GrainLevel::Baseline => "0:0:0:0", // No denoising
///     GrainLevel::VeryLight => "0.5:0.4:3:3", // Very light denoising
///     GrainLevel::Light => "0.9:0.7:4:4", // Light denoising
///     GrainLevel::LightModerate => "1.2:0.85:5:5", // Light to moderate denoising
///     GrainLevel::Moderate => "1.5:1.0:6:6", // Moderate denoising
///     GrainLevel::Elevated => "2.0:1.3:8:8", // Elevated denoising
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrainAnalysisResult {
    /// The final detected grain level based on median of sample analyses.
    /// This represents the consensus grain level across multiple samples.
    pub detected_level: GrainLevel,
}

/// Holds encoding results for a single grain level test
#[derive(Debug, Clone)]
pub struct GrainLevelTestResult {
    /// The encoded file size in bytes
    pub file_size: u64,
    /// XPSNR quality metric compared to baseline (higher is better)
    pub xpsnr: Option<f64>,
}

/// Grain level parameters that combine denoising and film grain synthesis
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GrainParameters {
    /// hqdn3d denoising parameters (spatial_luma:spatial_chroma:temporal_luma:temporal_chroma)
    pub hqdn3d: &'static str,
    /// Film grain synthesis strength (0-50)
    pub film_grain: u8,
}

impl GrainLevel {
    /// Returns the grain parameters (hqdn3d and film grain synthesis) for this level
    /// 
    /// IMPORTANT: Do not change these values unless explicitly instructed.
    /// These parameters are carefully calibrated and documented in docs/film_grain_technical.md.
    /// Any changes must be synchronized with the technical documentation.
    pub fn parameters(self) -> Option<GrainParameters> {
        match self {
            GrainLevel::Baseline => None,
            GrainLevel::VeryLight => Some(GrainParameters {
                hqdn3d: "0.5:0.4:3:3",
                film_grain: 4,
            }),
            GrainLevel::Light => Some(GrainParameters {
                hqdn3d: "0.9:0.7:4:4",
                film_grain: 7,
            }),
            GrainLevel::LightModerate => Some(GrainParameters {
                hqdn3d: "1.2:0.85:5:5",
                film_grain: 10,
            }),
            GrainLevel::Moderate => Some(GrainParameters {
                hqdn3d: "1.5:1.0:6:6",
                film_grain: 13,
            }),
            GrainLevel::Elevated => Some(GrainParameters {
                hqdn3d: "2:1.3:8:8",
                film_grain: 16,
            }),
        }
    }

    /// Returns just the hqdn3d parameters for this level
    pub fn hqdn3d_params(self) -> Option<&'static str> {
        self.parameters().map(|p| p.hqdn3d)
    }

    /// Returns just the film grain synthesis value for this level
    pub fn film_grain_value(self) -> Option<u8> {
        self.parameters().map(|p| p.film_grain)
    }
}
