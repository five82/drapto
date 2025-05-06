// drapto-core/src/processing/detection/grain_analysis/refinement.rs
use super::constants::HQDN3D_PARAMS;
use super::types::GrainLevel;

/// Interpolates hqdn3d parameters between two predefined GrainLevels.
/// Returns a new param string.
fn interpolate_hqdn3d_params(lower_level: GrainLevel, upper_level: GrainLevel, factor: f32) -> String {
    // Get param strings for the two levels
    let lower_params = HQDN3D_PARAMS.iter()
        .find(|(level, _)| *level == lower_level)
        .map(|(_, params)| *params)
        .unwrap_or("hqdn3d=0:0:0:0"); // Default if lower level not found (shouldn't happen)

    let upper_params = HQDN3D_PARAMS.iter()
        .find(|(level, _)| *level == upper_level)
        .map(|(_, params)| *params)
        .unwrap_or("hqdn3d=0:0:0:0"); // Default if upper level not found (shouldn't happen)

    // Parse the parameters into components
    let l_components = parse_hqdn3d_params(lower_params);
    let u_components = parse_hqdn3d_params(upper_params);

    // Interpolate each component
    let new_luma_spatial = l_components.0 + (u_components.0 - l_components.0) * factor;
    let new_chroma_spatial = l_components.1 + (u_components.1 - l_components.1) * factor;
    let new_luma_tmp = l_components.2 + (u_components.2 - l_components.2) * factor;
    let new_chroma_tmp = l_components.3 + (u_components.3 - l_components.3) * factor;

    // Format with controlled precision
    format!("hqdn3d={:.1}:{:.1}:{:.1}:{:.1}", // Use .1 for tmp as well, reference has floats
        new_luma_spatial.max(0.0), // Ensure non-negative
        new_chroma_spatial.max(0.0),
        new_luma_tmp.max(0.0),
        new_chroma_tmp.max(0.0)
    )
}

/// Helper to parse hqdn3d parameter string into components.
/// Returns (luma_spatial, chroma_spatial, luma_tmp, chroma_tmp)
fn parse_hqdn3d_params(params: &str) -> (f32, f32, f32, f32) {
    const DEFAULT_PARAMS: (f32, f32, f32, f32) = (0.0, 0.0, 0.0, 0.0);

    if let Some(values_str) = params.strip_prefix("hqdn3d=") {
        let parts: Vec<_> = values_str.split(':').collect();
        if parts.len() == 4 {
            let parsed_parts: Vec<Result<f32, _>> = parts.iter().map(|s| s.parse()).collect();
            if parsed_parts.iter().all(Result::is_ok) {
                // All parts parsed successfully
                return (
                    parsed_parts[0].as_ref().unwrap().max(0.0), // Ensure non-negative
                    parsed_parts[1].as_ref().unwrap().max(0.0),
                    parsed_parts[2].as_ref().unwrap().max(0.0),
                    parsed_parts[3].as_ref().unwrap().max(0.0),
                );
            } else {
                // Log specific parsing errors if any
                for (i, res) in parsed_parts.iter().enumerate() {
                    if let Err(e) = res {
                        log::warn!("Failed to parse hqdn3d component #{} ('{}') in '{}': {}. Using default.", i+1, parts[i], params, e);
                    }
                }
            }
        } else {
             log::warn!("Incorrect number of components ({}) in hqdn3d params: '{}'. Expected 4. Using defaults.", parts.len(), params);
        }
    } else {
        log::warn!("Invalid hqdn3d param string format (missing 'hqdn3d=' prefix): '{}'. Using defaults.", params);
    }
    DEFAULT_PARAMS // Return defaults if any parsing step failed
}


/// Calculate standard deviation of grain level estimates.
/// Returns the standard deviation as f64.
fn calculate_std_dev(levels: &[GrainLevel]) -> f64 {
    if levels.len() <= 1 {
        return 0.0;
    }

    // Convert GrainLevel to numeric values
    let numeric_values: Vec<f64> = levels.iter().map(|level| match level {
        GrainLevel::VeryClean => 0.0,
        GrainLevel::VeryLight => 1.0,
        GrainLevel::Light => 2.0,
        GrainLevel::Visible => 3.0,
        GrainLevel::Heavy => 4.0,
    }).collect();

    // Calculate mean
    let mean: f64 = numeric_values.iter().sum::<f64>() / numeric_values.len() as f64;

    // Calculate variance (using n, not n-1, as per reference code's apparent logic)
    let variance: f64 = numeric_values.iter()
        .map(|val| (val - mean).powi(2))
        .sum::<f64>() / numeric_values.len() as f64;

    // Return standard deviation
    variance.sqrt()
}

