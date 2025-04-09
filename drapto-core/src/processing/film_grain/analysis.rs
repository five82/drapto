// Import necessary types
use crate::processing::film_grain::types::SampleResult;

/// Calculates the optimal grain value for a single sample based on a modified Knee/Elbow Point metric.
/// The efficiency calculation has been modified to use a square-root scale on the grain value,
/// reducing the penalty for higher grain values. This helps prevent the function from selecting
/// overly low grain values when the source video benefits from preserving more texture.
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

    // Collect efficiencies using adjusted metric: reduction divided by sqrt(grain_value)
    let mut efficiencies: Vec<(u8, f64)> = Vec::new();
    for test in sample_results.iter().filter(|t| t.grain_value > 0 && t.file_size > 0) {
        let reduction = base_size.saturating_sub(test.file_size) as f64;
        if reduction <= 0.0 {
            continue; // Only consider tests with positive size reduction
        }
        // Instead of dividing by grain_value, divide by its square-root to lessen the bias
        let adjusted_denom = (test.grain_value as f64).sqrt();
        let efficiency = reduction / adjusted_denom;
        if efficiency > 0.0 {
            efficiencies.push((test.grain_value, efficiency));
        }
    }

    if efficiencies.is_empty() {
        log_callback(&format!(
            "[DEBUG] Sample {}: No positive efficiency improvements found. Optimal grain: 0",
            sample_index_for_log + 1
        ));
        return 0;
    }

    // Find the maximum efficiency achieved
    let initial_max = (0u8, 0.0f64); // (grain, efficiency)
    let (grain_at_max_efficiency, max_efficiency) = efficiencies.iter().fold(initial_max, |acc, &(grain, eff)| {
        if eff > acc.1 && eff.is_finite() {
            (grain, eff)
        } else {
            acc
        }
    });

    if max_efficiency <= 0.0 {
        log_callback(&format!(
            "[DEBUG] Sample {}: Max efficiency is not positive (Max: {:.2} at grain {}). Optimal grain: 0",
            sample_index_for_log + 1,
            max_efficiency,
            grain_at_max_efficiency
        ));
        return 0;
    }

    // Find all candidate grain values whose efficiency is within the threshold of the max
    let mut candidates: Vec<(u8, f64)> = efficiencies
        .into_iter()
        .filter(|&(_, eff)| eff.is_finite() && eff >= knee_threshold * max_efficiency)
        .collect();

    // Sort candidates by grain value (lowest first)
    candidates.sort_by_key(|&(grain, _)| grain);

    // Choose the lowest grain that meets the threshold, or fall back to 0 if none are found
    let chosen_grain = if let Some(&(best_grain, best_eff)) = candidates.first() {
        log_callback(&format!(
            "[DEBUG] Sample {}: Max efficiency {:.2} at grain {}. Knee threshold {:.1}%. Found {} candidate(s) meeting threshold >= {:.2}. Choosing lowest grain: {} (Adjusted Efficiency: {:.2})",
            sample_index_for_log + 1,
            max_efficiency,
            grain_at_max_efficiency,
            knee_threshold * 100.0,
            candidates.len(),
            knee_threshold * max_efficiency,
            best_grain,
            best_eff
        ));
        best_grain
    } else {
        log_callback(&format!(
            "[DEBUG] Sample {}: Max efficiency {:.2} at grain {}. Knee threshold {:.1}%. No candidates met threshold >= {:.2}. Falling back to default grain: 0",
            sample_index_for_log + 1,
            max_efficiency,
            grain_at_max_efficiency,
            knee_threshold * 100.0,
            knee_threshold * max_efficiency
        ));
        0 // Fallback to 0 as per original design
    };

    chosen_grain
}

/// Calculates the median value of a slice of u8.
/// Sorts the slice in place. Returns 0 if the slice is empty.
pub(crate) fn calculate_median(data: &mut [u8]) -> u8 {
    if data.is_empty() {
        return 0; // Or handle error/option as appropriate
    }
    data.sort_unstable();
    let mid = data.len() / 2;
    if data.len() % 2 == 0 {
        // Even number of elements: average the two middle ones
        // Use u16 for intermediate sum to avoid overflow, then round to nearest u8
        let avg = (data[mid - 1] as u16 + data[mid] as u16) as f64 / 2.0;
        avg.round() as u8
    } else {
        // Odd number of elements: return the middle one
        data[mid]
    }
}

/// Calculates the population standard deviation of a slice of u8.
/// Returns None if the slice has fewer than 1 element or if calculation results in NaN/Infinity.
pub(crate) fn calculate_std_dev(data: &[u8]) -> Option<f64> {
    let n = data.len();
    if n == 0 {
        return None; // Cannot calculate std dev for empty list
    }
    if n == 1 {
        return Some(0.0); // Standard deviation of a single point is 0
    }

    // Calculate the mean
    let sum: f64 = data.iter().map(|&x| x as f64).sum();
    let mean = sum / (n as f64);

    // Calculate the sum of squared differences from the mean
    let variance_sum: f64 = data.iter().map(|&x| {
        let diff = x as f64 - mean;
        diff * diff
    }).sum();

    // Calculate population variance (divide by N)
    let variance = variance_sum / (n as f64);

    // Calculate standard deviation (square root of variance)
    let std_dev = variance.sqrt();

    // Check for NaN or Infinity which can occur with floating point issues
    if std_dev.is_finite() {
        Some(std_dev)
    } else {
        None // Indicate calculation failed
    }
}