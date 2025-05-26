// ============================================================================
// drapto-core/src/processing/detection/grain_analysis/knee_point.rs
// ============================================================================
//
// KNEE POINT ANALYSIS: Optimal Grain Level Detection with Quality Awareness
//
// PURPOSE: Reduce video file sizes for home/mobile viewing while maintaining
// acceptable visual quality. This is NOT for archival or professional use.
//
// APPROACH: Find the denoising level that provides good file size reduction
// without over-processing, now incorporating XPSNR quality metrics to ensure
// quality loss remains acceptable. When in doubt, use VeryLight as it provides
// compression benefits (5-10% typically) with virtually no quality risk.
//
// KEY PRINCIPLES: 
// - Some denoising is almost always beneficial. Even light denoising removes 
//   high-entropy noise that doesn't contribute to perceived quality but 
//   consumes significant bitrate during encoding.
// - XPSNR measurements help ensure denoising doesn't degrade quality too much
// - Quality factors are applied to efficiency calculations to penalize levels
//   that cause significant quality loss (XPSNR < 35 dB)
//
// AI-ASSISTANT-INFO: Knee point analysis with XPSNR quality metrics

// ---- Internal module imports ----
use super::types::{GrainLevel, GrainLevelTestResult};
use crate::progress_reporting::{LogLevel, report_log_message};

// ---- External crate imports ----
use log;

// ---- Standard library imports ----
use std::collections::HashMap;

