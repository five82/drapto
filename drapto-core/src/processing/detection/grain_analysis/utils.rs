// ============================================================================
// drapto-core/src/processing/detection/grain_analysis/utils.rs
// ============================================================================
//
// GRAIN ANALYSIS UTILITIES: Helper Functions for Grain Analysis
//
// This file provides utility functions for the grain analysis module, including
// functions to determine denoising parameters from grain levels and to calculate
// the median grain level from a list of detected levels.
//
// KEY COMPONENTS:
// - Continuous parameter interpolation for fine-grained denoising control
// - Mapping between GrainLevel enum and numeric strength values
// - Utility functions for calculating median grain level
//
// AI-ASSISTANT-INFO: Utility functions for grain analysis

// ---- Internal module imports ----
use super::constants::HQDN3D_PARAMS;
use super::types::GrainLevel;

// ---- External crate imports ----
use colored::*;

/// Determines the appropriate hqdn3d filter parameter string based on the detected grain level.
///
/// This function maps a GrainLevel to the corresponding ffmpeg hqdn3d filter parameters.
/// For Baseline videos, it returns None to indicate that no denoising should be applied.
/// For all other levels, it returns a string with the appropriate parameters.
///
/// The hqdn3d filter parameters are in the format "y:cb:cr:strength" where:
/// - y: Luma spatial strength (higher values = more denoising)
/// - cb: Chroma spatial strength
/// - cr: Temporal strength
/// - strength: Temporal chroma strength
///
/// # Arguments
///
/// * `level` - The detected grain level
///
/// # Returns
///
/// * `Some(String)` - The hqdn3d filter parameters as a string
/// * `None` - If the level is Baseline (no denoising needed)
///
/// # Examples
///
/// ```rust
/// use drapto_core::processing::detection::grain_analysis::{GrainLevel, determine_hqdn3d_params};
///
/// // No denoising for very clean videos
/// assert_eq!(determine_hqdn3d_params(GrainLevel::Baseline), None);
///
/// // Light denoising for light grain
/// assert!(determine_hqdn3d_params(GrainLevel::Light).is_some());
/// ```
pub fn determine_hqdn3d_params(level: GrainLevel) -> Option<String> {
    // For Baseline videos, no denoising is needed
    if level == GrainLevel::Baseline {
        return None;
    }

    // Find the corresponding parameter string in the constants map
    HQDN3D_PARAMS
        .iter()
        .find(|(l, _)| *l == level)
        .map(|(_, s)| s.to_string())
        .or_else(|| {
            // This should never happen if all GrainLevel variants are covered in HQDN3D_PARAMS
            log::warn!(
                "{} Could not find hqdn3d params for level {:?}, this is unexpected.",
                "Warning:".yellow().bold(),
                level
            );
            None
        })
}

/// Maps a GrainLevel enum to a continuous strength value.
///
/// This function converts a GrainLevel enum variant to a numeric value on a
/// continuous scale from 0.0 to 4.0, where:
/// - 0.0 = Baseline (no denoising)
/// - 1.0 = VeryLight
/// - 2.0 = Light
/// - 3.0 = Moderate
/// - 4.0 = Elevated
///
/// This mapping is used for interpolation and calculations in the grain analysis process.
///
/// # Arguments
///
/// * `level` - The GrainLevel enum variant to convert
///
/// # Returns
///
/// * A float value representing the strength on a continuous scale
pub fn grain_level_to_strength(level: GrainLevel) -> f32 {
    match level {
        GrainLevel::Baseline => 0.0,
        GrainLevel::VeryLight => 1.0,
        GrainLevel::Light => 2.0,
        GrainLevel::Moderate => 3.0,
        GrainLevel::Elevated => 4.0,
    }
}

/// Maps a continuous strength value to the closest GrainLevel enum.
///
/// This function converts a numeric strength value (0.0-4.0) to the
/// nearest GrainLevel enum variant. It's the inverse of grain_level_to_strength.
///
/// # Arguments
///
/// * `strength` - A float value representing denoising strength (0.0-4.0)
///
/// # Returns
///
/// * The closest GrainLevel enum variant
pub fn strength_to_grain_level(strength: f32) -> GrainLevel {
    match strength {
        s if s <= 0.5 => GrainLevel::Baseline,
        s if s <= 1.5 => GrainLevel::VeryLight,
        s if s <= 2.5 => GrainLevel::Light,
        s if s <= 3.5 => GrainLevel::Moderate,
        _ => GrainLevel::Elevated,
    }
}

