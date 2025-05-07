// ============================================================================
// drapto-core/src/processing/detection/grain_analysis/knee_point.rs
// ============================================================================
//
// KNEE POINT ANALYSIS: Optimal Grain Level Detection Algorithm
//
// This module implements the knee point analysis algorithm for determining the
// optimal grain level for denoising. The algorithm works by finding the point
// of diminishing returns in the efficiency curve of denoising strength vs.
// file size reduction.
//
// The key insight is that there's typically a "knee point" in this curve where
// additional denoising strength provides minimal additional file size reduction
// but may degrade visual quality. This algorithm identifies that point to
// balance compression efficiency with visual quality preservation.
//
// AI-ASSISTANT-INFO: Knee point analysis for optimal denoising parameter selection

// ---- Internal module imports ----
use super::types::GrainLevel;

// ---- Standard library imports ----
use std::collections::HashMap;

/// Analyzes a sample's encoding results to determine the optimal grain level using knee point analysis.
///
/// This function implements the knee point detection algorithm, which finds the point of
/// diminishing returns in the efficiency curve of denoising strength vs. file size reduction.
/// It calculates an efficiency metric for each grain level and selects the level that provides
/// the best balance between compression efficiency and visual quality preservation.
///
/// # Algorithm Overview
///
/// 1. Calculate efficiency for each grain level: (size_reduction / sqrt(grain_level_value))
/// 2. Find the maximum efficiency point
/// 3. Set a threshold at knee_threshold * max_efficiency (e.g., 80% of max)
/// 4. Select the lowest grain level that meets or exceeds this threshold
///
/// The square root scaling in the efficiency calculation reduces bias against higher
/// denoising levels, providing a more balanced assessment.
///
/// # Arguments
///
/// * `results` - HashMap mapping grain levels to their encoded file sizes in bytes
/// * `knee_threshold` - Threshold factor (0.0-1.0) for determining the knee point
/// * `log_callback` - Function for logging analysis results
///
/// # Returns
///
/// * The optimal `GrainLevel` based on the knee point analysis
/// * `GrainLevel::VeryClean` if no suitable level is found or analysis fails
pub(super) fn analyze_sample_with_knee_point(
    results: &HashMap<Option<GrainLevel>, u64>,
    knee_threshold: f64,
    log_callback: &mut impl Fn(&str)
) -> GrainLevel {
    // ========================================================================
    // STEP 1: GET BASELINE SIZE (NO DENOISING)
    // ========================================================================

    // Get the baseline file size (with no denoising applied)
    // This is used as the reference point for calculating size reductions
    let baseline_size = match results.get(&None) {
        Some(&size) => size,
        None => {
            // If baseline size is missing, we can't perform the analysis
            log_callback("Baseline encode size missing from results. Cannot analyze with knee point.");
            return GrainLevel::VeryClean; // Return default value
        }
    };

    // ========================================================================
    // STEP 2: DEFINE GRAIN LEVEL MAPPING FUNCTION
    // ========================================================================

    // Map GrainLevel enum variants to numerical values for calculations
    // This allows us to quantify the "strength" of each denoising level
    let to_numeric = |level: Option<GrainLevel>| -> f64 {
        match level {
            None => 0.0,                         // No denoising
            Some(GrainLevel::VeryClean) => 0.0,  // Equivalent to no denoising
            Some(GrainLevel::VeryLight) => 1.0,  // Very light denoising
            Some(GrainLevel::Light) => 2.0,      // Light denoising
            Some(GrainLevel::Visible) => 3.0,    // Medium denoising
            Some(GrainLevel::Heavy) => 4.0,      // Strong denoising
        }
    };

    // ========================================================================
    // STEP 3: CALCULATE EFFICIENCY METRICS
    // ========================================================================

    // Initialize vector to store efficiency metrics for each grain level
    let mut efficiencies: Vec<(Option<GrainLevel>, f64)> = Vec::new();

    // Process each grain level and calculate its efficiency
    // Filter to only include valid grain levels with non-zero file sizes
    for (&level, &size) in results.iter().filter(|&(&k, &v)| v > 0 && k.is_some()) {
        // Convert grain level to numeric value
        let grain_numeric = to_numeric(level);

        // Skip baseline (None) or VeryClean levels as they represent no denoising
        if grain_numeric <= 0.0 {
            continue;
        }

        // Calculate size reduction compared to baseline
        // Use saturating_sub to avoid underflow if size > baseline_size
        let reduction = baseline_size.saturating_sub(size) as f64;

        // Skip levels that don't reduce file size
        if reduction <= 0.0 {
            continue;
        }

        // Calculate efficiency using square-root scaling
        // This reduces bias against higher denoising levels by making the
        // denominator grow more slowly than the level value
        let adjusted_denom = grain_numeric.sqrt();
        let efficiency = reduction / adjusted_denom;

        // Only include positive, finite efficiency values
        if efficiency > 0.0 && efficiency.is_finite() {
            efficiencies.push((level, efficiency));
        }
    }

    // If no levels provided positive efficiency, return VeryClean (no denoising)
    if efficiencies.is_empty() {
        log_callback("No positive efficiency improvements found with knee point analysis.");
        return GrainLevel::VeryClean;
    }

    // ========================================================================
    // STEP 4: FIND MAXIMUM EFFICIENCY POINT
    // ========================================================================

    // Find the grain level with the highest efficiency
    let (max_level, max_efficiency) = efficiencies.iter()
        .fold((None, 0.0), |acc, &(level, eff)| {
            // Update if this efficiency is higher than the current maximum
            // and is a valid finite number
            if eff > acc.1 && eff.is_finite() {
                (level, eff)
            } else {
                acc
            }
        });

    // Safety check: ensure maximum efficiency is positive
    if max_efficiency <= 0.0 {
        log_callback(&format!(
            "Max efficiency is not positive (Max: {:.2}). Using VeryClean.",
            max_efficiency
        ));
        return GrainLevel::VeryClean;
    }

    // ========================================================================
    // STEP 5: APPLY KNEE THRESHOLD
    // ========================================================================

    // Calculate the threshold efficiency (e.g., 80% of maximum)
    let threshold_efficiency = knee_threshold * max_efficiency;

    // Find all grain levels that meet or exceed the threshold
    let mut candidates: Vec<(Option<GrainLevel>, f64)> = efficiencies
        .into_iter()
        .filter(|&(_, eff)| eff.is_finite() && eff >= threshold_efficiency)
        .collect();

    // ========================================================================
    // STEP 6: SORT AND SELECT OPTIMAL LEVEL
    // ========================================================================

    // Sort candidates by grain level value (lowest first)
    // This prioritizes lighter denoising levels that still meet the threshold
    candidates.sort_by(|a, b| {
        let a_val = to_numeric(a.0);
        let b_val = to_numeric(b.0);
        a_val.partial_cmp(&b_val).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Choose the lowest grain level that meets the threshold
    // This is the "knee point" - the optimal balance between denoising and quality
    if let Some(&(Some(level), _)) = candidates.first() {
        // Log the analysis results
        log_callback(&format!(
            "Knee point analysis: Max efficiency {:.2} at level {:?}. Threshold {:.1}%. Choosing: {:?}",
            max_efficiency,
            max_level.unwrap_or(GrainLevel::VeryClean), // Default for logging
            knee_threshold * 100.0,
            level
        ));

        // Return the selected optimal grain level
        level
    } else {
        // No suitable candidates found
        log_callback(&format!(
            "No suitable candidates found in knee point analysis. Using VeryClean."
        ));

        // Default to no denoising
        GrainLevel::VeryClean
    }
}