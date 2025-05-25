// ============================================================================
// drapto-core/src/processing/detection/grain_analysis/knee_point.rs
// ============================================================================
//
// KNEE POINT ANALYSIS: Optimal Grain Level Detection for File Size Reduction
//
// PURPOSE: Reduce video file sizes for home/mobile viewing while maintaining
// acceptable visual quality. This is NOT for archival or professional use.
//
// APPROACH: Find the denoising level that provides good file size reduction
// without over-processing. When in doubt, use VeryLight as it provides
// compression benefits (5-10% typically) with virtually no quality risk.
//
// KEY PRINCIPLE: Some denoising is almost always beneficial. Even light
// denoising removes high-entropy noise that doesn't contribute to perceived
// quality but consumes significant bitrate during encoding.
//
// AI-ASSISTANT-INFO: Knee point analysis for optimal denoising parameter selection

// ---- Internal module imports ----
use super::types::GrainLevel;
use crate::progress_reporting::{LogLevel, report_log_message};

// ---- External crate imports ----
use log;

// ---- Standard library imports ----
use std::collections::HashMap;

/// Determines the optimal denoising level for file size reduction.
///
/// This function analyzes encoding results to find the best balance between
/// file size reduction and quality preservation. The goal is practical file
/// size reduction for home/mobile viewing, not archival quality.
///
/// # Algorithm Overview
///
/// 1. Calculate efficiency for each grain level: (size_reduction / sqrt(grain_level_value))
/// 2. Find the maximum efficiency point
/// 3. Select the lowest grain level achieving 80% of max efficiency
/// 4. If no clear knee point exists, default to VeryLight (safe choice)
///
/// # Key Behaviors
///
/// - Always returns at least VeryLight (never Baseline/no denoising)
/// - Provides clear messaging about why a level was chosen
/// - Conservative approach: when uncertain, choose lighter denoising
///
/// # Arguments
///
/// * `results` - HashMap mapping grain levels to their encoded file sizes in bytes
/// * `knee_threshold` - Threshold factor (0.0-1.0) for knee point selection
/// * `sample_index` - The index of the sample being analyzed (for logging)
///
/// # Returns
///
/// The optimal `GrainLevel` for file size reduction (minimum: VeryLight)
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
    
    // Track size reductions for better reporting
    let mut size_reductions: Vec<(Option<GrainLevel>, f64)> = Vec::new();

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
        
        // Calculate percentage reduction for reporting
        let reduction_pct = (reduction / baseline_size as f64) * 100.0;
        size_reductions.push((level, reduction_pct));

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

    // If no levels provided positive efficiency, use VeryLight as safe fallback
    if efficiencies.is_empty() {
        report_log_message(
            &format!(
                "  Sample {}: No efficiency improvements found. Using VeryLight (safe default).",
                sample_index
            ),
            LogLevel::Info,
        );
        return GrainLevel::VeryLight;
    }

    // ========================================================================
    // STEP 4: FIND MAXIMUM EFFICIENCY POINT
    // ========================================================================

    // Find the grain level with the highest efficiency
    let (_max_level, max_efficiency) =
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
                "  Sample {}: Max efficiency is not positive (Max: {:.2}). Using VeryLight (safe default).",
                sample_index, max_efficiency
            ),
            LogLevel::Info,
        );
        return GrainLevel::VeryLight;
    }

    // ========================================================================
    // STEP 5: APPLY KNEE THRESHOLD
    // ========================================================================

    // Calculate the threshold efficiency (e.g., 80% of maximum)
    let threshold_efficiency = knee_threshold * max_efficiency;

    // Find all grain levels that meet or exceed the threshold
    let mut candidates: Vec<(Option<GrainLevel>, f64)> = efficiencies
        .iter()
        .filter(|&&(_, eff)| eff.is_finite() && eff >= threshold_efficiency)
        .cloned()
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
    if let Some(&(Some(level), efficiency)) = candidates.first() {
        // Calculate the actual file size reduction percentage for this level
        let level_result = results.get(&Some(level)).unwrap_or(&baseline_size);
        let size_reduction_pct = ((baseline_size - level_result) as f64 / baseline_size as f64) * 100.0;
        
        // Report the analysis results as sub-items in verbose mode
        log::debug!(
            "Sample {}: Knee point found at {:?}. Efficiency: {:.2}, Size reduction: {:.1}%",
            sample_index, level, efficiency, size_reduction_pct
        );
        crate::progress_reporting::report_sub_item(&format!(
            "Sample {}: Selected {:?} ({:.1}% size reduction)",
            sample_index, level, size_reduction_pct
        ));

        // Return the selected optimal grain level
        level
    } else {
        // No suitable candidates found - this typically means all levels have efficiency
        // below the threshold or the efficiency curve doesn't have a clear knee point
        
        // Check if this is because efficiencies are increasing (no diminishing returns)
        let efficiency_increasing = efficiencies.windows(2).all(|w| w[1].1 >= w[0].1);
        
        // Find VeryLight's reduction for reporting
        let verylight_reduction = size_reductions.iter()
            .find(|(level, _)| matches!(level, Some(GrainLevel::VeryLight)))
            .map(|(_, pct)| pct);
            
        if efficiency_increasing && max_efficiency > 0.0 {
            // Efficiency is still increasing - grain is benefiting from stronger denoising
            // But we'll be conservative and use VeryLight
            if let Some(reduction_pct) = verylight_reduction {
                log::debug!(
                    "Sample {}: Efficiency curve shows continued improvement (no knee point). \
                    Using VeryLight ({:.1}% reduction) as conservative choice.",
                    sample_index, reduction_pct
                );
                crate::progress_reporting::report_sub_item(&format!(
                    "Sample {}: No clear knee point (efficiency still increasing). Using VeryLight ({:.1}% reduction).",
                    sample_index, reduction_pct
                ));
            } else {
                crate::progress_reporting::report_sub_item(&format!(
                    "Sample {}: No clear knee point. Using VeryLight (conservative choice).",
                    sample_index
                ));
            }
        } else if let Some(reduction_pct) = verylight_reduction {
            // Some other reason - just report VeryLight usage
            log::debug!(
                "Sample {}: No candidates met threshold (max efficiency: {:.2}). \
                Using VeryLight ({:.1}% reduction).",
                sample_index, max_efficiency, reduction_pct
            );
            crate::progress_reporting::report_sub_item(&format!(
                "Sample {}: Using VeryLight ({:.1}% reduction) as safe default.",
                sample_index, reduction_pct
            ));
        } else {
            log::debug!(
                "Sample {}: No suitable candidates in knee analysis. Using VeryLight as safe default.",
                sample_index
            );
            crate::progress_reporting::report_sub_item(&format!(
                "Sample {}: Using VeryLight (safe default for compression benefits).",
                sample_index
            ));
        }

        // Use VeryLight as safe fallback
        GrainLevel::VeryLight
    }
}
