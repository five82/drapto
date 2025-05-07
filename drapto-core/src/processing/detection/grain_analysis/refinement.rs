// ============================================================================
// drapto-core/src/processing/detection/grain_analysis/refinement.rs
// ============================================================================
//
// ADAPTIVE REFINEMENT: Improved Grain Analysis Through Parameter Interpolation
//
// This module implements the adaptive refinement phase of grain analysis, which
// improves the accuracy of grain level detection by testing additional denoising
// parameters between the initial estimates. It uses interpolation to generate
// intermediate parameter values and statistical analysis to determine the
// appropriate refinement range.
//
// KEY COMPONENTS:
// - Parameter interpolation between grain levels
// - Statistical analysis of initial estimates
// - Adaptive refinement range calculation
// - Generation of refined test parameters
//
// AI-ASSISTANT-INFO: Adaptive refinement for grain analysis

// ---- Internal module imports ----
use super::constants::HQDN3D_PARAMS;
use super::types::GrainLevel;

// ============================================================================
// PARAMETER INTERPOLATION
// ============================================================================

/// Interpolates hqdn3d filter parameters between two predefined grain levels.
///
/// This function creates intermediate denoising parameters by linearly interpolating
/// between the parameters of two known grain levels. It's used to generate refined
/// test parameters for the adaptive refinement phase.
///
/// # Arguments
///
/// * `lower_level` - The lower grain level to interpolate from
/// * `upper_level` - The upper grain level to interpolate to
/// * `factor` - Interpolation factor (0.0 to 1.0) where 0.0 is equivalent to
///   lower_level and 1.0 is equivalent to upper_level
///
/// # Returns
///
/// * A string containing the interpolated hqdn3d filter parameters
///
/// # Example
///
/// ```
/// let params = interpolate_hqdn3d_params(GrainLevel::VeryLight, GrainLevel::Light, 0.5);
/// // Result might be something like "hqdn3d=0.8:0.5:3.5:3.5"
/// ```
fn interpolate_hqdn3d_params(lower_level: GrainLevel, upper_level: GrainLevel, factor: f32) -> String {
    // Get the parameter strings for the lower and upper grain levels
    let lower_params = HQDN3D_PARAMS.iter()
        .find(|(level, _)| *level == lower_level)
        .map(|(_, params)| *params)
        .unwrap_or("hqdn3d=0:0:0:0"); // Default if level not found (shouldn't happen)

    let upper_params = HQDN3D_PARAMS.iter()
        .find(|(level, _)| *level == upper_level)
        .map(|(_, params)| *params)
        .unwrap_or("hqdn3d=0:0:0:0"); // Default if level not found (shouldn't happen)

    // Parse the parameter strings into their numeric components
    let l_components = parse_hqdn3d_params(lower_params);
    let u_components = parse_hqdn3d_params(upper_params);

    // Linearly interpolate each component based on the factor
    let new_luma_spatial = l_components.0 + (u_components.0 - l_components.0) * factor;
    let new_chroma_spatial = l_components.1 + (u_components.1 - l_components.1) * factor;
    let new_luma_tmp = l_components.2 + (u_components.2 - l_components.2) * factor;
    let new_chroma_tmp = l_components.3 + (u_components.3 - l_components.3) * factor;

    // Format the interpolated parameters with controlled precision
    // Using one decimal place for consistency with predefined parameters
    format!("hqdn3d={:.1}:{:.1}:{:.1}:{:.1}",
        new_luma_spatial.max(0.0),  // Ensure values are non-negative
        new_chroma_spatial.max(0.0),
        new_luma_tmp.max(0.0),
        new_chroma_tmp.max(0.0)
    )
}