/// Generates interpolated hqdn3d parameters based on a continuous strength value.
///
/// This function maps a continuous strength value (0.0-4.0) to appropriate hqdn3d
/// parameters by interpolating between the "anchor points" defined by the standard
/// GrainLevel enum values. This allows for more fine-grained control over denoising
/// than the discrete GrainLevel enum provides.
///
/// The hqdn3d parameters are in the format "y:cb:cr:strength" where:
/// - y: Luma spatial strength (higher values = more denoising)
/// - cb: Chroma spatial strength
/// - cr: Temporal luma strength
/// - strength: Temporal chroma strength
///
/// # Arguments
///
/// * `strength_value` - A float value representing denoising strength (0.0-4.0)
///
/// # Returns
///
/// * A String containing the interpolated hqdn3d parameters
/// * An empty string for strength_value <= 0.0 (no denoising)
///
/// # Examples
///
/// ```no_run
/// // This function is exported from the module, but the doctest can't access it directly
/// // Example usage within the crate:
/// //
/// // // No denoising
/// // assert_eq!(generate_hqdn3d_params(0.0), "".to_string());
/// //
/// // // Halfway between Baseline (0.0) and VeryLight (1.0)
/// // assert_eq!(generate_hqdn3d_params(0.5), "hqdn3d=0.25:0.15:1.5:1.5");
/// //
/// // // Exactly at Light (2.0)
/// // assert_eq!(generate_hqdn3d_params(2.0), "hqdn3d=1:0.7:4:4");
/// ```
pub fn generate_hqdn3d_params(strength_value: f32) -> String {
    // Map strength_value (0.0-4.0 continuous scale) to appropriate hqdn3d parameters
    // 0.0 = Baseline, 1.0 = VeryLight, 2.0 = Light, 3.0 = Moderate, 4.0 = Elevated

    // No denoising for strength <= 0
    if strength_value <= 0.0 {
        return "".to_string();
    }

    // Define the anchor points for interpolation
    // Format: (strength, luma_spatial, chroma_spatial, temp_luma, temp_chroma)
    let anchor_points = [
        (0.0, 0.0, 0.0, 0.0, 0.0),       // Baseline (no denoising)
        (1.0, 0.5, 0.3, 3.0, 3.0),       // VeryLight
        (2.0, 1.0, 0.7, 4.0, 4.0),       // Light
        (3.0, 1.5, 1.0, 6.0, 6.0),       // Moderate
        (4.0, 2.0, 1.3, 8.0, 8.0),       // Elevated
    ];

    // Find the two anchor points to interpolate between
    let mut lower_idx = 0;
    let mut upper_idx = 1;

    for (i, point) in anchor_points.iter().enumerate().skip(1) {
        if strength_value <= point.0 {
            lower_idx = i - 1;
            upper_idx = i;
            break;
        }
    }

    // If we're at or beyond the maximum strength, use the highest anchor point
    if strength_value >= anchor_points[anchor_points.len() - 1].0 {
        let (_, luma, chroma, temp_luma, temp_chroma) = anchor_points[anchor_points.len() - 1];
        return format!("hqdn3d={:.2}:{:.2}:{:.2}:{:.2}", luma, chroma, temp_luma, temp_chroma);
    }

    // Extract the anchor points
    let (lower_strength, lower_luma, lower_chroma, lower_temp_luma, lower_temp_chroma) = anchor_points[lower_idx];
    let (upper_strength, upper_luma, upper_chroma, upper_temp_luma, upper_temp_chroma) = anchor_points[upper_idx];

    // Calculate the interpolation factor (0.0-1.0)
    let range = upper_strength - lower_strength;
    let factor = if range > 0.0 { (strength_value - lower_strength) / range } else { 0.0 };

    // Interpolate each parameter
    let luma = lower_luma + factor * (upper_luma - lower_luma);
    let chroma = lower_chroma + factor * (upper_chroma - lower_chroma);
    let temp_luma = lower_temp_luma + factor * (upper_temp_luma - lower_temp_luma);
    let temp_chroma = lower_temp_chroma + factor * (upper_temp_chroma - lower_temp_chroma);

    // Format the parameters string
    format!("hqdn3d={:.2}:{:.2}:{:.2}:{:.2}", luma, chroma, temp_luma, temp_chroma)
}

/// Calculates the median GrainLevel from a list of detected grain levels.
///
/// This function sorts the input array and returns the median element.
/// For even-length arrays, it returns the lower median (at index (len-1)/2).
/// If the input array is empty, it returns GrainLevel::Baseline as a safe default.
///
/// This function is used to determine the consensus grain level across multiple
/// video samples, providing a robust estimate that is less affected by outliers
/// than a mean would be.
///
/// # Arguments
///
/// * `levels` - A mutable slice of GrainLevel values to find the median of
///
/// # Returns
///
/// * The median GrainLevel value
/// * GrainLevel::Baseline if the input slice is empty
///
/// # Note
///
/// This function modifies the input slice by sorting it.
pub(super) fn calculate_median_level(levels: &mut [GrainLevel]) -> GrainLevel {
    // Handle empty input case
    if levels.is_empty() {
        // This case should ideally not be reached if called after successful analysis phases
        log::warn!("calculate_median_level called with empty list. Defaulting to Baseline.");
        return GrainLevel::Baseline;
    }

    // Sort the levels to find the median
    // sort_unstable is fine as we only need the median element
    levels.sort_unstable();

    // Calculate the median index
    // For even-length arrays, this gives the lower median (at index (len-1)/2)
    let mid = (levels.len() - 1) / 2;

    // Return the median element
    levels[mid]
}