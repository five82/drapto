// drapto-core/src/processing/film_grain/mod.rs
//
// This module orchestrates the process of determining an optimal film grain
// value for a given video file based on the provided configuration. It aims
// to find a balance between visual quality (retaining grain) and encoding
// efficiency (file size).
//
// Submodules:
// - `analysis`: Contains functions for analyzing the results of grain tests
//   (e.g., calculating efficiency, finding the knee point, median, std dev).
// - `sampling`: Provides functions for extracting video duration and performing
//   the core task of extracting a sample clip and encoding it with a specific
//   grain value to measure the resulting file size.
// - `types`: Defines data structures used to store the results of grain tests
//   (e.g., `GrainTest`, `SampleResult`, `AllResults`).
//
// Main Function: `determine_optimal_grain`
// This is the public entry point for the film grain optimization process. It
// takes the input video path, core configuration, a logging callback, and two
// function pointers (via dependency injection) for fetching duration and testing
// samples.
//
// The process involves multiple phases:
// 1. Configuration & Validation: Reads settings from `CoreConfig`, applies defaults,
//    and performs basic validation (e.g., checks for required grain value 0).
// 2. Duration & Sampling Points: Determines the video duration and calculates
//    evenly spaced time points within the video to extract samples from.
// 3. Phase 1 (Initial Broad Testing): Tests a predefined set of initial grain
//    values (from config or defaults) across all calculated sample points. This
//    provides a baseline understanding of how grain affects file size.
// 4. Phase 2 (Initial Estimation per Sample): Analyzes the Phase 1 results for
//    each sample individually using the configured metric (currently Knee Point)
//    to get an initial estimate of the optimal grain for that specific sample.
// 5. Phase 3 (Focused Refinement):
//    - Calculates the median of the Phase 2 estimates.
//    - Determines an adaptive refinement range around this median based on the
//      standard deviation of the Phase 2 estimates (provides tighter focus if
//      estimates are consistent, wider if they vary).
//    - Generates a small number of new grain values to test within this refined
//      range (excluding values already tested in Phase 1).
//    - Tests these refined grain values across all samples.
// 6. Phase 4 (Final Selection):
//    - Combines the results from Phase 1 and Phase 3 for each sample.
//    - Re-applies the Knee Point metric to this combined, richer dataset for each
//      sample to get a final, more accurate estimate for that sample.
// 7. Final Result: Calculates the median of the final estimates from Phase 4.
//    This median value represents the overall recommended optimal grain value,
//    which is then capped at the configured maximum allowed value.
//
// Dependency Injection:
// The use of `duration_fetcher` and `sample_tester` function arguments allows
// the core logic here to be decoupled from the specific implementation details
// of interacting with external tools (like `ffprobe` or `HandBrakeCLI`), making
// the analysis logic more testable and potentially adaptable to different tools.

// Declare submodules
pub mod analysis;
pub mod sampling;
pub mod types;

// Use necessary items from crate root and submodules
use crate::config::{CoreConfig, FilmGrainMetricType}; // Added FilmGrainMetricType
use crate::error::{CoreError, CoreResult};
use std::collections::HashSet;
use std::path::Path;
use std::vec::Vec; // Explicit import for clarity
use rand::{thread_rng, Rng}; // Added for randomized sampling

// Use items from submodules
use self::analysis::{calculate_knee_point_grain, calculate_median, calculate_std_dev};
use self::sampling::DEFAULT_SAMPLE_DURATION_SECS; // Import the constant
use self::types::{AllResults, GrainTest, SampleResult};

// --- Constants (Only those still needed locally) ---
// These are now primarily accessed via CoreConfig defaults, but kept here for reference
// if needed for internal logic unrelated to config defaults.
// Consider removing them if truly unused after refactoring CoreConfig usage.
const DEFAULT_INITIAL_GRAIN_VALUES: &[u8] = &[0, 5, 10, 15];
const DEFAULT_FALLBACK_GRAIN_VALUE: u8 = 0;
const DEFAULT_KNEE_THRESHOLD: f64 = 0.8;
const DEFAULT_REFINEMENT_RANGE_DELTA: u8 = 3; // Used as fallback if std dev calc fails
const DEFAULT_MAX_VALUE: u8 = 15;
const DEFAULT_REFINEMENT_POINTS_COUNT: usize = 3;


// --- Main Public Function ---

