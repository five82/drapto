// drapto-core/src/processing/detection/grain_analysis/utils.rs
use super::constants::HQDN3D_PARAMS;
use super::types::GrainLevel;
use colored::*;

/// Determines the appropriate hqdn3d filter parameter string based on the detected grain level.
/// Returns None if the level is VeryClean.
pub fn determine_hqdn3d_params(level: GrainLevel) -> Option<String> {
    if level == GrainLevel::VeryClean {
        return None;
    }
    // Find the corresponding string in the map
    HQDN3D_PARAMS
        .iter()
        .find(|(l, _)| *l == level)
        .map(|(_, s)| s.to_string())
        .or_else(|| {
            log::warn!("{} Could not find hqdn3d params for level {:?}, this is unexpected.", "Warning:".yellow().bold(), level);
            None
        })
}

/// Calculates the median GrainLevel from a list of levels.
pub(super) fn calculate_median_level(levels: &mut [GrainLevel]) -> GrainLevel {
    if levels.is_empty() {
        // This case should ideally not be reached if called after successful analysis phases
        log::warn!("calculate_median_level called with empty list. Defaulting to VeryClean.");
        return GrainLevel::VeryClean;
    }
    // Sort unstable is fine as we only need the median element
    levels.sort_unstable();
    // Use (len - 1) / 2 to get the lower median index for even lengths, matching reference behavior
    let mid = (levels.len() - 1) / 2;
    levels[mid]
}