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
// AI-ASSISTANT-INFO: Utility functions for grain analysis

// ---- Internal module imports ----
use super::constants::HQDN3D_PARAMS;
use super::types::GrainLevel;

// ---- External crate imports ----
use colored::*;

/// Determines the appropriate hqdn3d filter parameter string based on the detected grain level.
///
/// This function maps a GrainLevel to the corresponding ffmpeg hqdn3d filter parameters.
/// For VeryClean videos, it returns None to indicate that no denoising should be applied.
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
/// * `None` - If the level is VeryClean (no denoising needed)
///
/// # Examples
///
/// ```rust
/// use drapto_core::processing::detection::grain_analysis::{GrainLevel, determine_hqdn3d_params};
///
/// // No denoising for very clean videos
/// assert_eq!(determine_hqdn3d_params(GrainLevel::VeryClean), None);
///
/// // Light denoising for light grain
/// assert!(determine_hqdn3d_params(GrainLevel::Light).is_some());
/// ```
pub fn determine_hqdn3d_params(level: GrainLevel) -> Option<String> {
    // For VeryClean videos, no denoising is needed
    if level == GrainLevel::VeryClean {
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

/// Calculates the median GrainLevel from a list of detected grain levels.
///
/// This function sorts the input array and returns the median element.
/// For even-length arrays, it returns the lower median (at index (len-1)/2).
/// If the input array is empty, it returns GrainLevel::VeryClean as a safe default.
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
/// * GrainLevel::VeryClean if the input slice is empty
///
/// # Note
///
/// This function modifies the input slice by sorting it.
pub(super) fn calculate_median_level(levels: &mut [GrainLevel]) -> GrainLevel {
    // Handle empty input case
    if levels.is_empty() {
        // This case should ideally not be reached if called after successful analysis phases
        log::warn!("calculate_median_level called with empty list. Defaulting to VeryClean.");
        return GrainLevel::VeryClean;
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