/// Parses an hqdn3d parameter string into its numeric components.
///
/// This helper function extracts the four numeric parameters from an hqdn3d filter string:
/// - luma spatial strength
/// - chroma spatial strength
/// - luma temporal strength
/// - chroma temporal strength
///
/// # Arguments
///
/// * `params` - The hqdn3d parameter string to parse (e.g., "hqdn3d=1.0:0.7:4.0:4.0")
///
/// # Returns
///
/// * A tuple containing the four components as f32 values:
///   (luma_spatial, chroma_spatial, luma_tmp, chroma_tmp)
/// * Returns (0.0, 0.0, 0.0, 0.0) if parsing fails
///
/// # Format
///
/// The expected format is "hqdn3d=y:cb:cr:strength" where:
/// - y: Luma spatial strength (higher values = more denoising)
/// - cb: Chroma spatial strength
/// - cr: Temporal strength
/// - strength: Temporal chroma strength
fn parse_hqdn3d_params(params: &str) -> (f32, f32, f32, f32) {
    // Default values to return if parsing fails
    const DEFAULT_PARAMS: (f32, f32, f32, f32) = (0.0, 0.0, 0.0, 0.0);

    // Step 1: Check for the "hqdn3d=" prefix
    if let Some(values_str) = params.strip_prefix("hqdn3d=") {
        // Step 2: Split the string by colons to get the four components
        let parts: Vec<_> = values_str.split(':').collect();

        // Step 3: Verify we have exactly 4 components
        if parts.len() == 4 {
            // Step 4: Parse each component as an f32
            let parsed_parts: Vec<Result<f32, _>> = parts.iter().map(|s| s.parse()).collect();

            // Step 5: Check if all components parsed successfully
            if parsed_parts.iter().all(Result::is_ok) {
                // All parts parsed successfully - return the values
                // Ensure all values are non-negative
                return (
                    parsed_parts[0].as_ref().unwrap().max(0.0),
                    parsed_parts[1].as_ref().unwrap().max(0.0),
                    parsed_parts[2].as_ref().unwrap().max(0.0),
                    parsed_parts[3].as_ref().unwrap().max(0.0),
                );
            } else {
                // Log specific parsing errors for debugging
                for (i, res) in parsed_parts.iter().enumerate() {
                    if let Err(e) = res {
                        log::warn!(
                            "Failed to parse hqdn3d component #{} ('{}') in '{}': {}. Using default.",
                            i+1, parts[i], params, e
                        );
                    }
                }
            }
        } else {
            // Wrong number of components
            log::warn!(
                "Incorrect number of components ({}) in hqdn3d params: '{}'. Expected 4. Using defaults.",
                parts.len(), params
            );
        }
    } else {
        // Missing prefix
        log::warn!(
            "Invalid hqdn3d param string format (missing 'hqdn3d=' prefix): '{}'. Using defaults.",
            params
        );
    }

    // Return defaults if any parsing step failed
    DEFAULT_PARAMS
}

// ============================================================================
// STATISTICAL ANALYSIS
// ============================================================================

/// Calculates the standard deviation of grain level estimates.
///
/// This function converts grain levels to numeric values and calculates
/// the standard deviation, which is used to determine the width of the
/// refinement range. A higher standard deviation indicates more variability
/// in the initial estimates and suggests a wider refinement range is needed.
///
/// # Arguments
///
/// * `levels` - A slice of GrainLevel values to analyze
///
/// # Returns
///
/// * The standard deviation as an f64 value
/// * Returns 0.0 if there are fewer than 2 levels
///
/// # Note
///
/// This implementation uses the population standard deviation (dividing by n)
/// rather than the sample standard deviation (dividing by n-1).
fn calculate_std_dev(levels: &[GrainLevel]) -> f64 {
    // Return 0 for empty or single-element arrays
    if levels.len() <= 1 {
        return 0.0;
    }

    // Step 1: Convert GrainLevel enum values to numeric values
    let numeric_values: Vec<f64> = levels.iter().map(|level| match level {
        GrainLevel::VeryClean => 0.0,
        GrainLevel::VeryLight => 1.0,
        GrainLevel::Light => 2.0,
        GrainLevel::Visible => 3.0,
        GrainLevel::Heavy => 4.0,
    }).collect();

    // Step 2: Calculate the mean of the numeric values
    let mean: f64 = numeric_values.iter().sum::<f64>() / numeric_values.len() as f64;

    // Step 3: Calculate the variance
    // Using population variance (n) rather than sample variance (n-1)
    let variance: f64 = numeric_values.iter()
        .map(|val| (val - mean).powi(2))  // Square of difference from mean
        .sum::<f64>() / numeric_values.len() as f64;  // Divide by n

    // Step 4: Return the square root of variance (standard deviation)
    variance.sqrt()
}

