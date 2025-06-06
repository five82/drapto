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
use crate::progress_reporting::{LogLevel, report_log_message};

// ---- External crate imports ----
use log;

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
/// 1. Establish a baseline using "Baseline" (no grain) or fallback to Baseline if necessary
/// 2. Calculate efficiency for each grain level: (size_reduction / sqrt(grain_level_value))
/// 3. Find the maximum efficiency point
/// 4. Set a threshold at knee_threshold * max_efficiency (e.g., 80% of max)
/// 5. Select the lowest grain level that meets or exceeds this threshold
///
/// The square root scaling in the efficiency calculation reduces bias against higher
/// denoising levels, providing a more balanced assessment. The algorithm always uses
/// "Baseline" as the baseline for comparison to ensure accurate results.
///
/// # Arguments
///
/// * `results` - HashMap mapping grain levels to their encoded file sizes in bytes
/// * `knee_threshold` - Threshold factor (0.0-1.0) for determining the knee point
/// * `sample_index` - The index of the sample being analyzed (for logging)
///
/// # Returns
///
/// * The optimal `GrainLevel` based on the knee point analysis
/// * `GrainLevel::Baseline` if no suitable level is found or analysis fails
pub(super) fn analyze_sample_with_knee_point(
    results: &HashMap<Option<GrainLevel>, u64>,
    knee_threshold: f64,
    sample_index: usize,
) -> GrainLevel {
    // ========================================================================
    // STEP 1: GET BASELINE SIZE (NO DENOISING)
    // ========================================================================

    // Get the baseline file size (with no denoising applied)
    // This is used as the reference point for calculating size reductions
    // First try None (no denoising), then fall back to Baseline if None is missing
    let baseline_size = match results.get(&None) {
        Some(&size) if size > 0 => {
            // Report baseline usage in verbose mode
            log::debug!("Sample {}: Using 'Baseline' (no denoising) for knee point analysis.", sample_index);
            crate::progress_reporting::report_sub_item(&format!(
                "Sample {}: Using 'Baseline' (no denoising) for knee point analysis.",
                sample_index
            ));
            size
        }
        _ => {
            // If None is missing or zero, try Baseline as fallback
            match results.get(&Some(GrainLevel::Baseline)) {
                Some(&size) if size > 0 => {
                    report_log_message(
                        &format!(
                            "  Sample {}: Baseline 'None' missing or zero. Using 'Baseline' as fallback.",
                            sample_index
                        ),
                        LogLevel::Info,
                    );
                    size
                }
                _ => {
                    // If both None and Baseline are missing or zero, we can't perform the analysis
                    report_log_message(
                        &format!(
                            "  Sample {}: ERROR: 'Baseline' reference is missing or zero. Cannot analyze with knee point.",
                            sample_index
                        ),
                        LogLevel::Error,
                    );
                    return GrainLevel::Baseline; // Return default value
                }
            }
        }
    };

    // ========================================================================
    // STEP 2: DEFINE GRAIN LEVEL MAPPING FUNCTION
    // ========================================================================

    // Map GrainLevel enum variants to numerical values for calculations
    // This allows us to quantify the "strength" of each denoising level
    let to_numeric = |level: Option<GrainLevel>| -> f64 {
        match level {
            None => 0.0,                        // No denoising
            Some(GrainLevel::Baseline) => 0.0,  // Equivalent to no denoising
            Some(GrainLevel::VeryLight) => 1.0, // Very light denoising
            Some(GrainLevel::Light) => 2.0,     // Light denoising
            Some(GrainLevel::Moderate) => 3.0,  // Moderate denoising
            Some(GrainLevel::Elevated) => 4.0,  // Elevated denoising
        }
    };

    // ========================================================================
    // STEP 3: CALCULATE EFFICIENCY METRICS
    // ========================================================================

    // Initialize vector to store efficiency metrics for each grain level
    let mut efficiencies: Vec<(Option<GrainLevel>, f64)> = Vec::new();

    // Process each grain level and calculate its efficiency
    // Filter to only include valid grain levels with non-zero file sizes
    // Note: We include all levels (including None and Baseline) in the iteration
    // but will filter appropriately below
    for (&level, &size) in results.iter().filter(|&(_, &v)| v > 0) {
        // Convert grain level to numeric value
        let grain_numeric = to_numeric(level);

        // Skip Baseline level as it represents no denoising
        // These are our reference points, not candidates for selection
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

    // If no levels provided positive efficiency, return Baseline (no denoising)
    if efficiencies.is_empty() {
        report_log_message(
            &format!(
                "  Sample {}: No positive efficiency improvements found with knee point analysis.",
                sample_index
            ),
            LogLevel::Info,
        );
        return GrainLevel::Baseline;
    }

    // ========================================================================
    // STEP 4: FIND MAXIMUM EFFICIENCY POINT
    // ========================================================================

    // Find the grain level with the highest efficiency
    let (max_level, max_efficiency) =
        efficiencies.iter().fold((None, 0.0), |acc, &(level, eff)| {
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
        report_log_message(
            &format!(
                "  Sample {}: Max efficiency is not positive (Max: {:.2}). Using Baseline.",
                sample_index, max_efficiency
            ),
            LogLevel::Info,
        );
        return GrainLevel::Baseline;
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
        a_val
            .partial_cmp(&b_val)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Choose the lowest grain level that meets the threshold
    // This is the "knee point" - the optimal balance between denoising and quality
    if let Some(&(Some(level), _)) = candidates.first() {
        // Report the analysis results as sub-items in verbose mode
        log::debug!(
            "Sample {}: Knee point analysis: Max efficiency {:.2} at level {:?}. Threshold {:.1}%. Choosing: {:?}",
            sample_index,
            max_efficiency,
            max_level.unwrap_or(GrainLevel::Baseline), // Default for logging
            knee_threshold * 100.0,
            level
        );
        crate::progress_reporting::report_sub_item(&format!(
            "Sample {}: Knee point analysis: Max efficiency {:.2} at level {:?}. Threshold {:.1}%. Choosing: {:?}",
            sample_index,
            max_efficiency,
            max_level.unwrap_or(GrainLevel::Baseline), // Default for logging
            knee_threshold * 100.0,
            level
        ));

        // Return the selected optimal grain level
        level
    } else {
        // No suitable candidates found - report in verbose mode
        log::debug!("Sample {}: No suitable candidates found in knee point analysis. Using Baseline.", sample_index);
        crate::progress_reporting::report_sub_item(&format!(
            "Sample {}: No suitable candidates found in knee point analysis. Using Baseline.",
            sample_index
        ));

        // Default to no denoising
        GrainLevel::Baseline
    }
}
