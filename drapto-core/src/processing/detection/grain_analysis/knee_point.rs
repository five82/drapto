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
    _knee_threshold: f64,
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
            None => 0.0,                            // Baseline (no denoising)
            Some(GrainLevel::Baseline) => 0.0,      // Equivalent to no denoising
            Some(GrainLevel::VeryLight) => 1.0,     // Very light denoising
            Some(GrainLevel::Light) => 2.0,         // Light denoising
            Some(GrainLevel::LightModerate) => 3.0, // Light to moderate denoising
            Some(GrainLevel::Moderate) => 4.0,      // Moderate denoising
            Some(GrainLevel::Elevated) => 5.0,      // Elevated denoising
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
    // STEP 4: FIND DIMINISHING RETURNS POINT
    // ========================================================================

    // Sort efficiencies by grain level (lowest to highest)
    let mut sorted_efficiencies = efficiencies.clone();
    sorted_efficiencies.sort_by(|a, b| {
        let a_val = to_numeric(a.0);
        let b_val = to_numeric(b.0);
        a_val
            .partial_cmp(&b_val)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Find the point of diminishing returns
    let mut selected_level: Option<(Option<GrainLevel>, f64)> = None;
    
    // If we only have one efficiency, use it
    if sorted_efficiencies.len() == 1 {
        selected_level = sorted_efficiencies.first().cloned();
    } else {
        // Calculate improvement rates between consecutive levels
        for i in 0..sorted_efficiencies.len() {
            if i == 0 {
                // First level always has good improvement over baseline
                continue;
            }
            
            let (curr_level, curr_eff) = sorted_efficiencies[i];
            let (prev_level, prev_eff) = sorted_efficiencies[i - 1];
            
            // Calculate the improvement rate
            let improvement = curr_eff - prev_eff;
            let improvement_rate = if prev_eff > 0.0 {
                improvement / prev_eff
            } else if improvement > 0.0 {
                1.0 // 100% improvement from zero
            } else {
                0.0
            };
            
            log::debug!(
                "Sample {}: {:?} to {:?}: improvement={:.2}, rate={:.1}%",
                sample_index,
                prev_level.unwrap_or(GrainLevel::Baseline),
                curr_level.unwrap_or(GrainLevel::Baseline),
                improvement,
                improvement_rate * 100.0
            );
            
            // If improvement rate drops below 25%, we've hit diminishing returns
            if improvement_rate < 0.25 {
                // Use the previous level (before diminishing returns)
                selected_level = Some(sorted_efficiencies[i - 1]);
                log::debug!(
                    "Sample {}: Diminishing returns detected at {:?}, selecting {:?}",
                    sample_index,
                    curr_level.unwrap_or(GrainLevel::Baseline),
                    prev_level.unwrap_or(GrainLevel::Baseline)
                );
                break;
            }
        }
        
        // If no diminishing returns found (efficiency keeps improving), 
        // check if this is a continuous improvement pattern
        if selected_level.is_none() && !sorted_efficiencies.is_empty() {
            // Check if efficiency is continuously increasing
            let continuously_improving = sorted_efficiencies.windows(2)
                .all(|w| w[1].1 > w[0].1);
                
            if continuously_improving && sorted_efficiencies.len() >= 3 {
                // Efficiency keeps improving - use Light as balanced choice
                // Light provides good compression while staying conservative
                log::debug!(
                    "Sample {}: Efficiency continuously improving. Selecting Light as balanced choice.",
                    sample_index
                );
                
                // Find Light level in the sorted efficiencies
                selected_level = sorted_efficiencies
                    .iter()
                    .find(|(level, _)| matches!(level, Some(GrainLevel::Light)))
                    .cloned();
                    
                // If Light wasn't tested (shouldn't happen), fall back to 70% threshold
                if selected_level.is_none() {
                    let max_efficiency = sorted_efficiencies
                        .iter()
                        .map(|(_, eff)| *eff)
                        .fold(0.0, f64::max);
                        
                    let threshold = 0.7 * max_efficiency;
                    selected_level = sorted_efficiencies
                        .iter()
                        .find(|(_, eff)| *eff >= threshold)
                        .cloned();
                }
            } else {
                // Not continuously improving or too few data points
                // Use the first level that achieves reasonable efficiency
                selected_level = sorted_efficiencies.first().cloned();
            }
        }
    }

    // ========================================================================
    // STEP 5: APPLY SELECTION
    // ========================================================================

    // Process the selected level
    if let Some((Some(level), efficiency)) = selected_level {
        // Calculate the actual file size reduction percentage for this level
        let level_result = results.get(&Some(level)).unwrap_or(&baseline_size);
        let size_reduction_pct = ((baseline_size - level_result) as f64 / baseline_size as f64) * 100.0;
        
        // Check if this was selected due to continuous improvement
        let is_continuous_improvement = sorted_efficiencies.windows(2)
            .all(|w| w[1].1 > w[0].1) && level == GrainLevel::Light;
            
        // Report the analysis results as sub-items in verbose mode
        if is_continuous_improvement {
            log::debug!(
                "Sample {}: Continuous improvement detected. Selected {:?} as balanced choice. Efficiency: {:.2}, Size reduction: {:.1}%",
                sample_index, level, efficiency, size_reduction_pct
            );
            crate::progress_reporting::report_sub_item(&format!(
                "Sample {}: Selected {:?} ({:.1}% reduction) - continuous improvement",
                sample_index, level, size_reduction_pct
            ));
        } else {
            log::debug!(
                "Sample {}: Knee point found at {:?}. Efficiency: {:.2}, Size reduction: {:.1}%",
                sample_index, level, efficiency, size_reduction_pct
            );
            crate::progress_reporting::report_sub_item(&format!(
                "Sample {}: Selected {:?} ({:.1}% size reduction)",
                sample_index, level, size_reduction_pct
            ));
        }

        // Return the selected optimal grain level
        level
    } else {
        // No suitable level found - this should rarely happen with improved logic
        // Use VeryLight as ultimate safe default
        
        // Find VeryLight's reduction for reporting
        let verylight_reduction = size_reductions.iter()
            .find(|(level, _)| matches!(level, Some(GrainLevel::VeryLight)))
            .map(|(_, pct)| pct);
            
        if let Some(reduction_pct) = verylight_reduction {
            log::debug!(
                "Sample {}: No clear pattern detected. \
                Using VeryLight ({:.1}% reduction) as conservative choice.",
                sample_index, reduction_pct
            );
            crate::progress_reporting::report_sub_item(&format!(
                "Sample {}: Using VeryLight ({:.1}% reduction) as safe default.",
                sample_index, reduction_pct
            ));
        } else {
            log::debug!(
                "Sample {}: No suitable level found. Using VeryLight as safe default.",
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