// ============================================================================
// REFINEMENT RANGE CALCULATION
// ============================================================================

/// Determines the optimal refinement range based on initial grain level estimates.
///
/// This function analyzes the initial grain level estimates to determine an
/// appropriate range for the refinement phase. It uses the median and standard
/// deviation of the estimates to calculate a range that's wide enough to capture
/// the optimal grain level but narrow enough to be efficient.
///
/// # Algorithm
///
/// 1. Find the median of the initial estimates
/// 2. Calculate the standard deviation of the estimates
/// 3. Determine a level delta based on the standard deviation
/// 4. Calculate lower and upper bounds by extending from the median by the level delta
///
/// # Arguments
///
/// * `initial_estimates` - A slice of GrainLevel values from the initial analysis
///
/// # Returns
///
/// * A tuple containing (lower_bound, upper_bound) as GrainLevel values
/// * Returns (VeryClean, VeryLight) if the input is empty
pub(super) fn calculate_refinement_range(initial_estimates: &[GrainLevel]) -> (GrainLevel, GrainLevel) {
    // Handle empty input case
    if initial_estimates.is_empty() {
        // This shouldn't happen if the function is called correctly
        log::warn!("calculate_refinement_range called with empty estimates. Defaulting range.");
        return (GrainLevel::VeryClean, GrainLevel::VeryLight);
    }

    // Step 1: Find the median of the initial estimates
    // Create a copy and sort for median calculation
    let mut sorted_estimates = initial_estimates.to_vec();
    sorted_estimates.sort(); // GrainLevel implements Ord

    // Get the median value (lower median for even-length arrays)
    let median_idx = sorted_estimates.len() / 2;
    let median = sorted_estimates[median_idx];

    // Step 2: Calculate the standard deviation to determine range width
    let std_dev = calculate_std_dev(initial_estimates);

    // Step 3: Calculate the level delta (number of levels to extend in each direction)
    // Use an adaptive factor to scale the standard deviation
    const ADAPTIVE_FACTOR: f64 = 1.5; // Scaling factor for standard deviation

    // Round to nearest integer and ensure at least 1 level difference
    let level_delta = (std_dev * ADAPTIVE_FACTOR).round().max(1.0) as usize;

    // Step 4: Convert the median GrainLevel to a numeric index
    let median_index: usize = match median {
        GrainLevel::VeryClean => 0,
        GrainLevel::VeryLight => 1,
        GrainLevel::Light => 2,
        GrainLevel::Visible => 3,
        GrainLevel::Heavy => 4,
    };

    // Define all grain levels in order for easy indexing
    const ALL_LEVELS: [GrainLevel; 5] = [
        GrainLevel::VeryClean,
        GrainLevel::VeryLight,
        GrainLevel::Light,
        GrainLevel::Visible,
        GrainLevel::Heavy,
    ];
    const MAX_INDEX: usize = ALL_LEVELS.len() - 1;

    // Step 5: Calculate lower and upper bound indices
    // Use saturating_sub to avoid underflow and min to avoid overflow
    let lower_idx = median_index.saturating_sub(level_delta);
    let upper_idx = (median_index + level_delta).min(MAX_INDEX);

    // Step 6: Convert indices back to GrainLevels
    let lower_level = ALL_LEVELS[lower_idx];
    let upper_level = ALL_LEVELS[upper_idx];

    // Return the calculated refinement range
    (lower_level, upper_level)
}


// ============================================================================
// REFINEMENT PARAMETER GENERATION
// ============================================================================