/// Determines the optimal denoising level for file size reduction with quality awareness.
///
/// This function analyzes encoding results to find the best balance between
/// file size reduction and quality preservation. The goal is practical file
/// size reduction for home/mobile viewing, not archival quality.
///
/// # Algorithm Overview
///
/// 1. Calculate efficiency for each grain level: (size_reduction * quality_factor / sqrt(grain_level_value))
/// 2. Quality factor is derived from XPSNR measurements:
///    - XPSNR > 45 dB: factor = 1.0 (virtually no quality loss)
///    - XPSNR 40-45 dB: factor = 0.9-1.0 (minimal quality loss)
///    - XPSNR 35-40 dB: factor = 0.7-0.9 (moderate quality loss)
///    - XPSNR < 35 dB: factor < 0.7 (significant quality loss)
/// 3. Find diminishing returns point where improvement rate drops below 25%
/// 4. If no clear knee point exists, default to VeryLight (safe choice)
///
/// # Key Behaviors
///
/// - Always returns at least VeryLight (never Baseline/no denoising)
/// - Provides clear messaging about why a level was chosen
/// - Conservative approach: when uncertain, choose lighter denoising
/// - Penalizes levels that cause significant quality degradation
///
/// # Arguments
///
/// * `results` - HashMap mapping grain levels to their test results (size and XPSNR)
/// * `knee_threshold` - Threshold factor (0.0-1.0) for knee point selection (currently unused)
/// * `sample_index` - The index of the sample being analyzed (for logging)
///
/// # Returns
///
/// The optimal `GrainLevel` for file size reduction (minimum: VeryLight)
pub(super) fn analyze_sample_with_knee_point(
    results: &HashMap<Option<GrainLevel>, GrainLevelTestResult>,
    _knee_threshold: f64,
    sample_index: usize,
) -> GrainLevel {
    // ========================================================================
    // STEP 1: GET BASELINE AND VERYLIGHT RESULTS
    // ========================================================================

    // Get the baseline (no denoising) for informational display
    let baseline_result = match results.get(&None) {
        Some(result) if result.file_size > 0 => {
            Some(result)
        }
        _ => {
            // If None is missing, try Baseline as fallback
            results.get(&Some(GrainLevel::Baseline)).filter(|r| r.file_size > 0)
        }
    };
    
    // Get VeryLight result as our reference point for comparisons
    let verylight_result = match results.get(&Some(GrainLevel::VeryLight)) {
        Some(result) if result.file_size > 0 => {
            log::debug!("Sample {}: Using VeryLight as reference for comparisons.", sample_index);
            result
        }
        _ => {
            report_log_message(
                &format!(
                    "  Sample {}: ERROR: VeryLight reference is missing. Cannot analyze.",
                    sample_index
                ),
                LogLevel::Error,
            );
            return GrainLevel::VeryLight; // Return default value
        }
    };
    
    let verylight_size = verylight_result.file_size;
    let verylight_xpsnr = verylight_result.xpsnr;
    
    // Report baseline information if available
    if let Some(baseline) = baseline_result {
        let baseline_size_mb = baseline.file_size as f64 / (1024.0 * 1024.0);
        if let Some(baseline_xpsnr_val) = baseline.xpsnr {
            crate::progress_reporting::report_sub_item(&format!(
                "Sample {}: Baseline reference - {:.1} MB, XPSNR: {:.1} dB",
                sample_index, baseline_size_mb, baseline_xpsnr_val
            ));
        } else {
            crate::progress_reporting::report_sub_item(&format!(
                "Sample {}: Baseline reference - {:.1} MB",
                sample_index, baseline_size_mb
            ));
        }
    }

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
    
    // Track size reductions and quality metrics for better reporting
    let mut size_reductions: Vec<(Option<GrainLevel>, f64)> = Vec::new();
    let mut quality_metrics: Vec<(Option<GrainLevel>, f64)> = Vec::new();

    // Process each grain level and calculate its efficiency
    // Filter to only include valid grain levels with non-zero file sizes
    // Note: We include all levels (including None and Baseline) in the iteration
    // but will filter appropriately below
    for (&level, result) in results.iter().filter(|&(_, r)| r.file_size > 0) {
        let size = result.file_size;
        // Convert grain level to numeric value
        let grain_numeric = to_numeric(level);

        // Skip Baseline and VeryLight levels
        // Baseline is not a candidate, VeryLight is our reference
        if grain_numeric <= 1.0 {
            continue;
        }

        // Calculate size reduction compared to VeryLight
        // Use saturating_sub to avoid underflow if size > verylight_size
        let reduction = verylight_size.saturating_sub(size) as f64;

        // Skip levels that don't reduce file size beyond VeryLight
        if reduction <= 0.0 {
            continue;
        }
        
        // Calculate percentage reduction for reporting (still relative to VeryLight)
        let reduction_pct = (reduction / verylight_size as f64) * 100.0;
        size_reductions.push((level, reduction_pct));

        // Calculate quality loss if XPSNR is available
        let quality_factor = if let Some(xpsnr) = result.xpsnr {
            // Calculate delta from VeryLight XPSNR
            let xpsnr_delta = if let Some(verylight_xpsnr_val) = verylight_xpsnr {
                verylight_xpsnr_val - xpsnr
            } else {
                // If no VeryLight XPSNR, we can't calculate a proper delta
                // Be conservative and assume some quality loss
                log::debug!("No VeryLight XPSNR available for delta calculation");
                1.0 // Assume 1 dB loss as conservative estimate
            };
            
            // Quality factor based on XPSNR delta from VeryLight:
            // Goal: Maximize compression while staying below perceptible quality loss
            // Based on research consensus for XPSNR JND thresholds:
            // - < 0.25 dB: Virtually imperceptible (no penalty)
            // - 0.25-0.45 dB: Borderline JND, maybe noticeable on side-by-side
            // - 0.5-1.0 dB: Reliably noticeable, still acceptable for streaming
            // - 1.0-3.0 dB: Clearly visible to most viewers
            // - > 3.0 dB: Significant quality degradation
            
            let quality_factor = if xpsnr_delta < 0.25 {
                1.0 // Virtually imperceptible - no penalty
            } else if xpsnr_delta < 0.45 {
                1.0 // Borderline JND - still no penalty for home viewing
            } else if xpsnr_delta < 1.0 {
                1.0 - (xpsnr_delta - 0.45) * 0.091 // Linear from 1.0 to 0.95
            } else if xpsnr_delta < 3.0 {
                0.95 - (xpsnr_delta - 1.0) * 0.125 // Linear from 0.95 to 0.7
            } else {
                // More than 3 dB loss - apply heavy penalty
                (0.7 - (xpsnr_delta - 3.0) * 0.15).max(0.2)
            };
            
            quality_metrics.push((level, xpsnr));
            quality_factor
        } else {
            // If no XPSNR data, be conservative and apply a small penalty
            // This encourages using levels with quality measurements
            0.85
        };

        // Calculate efficiency using square-root scaling with quality factor
        // This reduces bias against higher denoising levels by making the
        // denominator grow more slowly than the level value
        let adjusted_denom = grain_numeric.sqrt();
        let efficiency = (reduction * quality_factor) / adjusted_denom;

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
        let level_result = results.get(&Some(level)).unwrap();
        // Size reduction from VeryLight (for decision display)
        let size_reduction_from_verylight = ((verylight_size - level_result.file_size) as f64 / verylight_size as f64) * 100.0;
        
        // Get quality metric for reporting (show delta from VeryLight)
        let quality_info = if let Some(xpsnr) = level_result.xpsnr {
            if let Some(verylight_xpsnr_val) = verylight_xpsnr {
                let delta = verylight_xpsnr_val - xpsnr;
                if delta > 0.1 {
                    format!(" (XPSNR: {:.1} dB, -{:.1} dB from VeryLight)", xpsnr, delta)
                } else {
                    format!(" (XPSNR: {:.1} dB, same as VeryLight)", xpsnr)
                }
            } else {
                format!(" (XPSNR: {:.1} dB)", xpsnr)
            }
        } else {
            String::new()
        };
        
        // Check if this was selected due to continuous improvement
        let is_continuous_improvement = sorted_efficiencies.windows(2)
            .all(|w| w[1].1 > w[0].1) && level == GrainLevel::Light;
            
        // Report the analysis results as sub-items in verbose mode
        if is_continuous_improvement {
            log::debug!(
                "Sample {}: Continuous improvement detected. Selected {:?} as balanced choice. Efficiency: {:.2}, Size reduction: {:.1}% beyond VeryLight",
                sample_index, level, efficiency, size_reduction_from_verylight
            );
            crate::progress_reporting::report_sub_item(&format!(
                "Sample {}: Selected {:?} ({:.1}% additional reduction{}) - continuous improvement",
                sample_index, level, size_reduction_from_verylight, quality_info
            ));
        } else {
            log::debug!(
                "Sample {}: Knee point found at {:?}. Efficiency: {:.2}, Size reduction: {:.1}% beyond VeryLight",
                sample_index, level, efficiency, size_reduction_from_verylight
            );
            crate::progress_reporting::report_sub_item(&format!(
                "Sample {}: Selected {:?} ({:.1}% additional reduction{})",
                sample_index, level, size_reduction_from_verylight, quality_info
            ));
        }

        // Return the selected optimal grain level
        level
    } else {
        // No suitable level found - return VeryLight
        log::debug!(
            "Sample {}: No improvements beyond VeryLight found. Using VeryLight as optimal choice.",
            sample_index
        );
        crate::progress_reporting::report_sub_item(&format!(
            "Sample {}: Using VeryLight (no better alternatives found).",
            sample_index
        ));

        // Use VeryLight as the result
        GrainLevel::VeryLight
    }
}