/// Analyzes the video file to determine the optimal film grain value using the configured metric.
/// This function now orchestrates calls to the sampling and analysis submodules.
pub fn determine_optimal_grain<F, D, S>(
    input_path: &Path,
    config: &CoreConfig,
    mut log_callback: F,
    duration_fetcher: D, // Dependency injection for duration lookup (e.g., sampling::get_video_duration_secs)
    sample_tester: S,    // Dependency injection for sample testing (e.g., sampling::extract_and_test_sample)
    handbrake_cmd_parts: &[String], // <-- Add HandBrake command parts
) -> CoreResult<u8>
where
    F: FnMut(&str),
    D: Fn(&Path) -> CoreResult<f64>,
    // Update sample_tester signature to include handbrake_cmd_parts
    S: Fn(&Path, f64, u32, u8, &CoreConfig, &[String]) -> CoreResult<u64>,
{
    // --- Get Configuration ---
    // Use constants from this module only if config doesn't provide them
    let sample_duration = config.film_grain_sample_duration.unwrap_or(DEFAULT_SAMPLE_DURATION_SECS);
    // let sample_count = config.film_grain_sample_count.unwrap_or(DEFAULT_SAMPLE_COUNT); // Replaced by dynamic calculation
    let initial_grain_values_slice = config.film_grain_initial_values.as_deref().unwrap_or(DEFAULT_INITIAL_GRAIN_VALUES);
    let fallback_value = config.film_grain_fallback_value.unwrap_or(DEFAULT_FALLBACK_GRAIN_VALUE);
    let metric_type = config.film_grain_metric_type.clone().unwrap_or(FilmGrainMetricType::KneePoint); // Use imported type
    let knee_threshold = config.film_grain_knee_threshold.unwrap_or(DEFAULT_KNEE_THRESHOLD);
    let max_value = config.film_grain_max_value.unwrap_or(DEFAULT_MAX_VALUE);
    let _refinement_points_count = config.film_grain_refinement_points_count.unwrap_or(DEFAULT_REFINEMENT_POINTS_COUNT);

    // --- Convert initial values to HashSet for quick lookups ---
    let initial_grain_values_set: HashSet<u8> = initial_grain_values_slice.iter().cloned().collect();

    // Basic validation
    // Validation for sample_count == 0 removed as dynamic calculation ensures >= 3 samples.
    if initial_grain_values_set.is_empty() {
        log_callback("[WARN] Film grain optimization requires at least one initial value. Using fallback.");
        return Ok(fallback_value);
    }
     if !initial_grain_values_set.contains(&0) {
         log_callback("[WARN] Initial grain values must include 0 for baseline comparison. Using fallback.");
         return Ok(fallback_value);
     }
     if metric_type != FilmGrainMetricType::KneePoint { // Use imported type
         log_callback(&format!("[WARN] Unsupported film grain metric type configured: {:?}. Only KneePoint is currently implemented. Using fallback.", metric_type));
         return Ok(fallback_value);
     }


    // --- Get Video Duration ---
    log_callback("[INFO] Detecting video duration...");
    let total_duration_secs = duration_fetcher(input_path)?;
    log_callback(&format!("[INFO] Video duration: {:.2} seconds", total_duration_secs));

    // --- Calculate Number of Samples ---
    const MIN_SAMPLES: usize = 3;
    const MAX_SAMPLES: usize = 9;
    const SECS_PER_SAMPLE_TARGET: f64 = 600.0; // 10 minutes

    let base_samples = (total_duration_secs / SECS_PER_SAMPLE_TARGET).ceil() as usize;
    let mut num_samples = base_samples.max(MIN_SAMPLES).min(MAX_SAMPLES);

    // Ensure odd number of samples, rounding up if necessary, capped by MAX_SAMPLES
    if num_samples % 2 == 0 {
        num_samples = (num_samples + 1).min(MAX_SAMPLES);
    }
    log_callback(&format!("[INFO] Calculated number of samples: {}", num_samples));

    // --- Validate Duration for Calculated Samples ---
    let min_required_duration = (sample_duration * num_samples as u32) as f64;
    if total_duration_secs < min_required_duration {
        log_callback(&format!(
            "[WARN] Video duration ({:.2}s) is too short for the minimum required duration ({:.2}s) for {} samples. Using fallback.",
            total_duration_secs, min_required_duration, num_samples
        ));
        return Ok(fallback_value);
    }

    // --- Calculate Randomized Sample Positions (within 15%-85% window) ---
    let sample_duration_f64 = sample_duration as f64;
    let start_boundary = total_duration_secs * 0.15;
    let end_boundary = total_duration_secs * 0.85; // Point after which sample *cannot* start
    let latest_possible_start = end_boundary - sample_duration_f64; // Latest time a sample can start

    if latest_possible_start <= start_boundary {
         log_callback(&format!(
            "[WARN] Video duration ({:.2}s) results in an invalid sampling window ({:.2}s - {:.2}s) for sample duration {}. Using fallback.",
            total_duration_secs, start_boundary, end_boundary, sample_duration
        ));
        return Ok(fallback_value);
    }

    // Check if the usable window duration is sufficient for the number of samples
    // Note: This is a basic check; it doesn't guarantee non-overlapping random samples,
    // but makes it highly likely for typical durations and sample counts.
    let usable_window_duration = latest_possible_start - start_boundary;
    if usable_window_duration < (num_samples as f64 * sample_duration_f64 * 0.5) { // Heuristic: window needs to be at least half the total sample time
         log_callback(&format!(
            "[WARN] Usable sampling window duration ({:.2}s to {:.2}s = {:.2}s) might be too small for {} samples of duration {}. Using fallback.",
            start_boundary, latest_possible_start, usable_window_duration, num_samples, sample_duration
        ));
        return Ok(fallback_value);
    }

    let mut sample_start_times = Vec::with_capacity(num_samples);
    let mut rng = thread_rng();
    // TODO: Consider adding logic to prevent samples from being too close together,
    // although random distribution makes significant overlap unlikely for small sample counts.
    for _ in 0..num_samples {
        let start_time = rng.gen_range(start_boundary..=latest_possible_start);
        sample_start_times.push(start_time);
    }
    // Sort for potentially more predictable logging/debugging, though not strictly necessary
    sample_start_times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    log_callback(&format!("[DEBUG] Generated random sample start times ({}): {:?}", num_samples, sample_start_times));

    // --- Phase 1: Test Initial Values ---
    log_callback(&format!("[INFO] Starting Film Grain Optimization - Phase 1: Initial Testing (Values: {:?})", initial_grain_values_slice));
    let mut phase1_results: AllResults = Vec::with_capacity(num_samples);

    for (i, &start_time) in sample_start_times.iter().enumerate() {
        log_callback(&format!(
            "[INFO] Analyzing sample {}/{} (at {:.2}s)...",
            i + 1, num_samples, start_time
        ));
        let mut sample_results: SampleResult = Vec::with_capacity(initial_grain_values_set.len());

        // Test initial values (iterate slice for order, use set for contains checks later)
        for &grain_value in initial_grain_values_slice {
            log_callback(&format!("[INFO]   Testing grain value {}...", grain_value));
            // Pass handbrake_cmd_parts to sample_tester
            match sample_tester(input_path, start_time, sample_duration, grain_value, config, handbrake_cmd_parts) {
                Ok(file_size) => {
                    sample_results.push(GrainTest { grain_value, file_size }); // Use imported type
                }
                Err(e) => {
                    log_callback(&format!("[ERROR] Failed testing grain value {}: {}", grain_value, e));
                    return Err(CoreError::FilmGrainAnalysisFailed(format!(
                        "Sample testing failed for grain value {}: {}", grain_value, e
                    )));
                }
            }
        }
        // Sort results for this sample before storing
        sample_results.sort_by_key(|test| test.grain_value);
        log_callback(&format!("[DEBUG]   Sample {} Phase 1 results: {:?}", i + 1, sample_results));
        phase1_results.push(sample_results);
    }

    // --- Phase 2: Estimate Optimal per Sample (using Knee Point) ---
    log_callback("[INFO] Phase 2: Estimating optimal grain per sample using Knee Point metric...");
    let mut initial_estimates: Vec<u8> = Vec::with_capacity(num_samples);
    for (i, sample_results) in phase1_results.iter().enumerate() {
        // Pass a mutable reference to log_callback
        let estimate = calculate_knee_point_grain(sample_results, knee_threshold, &mut log_callback, i); // Use analysis:: function
        initial_estimates.push(estimate);
    }
    log_callback(&format!("[INFO] Phase 2 Initial estimates per sample: {:?}", initial_estimates));

    // --- Phase 3: Focused Refinement ---
    let mut phase3_results: AllResults = Vec::with_capacity(num_samples); // Initialize even if refinement is skipped
    let mut refined_grain_values: Vec<u8> = Vec::new(); // Initialize

    if initial_estimates.is_empty() {
        log_callback("[WARN] No initial estimates generated in Phase 2. Skipping refinement and returning fallback value.");
        return Ok(fallback_value); // Return fallback if no initial estimates
    } else {
        let mut estimates_for_median = initial_estimates.clone();
        let median_estimate = calculate_median(&mut estimates_for_median); // Use analysis:: function
        log_callback(&format!("[INFO] Phase 3: Median of initial estimates: {}", median_estimate));

        // --- Adaptive Refinement Delta Calculation ---
        let std_dev_opt = calculate_std_dev(&initial_estimates); // Use analysis:: function
        const ADAPTIVE_DELTA_FACTOR: f64 = 1.5; // Factor to scale std dev by
        const MIN_ADAPTIVE_DELTA: u8 = 1; // Minimum delta when std dev > 0

        let adaptive_refinement_delta = match std_dev_opt {
            Some(std_dev) if std_dev.is_finite() && std_dev > 0.0 => {
                // Valid, positive standard deviation found
                let scaled_delta = (std_dev * ADAPTIVE_DELTA_FACTOR).round();
                // Ensure the result fits in u8 and clamp
                let delta = if scaled_delta < 0.0 {
                    MIN_ADAPTIVE_DELTA // Should not happen if std_dev > 0, but safety first
                } else if scaled_delta > u8::MAX as f64 {
                    // Avoid delta being excessively large, cap relative to max_value?
                    // For now, let's cap it reasonably, e.g., half of max_value or a fixed large number.
                    (max_value / 2).max(MIN_ADAPTIVE_DELTA)
                } else {
                    (scaled_delta as u8).max(MIN_ADAPTIVE_DELTA)
                };
                 log_callback(&format!("[DEBUG] Phase 3: Calculated standard deviation: {:.2}. Using adaptive delta: {}", std_dev, delta));
                 delta
            }
            Some(std_dev) if std_dev == 0.0 => {
                // Standard deviation is zero (all estimates were the same)
                log_callback(&format!("[DEBUG] Phase 3: Standard deviation is 0.0 (all initial estimates agree). Using minimal delta: {}", MIN_ADAPTIVE_DELTA));
                MIN_ADAPTIVE_DELTA // Use the minimum delta for refinement
            }
            _ => {
                // Fallback if std dev is NaN, infinite, or calculated from < 2 estimates (None)
                log_callback(&format!("[DEBUG] Phase 3: Std dev is NaN/Infinite or calculated from < 2 estimates. Using default delta: {}", DEFAULT_REFINEMENT_RANGE_DELTA));
                DEFAULT_REFINEMENT_RANGE_DELTA // Use the old default constant as fallback
            }
        };
        log_callback(&format!("[INFO] Phase 3: Using adaptive refinement delta: {}", adaptive_refinement_delta));
        // --- End Adaptive Refinement Delta ---

        log_callback(&format!("[INFO] Phase 3: Median of initial estimates: {}", median_estimate));

        // Define refinement range, clamping to 0..=max_value
        let lower_bound = median_estimate.saturating_sub(adaptive_refinement_delta);
        let upper_bound = median_estimate.saturating_add(adaptive_refinement_delta).min(max_value); // Cap at max_value

        log_callback(&format!("[INFO] Phase 3: Refinement range around median: [{}, {}]", lower_bound, upper_bound));

        // Generate refinement points within the range [lower_bound, upper_bound], excluding initial values
        if upper_bound >= lower_bound {
            // Iterate through all integer points in the calculated range
            for point in lower_bound..=upper_bound {
                // Add if not already in initial values
                if !initial_grain_values_set.contains(&point) {
                    // Check if the point is already in the list to avoid duplicates
                    // (though the direct iteration shouldn't cause duplicates here)
                    if !refined_grain_values.contains(&point) {
                         refined_grain_values.push(point);
                    }
                }
            }
            // Sort the collected points for consistent testing order
            refined_grain_values.sort_unstable();
        }

        // Old logic using refinement_points_count and step (commented out for reference):
        /*
        if upper_bound >= lower_bound && refinement_points_count > 0 { // Check upper_bound >= lower_bound
            // Use a HashSet for efficient checking of generated points
            let mut generated_points = HashSet::new();
            let step = if refinement_points_count > 0 && upper_bound > lower_bound {
                 (upper_bound as f64 - lower_bound as f64) / (refinement_points_count + 1) as f64
            } else {
                0.0 // Avoid division by zero if count is 0 or bounds are equal
            };

             for i in 1..=refinement_points_count {
                 let point = if step > 0.0 {
                     (lower_bound as f64 + step * i as f64).round() as u8
                 } else {
                     lower_bound // If no range or step, just consider the lower bound
                 };

                 // Clamp point within bounds [lower_bound, upper_bound]
                 let clamped_point = point.max(lower_bound).min(upper_bound);

                 // Add if not already in initial values and not already generated
                 if !initial_grain_values_set.contains(&clamped_point) && generated_points.insert(clamped_point) {
                    refined_grain_values.push(clamped_point); // Need to add to the actual list
                 }
             }
             refined_grain_values.sort_unstable(); // Sort after collecting
        }
        */


        if refined_grain_values.is_empty() {
            log_callback("[INFO] Phase 3: No suitable refinement points generated (or range too small). Skipping refined testing.");
        } else {
            log_callback(&format!("[INFO] Phase 3: Testing refined grain values: {:?}", refined_grain_values));

            // Initialize phase3_results with empty vectors for each sample
            for _ in 0..num_samples {
                phase3_results.push(Vec::with_capacity(refined_grain_values.len()));
            }

            // Test refined values for all samples
            for &grain_value in &refined_grain_values {
                 log_callback(&format!("[INFO]   Testing refined grain value {}...", grain_value));
                 for (i, &start_time) in sample_start_times.iter().enumerate() {
                     // Pass handbrake_cmd_parts to sample_tester
                     match sample_tester(input_path, start_time, sample_duration, grain_value, config, handbrake_cmd_parts) {
                         Ok(file_size) => {
                             // Add result to the correct sample's vector in phase3_results
                             phase3_results[i].push(GrainTest { grain_value, file_size }); // Use imported type
                         }
                         Err(e) => {
                             log_callback(&format!("[ERROR] Failed testing refined grain value {}: {}", grain_value, e));
                             return Err(CoreError::FilmGrainAnalysisFailed(format!(
                                 "Refined sample testing failed for grain value {}: {}", grain_value, e
                             )));
                         }
                     }
                 }
            }
             // Log phase 3 results per sample
             for (i, results) in phase3_results.iter_mut().enumerate() {
                 results.sort_by_key(|test| test.grain_value); // Sort before logging/merging
                 log_callback(&format!("[DEBUG]   Sample {} Phase 3 results: {:?}", i + 1, results));
             }
        }


    // --- Phase 4: Final Selection ---
    log_callback("[INFO] Phase 4: Determining final optimal grain using Knee Point on combined results...");
    let mut final_estimates: Vec<u8> = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        // Combine Phase 1 and Phase 3 results for this sample
        let mut combined_results = phase1_results[i].clone();
        // Ensure phase3_results has data for this index before extending
        if i < phase3_results.len() {
             combined_results.extend(phase3_results[i].clone());
        }


        // Sort combined results and remove duplicates (preferring the first occurrence)
        combined_results.sort_by_key(|test| test.grain_value);
        combined_results.dedup_by_key(|test| test.grain_value); // Keep first if duplicates exist

        log_callback(&format!("[DEBUG] Sample {}: Combined results for final analysis: {:?}", i + 1, combined_results));

        // Re-apply the Knee Point metric to the full dataset for this sample
        let final_estimate = calculate_knee_point_grain(&combined_results, knee_threshold, &mut log_callback, i); // Use analysis:: function
        final_estimates.push(final_estimate);
    }

    log_callback(&format!("[INFO] Phase 4 Final estimates per sample: {:?}", final_estimates));

    // --- Final Result ---
    if final_estimates.is_empty() {
        log_callback("[WARN] No final estimates generated. Using fallback value.");
        return Ok(fallback_value);
    }

    let mut final_estimates_for_median = final_estimates; // Can consume this now
    let median_value = calculate_median(&mut final_estimates_for_median); // Use analysis:: function
    let capped_median_value = median_value.min(max_value); // Apply final cap

    log_callback(&format!(
        "[INFO] Final Result: Median optimal grain: {}. Capped at {}: {}",
        median_value, max_value, capped_median_value
    ));

    Ok(capped_median_value)
    } // Close the main 'else' block from line 250
}

// Test module removed as per standard practice for library crates