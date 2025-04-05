// drapto-core/src/processing/film_grain/analysis.rs
//
// This module provides functions specifically for analyzing the results obtained
// from testing different film grain values during the optimization process.
// It focuses on statistical calculations and applying the chosen efficiency metric.
//
// Functions:
// - `calculate_median`: A helper function (`pub(crate)`) that computes the median
//   value of a slice of `u8` values. It sorts the input slice in place.
// - `calculate_std_dev`: A helper function (`pub(crate)`) that computes the sample
//   standard deviation of a slice of `u8` values. Returns `None` if fewer than
//   two values are provided.
// - `calculate_knee_point_grain`: The core analysis function (`pub(crate)`) for
//   the "Knee Point" (or Elbow Point) metric. It takes the results from testing
//   various grain values for a single sample (`SampleResult`), calculates the
//   efficiency (reduction in file size per unit of grain) for each test relative
//   to the grain=0 baseline, finds the maximum efficiency achieved, and then selects
//   the *lowest* grain value that achieves an efficiency within a certain threshold
//   (e.g., 80%) of the maximum efficiency. This aims to find the point of diminishing
//   returns, selecting a grain value that provides most of the size reduction benefit
//   without potentially going much higher for minimal extra gain. It includes detailed
//   logging via the provided `log_callback`.

use super::types::SampleResult; // Use super to access types module
use std::vec::Vec; // Explicit import

// --- Helper Functions ---

/// Helper function to calculate the median of a slice of u8.
/// Note: Sorts the input slice in place.
pub(crate) fn calculate_median(values: &mut [u8]) -> u8 {
    if values.is_empty() {
        return 0; // Or handle error/default appropriately
    }
    values.sort_unstable();
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        // Even number of elements, average the two middle ones
        // Use u16 for intermediate sum to avoid overflow
        ((values[mid - 1] as u16 + values[mid] as u16) / 2) as u8
    } else {
        // Odd number of elements, return the middle one
        values[mid]
    }
}

/// Helper function to calculate the standard deviation of a slice of u8.
/// Returns None if the slice has fewer than 2 elements (std dev is undefined/0).
/// Uses sample standard deviation (n-1 denominator).
pub(crate) fn calculate_std_dev(values: &[u8]) -> Option<f64> {
    let n = values.len();
    if n < 2 {
        return None; // Standard deviation requires at least 2 data points
    }

    let n_f64 = n as f64;

    // Calculate the mean
    let sum: u64 = values.iter().map(|&v| v as u64).sum();
    let mean = sum as f64 / n_f64;

    // Calculate the sum of squared differences from the mean
    let variance_sum: f64 = values
        .iter()
        .map(|&v| {
            let diff = v as f64 - mean;
            diff * diff
        })
        .sum();

    // Calculate variance (using n-1 for sample standard deviation)
    // Handle potential division by zero if n=1, although already checked n < 2
    let variance = if n > 1 { variance_sum / (n_f64 - 1.0) } else { 0.0 };


    // Standard deviation is the square root of variance
    Some(variance.sqrt())
}


/// Calculates the optimal grain value for a single sample based on the Knee/Elbow Point metric.
pub(crate) fn calculate_knee_point_grain(
    sample_results: &SampleResult, // Assumes results are already sorted by grain_value
    knee_threshold: f64,
    log_callback: &mut dyn FnMut(&str),
    sample_index_for_log: usize, // For clearer logging
) -> u8 {
    // Find base size (grain=0)
    let base_test = sample_results.iter().find(|test| test.grain_value == 0);

    let base_size = match base_test {
        Some(test) if test.file_size > 0 => test.file_size,
        _ => {
            log_callback(&format!(
                "[WARN] Sample {}: Base size (grain=0) is zero or missing. Cannot calculate knee point. Defaulting to 0.",
                sample_index_for_log + 1
            ));
            return 0; // Cannot calculate efficiency without a valid base size
        }
    };

    let mut efficiencies: Vec<(u8, f64)> = Vec::new();
    for test in sample_results.iter().filter(|t| t.grain_value > 0 && t.file_size > 0) {
        // Ensure reduction is non-negative before casting, prevent underflow
        let reduction = base_size.saturating_sub(test.file_size) as f64;
        if reduction <= 0.0 { continue; } // Only consider actual reductions

        let efficiency = reduction / test.grain_value as f64;
        if efficiency > 0.0 { // Only consider positive efficiency
             efficiencies.push((test.grain_value, efficiency));
        }
    }

    if efficiencies.is_empty() {
        log_callback(&format!(
            "[DEBUG] Sample {}: No positive efficiency improvements found. Optimal grain: 0",
            sample_index_for_log + 1
        ));
        return 0; // No improvement found
    }

    // Find max efficiency
    // Use fold to handle potential NaN or comparison issues more robustly, finding the max valid efficiency.
    let initial_max = (0u8, 0.0f64); // (grain, efficiency)
    let (grain_at_max_efficiency, max_efficiency) = efficiencies
        .iter()
        .fold(initial_max, |acc, &(grain, eff)| {
            if eff > acc.1 && eff.is_finite() {
                (grain, eff)
            } else {
                acc
            }
        });


    if max_efficiency <= 0.0 {
         log_callback(&format!(
            "[DEBUG] Sample {}: Max efficiency is not positive (Max: {:.2} at grain {}). Optimal grain: 0",
             sample_index_for_log + 1, max_efficiency, grain_at_max_efficiency
        ));
        return 0;
    }


    // Find candidates meeting the threshold
    let mut candidates: Vec<(u8, f64)> = efficiencies
        .into_iter()
        .filter(|&(_, eff)| eff.is_finite() && eff >= knee_threshold * max_efficiency)
        .collect();

    // Sort candidates by grain value (ascending)
    candidates.sort_by_key(|&(grain, _)| grain);

    // Return the grain value of the first candidate (lowest grain meeting threshold)
    // If no candidates meet the threshold, return the grain value that had the max efficiency.
    let chosen_grain = if let Some(&(best_grain, best_eff)) = candidates.first() {
        log_callback(&format!(
            "[DEBUG] Sample {}: Max efficiency {:.2} at grain {}. Knee threshold {:.1}%. Found {} candidates meeting threshold >= {:.2}. Choosing lowest grain: {} (Efficiency: {:.2})",
            sample_index_for_log + 1, max_efficiency, grain_at_max_efficiency, knee_threshold * 100.0, candidates.len(), knee_threshold * max_efficiency, best_grain, best_eff
        ));
        best_grain
    } else {
         log_callback(&format!(
            "[DEBUG] Sample {}: Max efficiency {:.2} at grain {}. Knee threshold {:.1}%. No candidates met threshold >= {:.2}. Falling back to default grain: 0",
            sample_index_for_log + 1, max_efficiency, grain_at_max_efficiency, knee_threshold * 100.0, knee_threshold * max_efficiency
        ));
        0 // Fallback to 0 as per proposal
    };

    chosen_grain
}