/// Determine the refinement range based on initial estimates.
/// Returns a tuple with (lower bound GrainLevel, upper bound GrainLevel).
pub(super) fn calculate_refinement_range(initial_estimates: &[GrainLevel]) -> (GrainLevel, GrainLevel) {
    if initial_estimates.is_empty() {
        // Default range if no estimates (shouldn't happen if called correctly)
        log::warn!("calculate_refinement_range called with empty estimates. Defaulting range.");
        return (GrainLevel::VeryClean, GrainLevel::VeryLight);
    }

    // Create a copy and sort for median calculation
    let mut sorted_estimates = initial_estimates.to_vec();
    sorted_estimates.sort(); // GrainLevel derives Ord

    // Get median
    let median_idx = sorted_estimates.len() / 2; // Integer division gives lower median for even len
    let median = sorted_estimates[median_idx];

    // Calculate std dev and use it to determine range width
    let std_dev = calculate_std_dev(initial_estimates);
    const ADAPTIVE_FACTOR: f64 = 1.5; // From reference code
    // Round to nearest integer level delta, ensure at least 1 level difference
    let level_delta = (std_dev * ADAPTIVE_FACTOR).round().max(1.0) as usize;

    // Convert median GrainLevel to index
    let median_index: usize = match median { // Explicitly type as usize
        GrainLevel::VeryClean => 0,
        GrainLevel::VeryLight => 1,
        GrainLevel::Light => 2,
        GrainLevel::Visible => 3,
        GrainLevel::Heavy => 4,
    };

    // Define all levels for easy indexing
    const ALL_LEVELS: [GrainLevel; 5] = [
        GrainLevel::VeryClean,
        GrainLevel::VeryLight,
        GrainLevel::Light,
        GrainLevel::Visible,
        GrainLevel::Heavy,
    ];
    const MAX_INDEX: usize = ALL_LEVELS.len() - 1;

    // Calculate lower and upper bounds, clamping to valid indices
    let lower_idx = median_index.saturating_sub(level_delta);
    let upper_idx = (median_index + level_delta).min(MAX_INDEX);

    // Convert indices back to GrainLevels using the array
    let lower_level = ALL_LEVELS[lower_idx];
    let upper_level = ALL_LEVELS[upper_idx];

    (lower_level, upper_level)
}


/// Generate refinement parameters between lower and upper bound levels.
/// Returns a vector of (GrainLevel, hqdn3d_params) pairs.
pub(super) fn generate_refinement_params(
    lower_level: GrainLevel,
    upper_level: GrainLevel,
    initial_params: &[(Option<GrainLevel>, Option<&str>)] // Pass initial params to avoid re-testing
) -> Vec<(Option<GrainLevel>, String)> {
    let mut refined_params = Vec::new();

    // Define all levels and their indices for easier logic
    const ALL_LEVELS: [(GrainLevel, usize); 5] = [
        (GrainLevel::VeryClean, 0),
        (GrainLevel::VeryLight, 1),
        (GrainLevel::Light, 2),
        (GrainLevel::Visible, 3),
        (GrainLevel::Heavy, 4),
    ];

    let lower_idx_opt = ALL_LEVELS.iter().find(|(l, _)| *l == lower_level).map(|(_, idx)| *idx);
    let upper_idx_opt = ALL_LEVELS.iter().find(|(l, _)| *l == upper_level).map(|(_, idx)| *idx);

    // Ensure both levels are valid and get indices
    let (lower_idx, upper_idx) = match (lower_idx_opt, upper_idx_opt) {
        (Some(l), Some(u)) if u > l => (l, u),
        _ => {
            log::debug!("Refinement bounds invalid or identical ({:?}, {:?}). No refinement needed.", lower_level, upper_level);
            return refined_params; // Invalid or identical bounds
        }
    };

    // Skip if bounds are adjacent (no intermediate levels to test)
    if upper_idx <= lower_idx + 1 {
         log::debug!("Refinement bounds are adjacent ({:?}, {:?}). No refinement needed.", lower_level, upper_level);
        return refined_params;
    }

    // Generate intermediate points between lower and upper bounds
    let range = upper_idx - lower_idx; // Total steps between bounds

    for i in 1..range { // Iterate through the steps *between* the bounds
        let interpolated_level_idx = lower_idx + i;
        let factor = i as f32 / range as f32; // Interpolation factor (0 to 1 exclusive)

        // Get the GrainLevel corresponding to this intermediate index
        let interpolated_level = match ALL_LEVELS.iter().find(|(_, idx)| *idx == interpolated_level_idx) {
            Some((level, _)) => *level,
            None => continue, // Should not happen with valid indices
        };

        // Skip if this exact level was already tested in the initial phase
        if initial_params.iter().any(|(level_opt, _)| level_opt == &Some(interpolated_level)) {
             log::trace!("Skipping refinement for level {:?} as it was in initial tests.", interpolated_level);
            continue;
        }

        // Interpolate parameters using the original bounds and the calculated factor
        let params = interpolate_hqdn3d_params(lower_level, upper_level, factor);
        refined_params.push((Some(interpolated_level), params));
    }

    refined_params
}