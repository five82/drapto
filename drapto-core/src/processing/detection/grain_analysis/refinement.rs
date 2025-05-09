// ============================================================================
// drapto-core/src/processing/detection/grain_analysis/refinement.rs
// ============================================================================
//
// ADAPTIVE REFINEMENT: Improved Grain Analysis Through Direct Parameter Testing
//
// This module implements the adaptive refinement phase of grain analysis, which
// improves the accuracy of grain level detection by testing additional denoising
// parameters between the initial estimates. It uses direct parameter testing of
// intermediate grain levels and statistical analysis to determine the appropriate
// refinement range.
//
// KEY COMPONENTS:
// - Statistical analysis of initial estimates
// - Adaptive refinement range calculation based on standard deviation
// - Direct testing of intermediate grain levels
// - Type-safe parameter selection using predefined values
//
// AI-ASSISTANT-INFO: Adaptive refinement for grain analysis

// ---- Internal module imports ----
use super::types::GrainLevel;

// ============================================================================
// STATISTICAL ANALYSIS
// ============================================================================

/// Calculates the standard deviation of grain level estimates.
///
/// This function converts grain levels to numeric values using the grain_level_to_strength
/// utility function and calculates the standard deviation, which is used to determine
/// the width of the refinement range. A higher standard deviation indicates more variability
/// in the initial estimates and suggests a wider refinement range is needed.
///
/// # Arguments
///
/// * `levels` - A slice of GrainLevel values to analyze
///
/// # Returns
///
/// * Option<f64> containing the standard deviation, or None if calculation fails
///   - Returns None for insufficient data (fewer than 2 levels)
///   - Returns None for non-finite results (NaN or infinite)
///
/// # Note
///
/// This implementation uses the population standard deviation (dividing by n)
/// rather than the sample standard deviation (dividing by n-1).
fn calculate_std_dev(levels: &[GrainLevel]) -> Option<f64> {
    // Return None for insufficient data
    if levels.len() <= 1 {
        return None;
    }

    // Import the utility function
    use super::utils::grain_level_to_strength;

    // Step 1: Convert GrainLevel enum values to numeric values using the utility function
    let numeric_values: Vec<f64> = levels.iter()
        .map(|&level| grain_level_to_strength(level) as f64)
        .collect();

    // Step 2: Calculate the mean of the numeric values
    let mean: f64 = numeric_values.iter().sum::<f64>() / numeric_values.len() as f64;

    // Step 3: Calculate the variance
    // Using population variance (n) rather than sample variance (n-1)
    let variance: f64 = numeric_values.iter()
        .map(|val| (val - mean).powi(2))  // Square of difference from mean
        .sum::<f64>() / numeric_values.len() as f64;  // Divide by n

    // Step 4: Calculate the standard deviation and check for valid result
    let std_dev = variance.sqrt();

    // Return None for non-finite values (NaN or infinite)
    if !std_dev.is_finite() {
        return None;
    }

    Some(std_dev)
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
/// * Returns (Baseline, VeryLight) if the input is empty
pub(super) fn calculate_refinement_range(initial_estimates: &[GrainLevel]) -> (GrainLevel, GrainLevel) {
    // Handle empty input case
    if initial_estimates.is_empty() {
        // This shouldn't happen if the function is called correctly
        log::warn!("calculate_refinement_range called with empty estimates. Defaulting range.");
        return (GrainLevel::Baseline, GrainLevel::VeryLight);
    }

    // Step 1: Find the median of the initial estimates
    // Create a copy and sort for median calculation
    let mut sorted_estimates = initial_estimates.to_vec();
    sorted_estimates.sort(); // GrainLevel implements Ord

    // Get the median value (lower median for even-length arrays)
    let median_idx = sorted_estimates.len() / 2;
    let median = sorted_estimates[median_idx];

    // Step 2: Calculate the standard deviation to determine range width
    // Step 3: Calculate the level delta (number of levels to extend in each direction)
    const ADAPTIVE_FACTOR: f64 = 1.5; // Scaling factor for standard deviation
    const MIN_DELTA: usize = 1;       // Minimum level difference
    const DEFAULT_DELTA: usize = 2;   // Default level difference if std_dev calculation fails

    // Calculate adaptive delta based on standard deviation with improved handling
    let level_delta = match calculate_std_dev(initial_estimates) {
        Some(std_dev) if std_dev > 0.0 => {
            // Valid non-zero std dev - scale by adaptive factor
            let scaled_delta = (std_dev * ADAPTIVE_FACTOR).round() as usize;
            scaled_delta.max(MIN_DELTA) // Ensure at least MIN_DELTA
        },
        Some(0.0) => {
            // All estimates identical - use minimal delta
            log::debug!("Standard deviation is zero (all estimates identical). Using minimal delta.");
            MIN_DELTA
        },
        _ => {
            // Failed calculation or invalid result - use default
            log::debug!("Standard deviation calculation failed. Using default delta.");
            DEFAULT_DELTA
        }
    };

    // Step 4: Convert the median GrainLevel to a numeric index
    let median_index: usize = match median {
        GrainLevel::Baseline => 0,
        GrainLevel::VeryLight => 1,
        GrainLevel::Light => 2,
        GrainLevel::Moderate => 3,
        GrainLevel::Elevated => 4,
    };

    // Define all grain levels in order for easy indexing
    const ALL_LEVELS: [GrainLevel; 5] = [
        GrainLevel::Baseline,
        GrainLevel::VeryLight,
        GrainLevel::Light,
        GrainLevel::Moderate,
        GrainLevel::Elevated,
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
/// This function creates intermediate test points between the lower and upper
/// bounds using continuous parameter interpolation. This allows for more precise
/// grain analysis by testing a wider range of denoising strengths than just the
/// predefined GrainLevel enum values.
///
/// # Algorithm
///
/// 1. Convert GrainLevels to continuous strength values
/// 2. Calculate the range size and determine number of test points
/// 3. Generate evenly spaced test points within the range
/// 4. For each test point:
///    a. Generate interpolated hqdn3d parameters
///    b. Skip if parameters match any already tested in initial phase
///    c. Add to the list of refined parameters
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

    // Import utility functions
    use super::utils::{grain_level_to_strength, generate_hqdn3d_params};

    // Convert GrainLevels to continuous strength values
    let lower_f = grain_level_to_strength(lower_level);
    let upper_f = grain_level_to_strength(upper_level);

    // Skip if range is too small
    if upper_f <= lower_f + 0.1 {
        log::debug!(
            "Refinement bounds are too close ({:.2}, {:.2}). No refinement needed.",
            lower_f, upper_f
        );
        return refined_params;
    }

    // Generate 3-5 intermediate points based on range size
    // Larger ranges get more test points for better coverage
    let num_points = ((upper_f - lower_f) * 2.0).round().clamp(3.0, 5.0) as usize;
    let step = (upper_f - lower_f) / (num_points as f32 + 1.0);

    log::debug!(
        "Generating {} intermediate test points between {:.2} and {:.2} with step {:.2}",
        num_points, lower_f, upper_f, step
    );

    // Extract all existing parameter strings from initial tests for comparison
    let existing_params: Vec<String> = initial_params.iter()
        .filter_map(|(_, params_opt)| params_opt.map(|p| p.to_string()))
        .collect();

    // Generate and add intermediate test points
    for i in 1..=num_points {
        let point_f = lower_f + step * (i as f32);

        // Generate the hqdn3d parameters for this interpolated point
        let hqdn3d_params = generate_hqdn3d_params(point_f);

        // Skip if empty (shouldn't happen in this range) or if this exact parameter string already exists
        if hqdn3d_params.is_empty() || existing_params.contains(&hqdn3d_params) {
            log::debug!(
                "Skipping refinement point {:.2} as it produced empty params or duplicates existing test.",
                point_f
            );
            continue;
        }

        // Log before we move the string
        log::debug!(
            "Added refinement point at strength {:.2} with params: {}",
            point_f, &hqdn3d_params
        );

        // Add the interpolated test point to refined parameters
        // Use None for GrainLevel since this is an interpolated point that doesn't directly
        // correspond to a GrainLevel enum variant
        refined_params.push((None, hqdn3d_params));
    }



    refined_params
}