/// Generates refined denoising parameters between lower and upper bound levels.
///
/// This function creates intermediate denoising parameters by interpolating
/// between the lower and upper bound levels determined by the refinement range
/// calculation. It skips levels that were already tested in the initial phase
/// to avoid redundant testing.
///
/// # Algorithm
///
/// 1. Validate the lower and upper bound levels
/// 2. Calculate the number of steps between the bounds
/// 3. For each intermediate step:
///    a. Calculate the interpolation factor
///    b. Find the corresponding grain level
///    c. Skip if already tested in initial phase
///    d. Interpolate parameters between lower and upper bounds
///
/// # Arguments
///
/// * `lower_level` - The lower bound grain level
/// * `upper_level` - The upper bound grain level
/// * `initial_params` - Parameters already tested in the initial phase
///
/// # Returns
///
/// * A vector of (Option<GrainLevel>, String) pairs containing the refined parameters
/// * Returns an empty vector if no refinement is needed
pub(super) fn generate_refinement_params(
    lower_level: GrainLevel,
    upper_level: GrainLevel,
    initial_params: &[(Option<GrainLevel>, Option<&str>)]
) -> Vec<(Option<GrainLevel>, String)> {
    // Initialize vector to store refined parameters
    let mut refined_params = Vec::new();

    // Define all grain levels with their corresponding indices
    const ALL_LEVELS: [(GrainLevel, usize); 5] = [
        (GrainLevel::VeryClean, 0),
        (GrainLevel::VeryLight, 1),
        (GrainLevel::Light, 2),
        (GrainLevel::Visible, 3),
        (GrainLevel::Heavy, 4),
    ];

    // Step 1: Find the indices of the lower and upper bound levels
    let lower_idx_opt = ALL_LEVELS.iter().find(|(l, _)| *l == lower_level).map(|(_, idx)| *idx);
    let upper_idx_opt = ALL_LEVELS.iter().find(|(l, _)| *l == upper_level).map(|(_, idx)| *idx);

    // Step 2: Validate the bounds and get their indices
    let (lower_idx, upper_idx) = match (lower_idx_opt, upper_idx_opt) {
        (Some(l), Some(u)) if u > l => (l, u),
        _ => {
            // Invalid or identical bounds - no refinement needed
            log::debug!(
                "Refinement bounds invalid or identical ({:?}, {:?}). No refinement needed.",
                lower_level, upper_level
            );
            return refined_params;
        }
    };

    // Step 3: Check if bounds are adjacent (no intermediate levels to test)
    if upper_idx <= lower_idx + 1 {
        log::debug!(
            "Refinement bounds are adjacent ({:?}, {:?}). No refinement needed.",
            lower_level, upper_level
        );
        return refined_params;
    }

    // Step 4: Generate intermediate points between lower and upper bounds
    let range = upper_idx - lower_idx; // Total steps between bounds

    // Iterate through each intermediate step
    for i in 1..range {
        // Calculate the index of this intermediate level
        let interpolated_level_idx = lower_idx + i;

        // Calculate interpolation factor (0 to 1 exclusive)
        let factor = i as f32 / range as f32;

        // Step 5: Find the grain level corresponding to this intermediate index
        let interpolated_level = match ALL_LEVELS.iter().find(|(_, idx)| *idx == interpolated_level_idx) {
            Some((level, _)) => *level,
            None => continue, // Should not happen with valid indices
        };

        // Step 6: Skip if this level was already tested in the initial phase
        if initial_params.iter().any(|(level_opt, _)| level_opt == &Some(interpolated_level)) {
            log::trace!(
                "Skipping refinement for level {:?} as it was in initial tests.",
                interpolated_level
            );
            continue;
        }

        // Step 7: Generate interpolated parameters for this level
        let params = interpolate_hqdn3d_params(lower_level, upper_level, factor);

        // Add to the list of refined parameters
        refined_params.push((Some(interpolated_level), params));
    }

    refined_params
}