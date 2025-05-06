// drapto-core/src/processing/detection/grain_analysis/knee_point.rs
use super::types::GrainLevel;
use std::collections::HashMap;

/// Calculates the optimal grain level for a single sample based on a knee point analysis.
/// Returns the GrainLevel that provides the best compression efficiency.
pub(super) fn analyze_sample_with_knee_point(
    results: &HashMap<Option<GrainLevel>, u64>,
    knee_threshold: f64,
    log_callback: &mut impl Fn(&str)
) -> GrainLevel {
    // Implementation based on reference code's calculate_knee_point_grain

    // Find baseline size (no denoise)
    let baseline_size = match results.get(&None) {
        Some(&size) => size,
        None => {
            log_callback("Baseline encode size missing from results. Cannot analyze with knee point.");
            return GrainLevel::VeryClean;
        }
    };

    // Map GrainLevel to a numerical scale for calculations
    // VeryLight -> 1, Light -> 2, Visible -> 3, Heavy -> 4
    let to_numeric = |level: Option<GrainLevel>| -> f64 {
        match level {
            None => 0.0,
            Some(GrainLevel::VeryClean) => 0.0, // Treat VeryClean as 0 for efficiency calc? Or skip? Let's skip.
            Some(GrainLevel::VeryLight) => 1.0,
            Some(GrainLevel::Light) => 2.0,
            Some(GrainLevel::Visible) => 3.0,
            Some(GrainLevel::Heavy) => 4.0,
        }
    };

    // Collect efficiencies using adjusted metric: reduction divided by sqrt(grain_value)
    let mut efficiencies: Vec<(Option<GrainLevel>, f64)> = Vec::new();

    // Apply compiler suggestion for reference pattern in closure
    for (&level, &size) in results.iter().filter(|&(&k, &v)| v > 0 && k.is_some()) {
        let grain_numeric = to_numeric(level);
        if grain_numeric <= 0.0 {
            continue; // Skip baseline or VeryClean
        }

        let reduction = baseline_size.saturating_sub(size) as f64;
        if reduction <= 0.0 {
            continue; // Only consider tests with positive size reduction
        }

        // Use square-root scale to reduce bias against higher levels
        let adjusted_denom = grain_numeric.sqrt();
        let efficiency = reduction / adjusted_denom;

        if efficiency > 0.0 && efficiency.is_finite() { // Added is_finite check
            efficiencies.push((level, efficiency));
        }
    }

    if efficiencies.is_empty() {
        log_callback("No positive efficiency improvements found with knee point analysis.");
        return GrainLevel::VeryClean;
    }

    // Find the maximum efficiency
    let (max_level, max_efficiency) = efficiencies.iter()
        .fold((None, 0.0), |acc, &(level, eff)| {
            if eff > acc.1 && eff.is_finite() {
                (level, eff)
            } else {
                acc
            }
        });

    if max_efficiency <= 0.0 {
        log_callback(&format!(
            "Max efficiency is not positive (Max: {:.2}). Using VeryClean.",
            max_efficiency
        ));
        return GrainLevel::VeryClean;
    }

    // Find all candidates meeting the threshold
    let threshold_efficiency = knee_threshold * max_efficiency;
    let mut candidates: Vec<(Option<GrainLevel>, f64)> = efficiencies
        .into_iter()
        .filter(|&(_, eff)| eff.is_finite() && eff >= threshold_efficiency)
        .collect();

    // Sort candidates by lowest grain level first (VeryLight before Light, etc.)
    candidates.sort_by(|a, b| {
        let a_val = to_numeric(a.0);
        let b_val = to_numeric(b.0);
        a_val.partial_cmp(&b_val).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Choose the lowest level that meets the threshold
    if let Some(&(Some(level), _)) = candidates.first() {
        log_callback(&format!(
            "Knee point analysis: Max efficiency {:.2} at level {:?}. Threshold {:.1}%. Choosing: {:?}",
            max_efficiency,
            max_level.unwrap_or(GrainLevel::VeryClean), // Provide default for logging
            knee_threshold * 100.0,
            level
        ));
        level
    } else {
        log_callback(&format!(
            "No suitable candidates found in knee point analysis. Using VeryClean."
        ));
        GrainLevel::VeryClean
    }
}