use crate::{CoreConfig, CoreError, CoreResult}; // Removed unused FilmGrainMetricType import
use std::path::Path;
use std::process::Command;
use std::fs;
use std::process::Stdio;
use tempfile::tempdir;
use std::fmt;
use std::collections::HashSet; // Needed for checking tested values

// --- Data Structures ---

/// Stores the result of a single grain test encode for a sample
// Keep Clone separate
#[derive(Clone)]
struct GrainTest {
    grain_value: u8,
    file_size: u64, // Bytes
}

impl fmt::Debug for GrainTest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file_size_mb = self.file_size as f64 / (1024.0 * 1024.0);
        f.debug_struct("GrainTest")
            .field("grain_value", &self.grain_value)
            // Format file_size to 2 decimal places and add " MB"
            .field("file_size", &format_args!("{:.2} MB", file_size_mb))
            .finish()
    }
}

/// Stores all test results for a single sample point
type SampleResult = Vec<GrainTest>;

/// Stores all results across all sample points
type AllResults = Vec<SampleResult>;

// --- Constants ---
const DEFAULT_SAMPLE_DURATION_SECS: u32 = 10;
const DEFAULT_SAMPLE_COUNT: usize = 3;
const DEFAULT_INITIAL_GRAIN_VALUES: &[u8] = &[0, 5, 10, 15, 20]; // Phase 1 Coarse Values (0-20 range)
const DEFAULT_FALLBACK_GRAIN_VALUE: u8 = 0;
const DEFAULT_KNEE_THRESHOLD: f64 = 0.8; // Default 80%
const DEFAULT_REFINEMENT_RANGE_DELTA: u8 = 3; // Default +/- 3
const DEFAULT_MAX_VALUE: u8 = 20; // Default max 20
const DEFAULT_REFINEMENT_POINTS_COUNT: usize = 3; // Default 3 points

// --- Helper Functions ---

/// Gets video duration in seconds using ffprobe (or mock for tests).
/// TODO: Consider moving this to a shared utility module if needed elsewhere.
pub(crate) fn get_video_duration_secs(_input_path: &Path) -> CoreResult<f64> {
    #[cfg(test)]
    {
        // Test implementation using thread_local mock
        MOCK_DURATION_SECS.with(|cell| {
            if let Some(duration) = cell.get() {
                Ok(duration)
            } else {
                Ok(300.0) // Default mock duration if not set
            }
        })
        // To test failure:
        // Err(CoreError::FfprobeParse("Mock duration failure".to_string()))
    }
    #[cfg(not(test))]
    {
        // Real implementation using ffprobe
        let cmd_name = "ffprobe";
        let output = Command::new(cmd_name)
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1", // Output only the duration value
            ])
            .arg(_input_path) // Use the (potentially prefixed) parameter name
            .output()
            .map_err(|e| CoreError::CommandStart(cmd_name.to_string(), e))?;

        if !output.status.success() {
            return Err(CoreError::CommandFailed(
                cmd_name.to_string(),
                output.status,
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .trim()
            .parse::<f64>()
            .map_err(|e| CoreError::FfprobeParse(format!("Failed to parse duration '{}': {}", stdout.trim(), e)))
    }
}

// --- Mocking Helpers (Only compiled for tests) ---

#[cfg(test)]
thread_local! {
    static MOCK_DURATION_SECS: std::cell::Cell<Option<f64>> = std::cell::Cell::new(None);
}

// Helper function set_mock_duration removed as it's no longer used


// --- Mockable Sample Testing ---

// Define the actual implementation for non-test builds
// #[cfg(not(test))] // No longer needed with DI
/// Encodes a sample using HandBrakeCLI with specific settings and returns the output file size.
/// Suppresses HandBrakeCLI output. Made pub(crate) for DI.
pub(crate) fn extract_and_test_sample(
    input_path: &Path,
    start_secs: f64,
    duration_secs: u32,
    grain_value: u8,
    config: &CoreConfig,
    // log_callback: &mut dyn FnMut(&str), // Not needed here, logging is done by caller
) -> CoreResult<u64> {
    let temp_dir = tempdir().map_err(CoreError::Io)?;
    let output_filename = format!(
        "sample_start{}_dur{}_grain{}.mkv",
        start_secs.round(), duration_secs, grain_value
    );
    let output_path = temp_dir.path().join(output_filename);

    let mut handbrake_args: Vec<String> = Vec::new();

    // --- Base Parameters (mirroring lib.rs, but without audio/subs for speed) ---
    handbrake_args.push("--encoder".to_string());
    handbrake_args.push("svt_av1_10bit".to_string());
    handbrake_args.push("--encoder-tune".to_string());
    handbrake_args.push("0".to_string());

    // Dynamic film grain setting
    let encopts = format!("film-grain={}:film-grain-denoise=1", grain_value);
    handbrake_args.push("--encopts".to_string());
    handbrake_args.push(encopts);

    // Encoder Preset
    let encoder_preset = config.default_encoder_preset.unwrap_or(6);
    handbrake_args.push("--encoder-preset".to_string());
    handbrake_args.push(encoder_preset.to_string());

    // Quality
    let quality = config.default_quality.unwrap_or(28);
    handbrake_args.push("--quality".to_string());
    handbrake_args.push(quality.to_string());

    // Crop Mode (use config or default 'off' for samples?) - Let's use config for consistency
    if let Some(crop_mode) = &config.default_crop_mode {
        handbrake_args.push("--crop-mode".to_string());
        handbrake_args.push(crop_mode.clone());
    } else {
         // Default to 'off' if not specified, to avoid auto-crop variance affecting size
         handbrake_args.push("--crop-mode".to_string());
         handbrake_args.push("off".to_string());
    }

    // --- Sample Specific Parameters ---
    handbrake_args.push("--start-at".to_string());
    handbrake_args.push(format!("duration:{}", start_secs));
    handbrake_args.push("--stop-at".to_string());
    handbrake_args.push(format!("duration:{}", duration_secs));

    // Disable audio and subtitles for faster sample encodes
    handbrake_args.push("-a".to_string());
    handbrake_args.push("none".to_string());
    handbrake_args.push("-s".to_string());
    handbrake_args.push("none".to_string());

    // Input and Output
    handbrake_args.push("-i".to_string());
    handbrake_args.push(input_path.to_string_lossy().to_string());
    handbrake_args.push("-o".to_string());
    handbrake_args.push(output_path.to_string_lossy().to_string());

    // --- Output Suppression ---
    // Use verbose level 0 for minimal console output during sample encodes
    handbrake_args.push("--verbose=0".to_string());

    // --- Execute ---
    let cmd_handbrake = "HandBrakeCLI";
    let status = Command::new(cmd_handbrake)
        .args(&handbrake_args)
        .stdout(Stdio::null()) // Ensure stdout is ignored
        .stderr(Stdio::null()) // Ensure stderr is ignored
        .status()
        .map_err(|e| CoreError::CommandStart(cmd_handbrake.to_string(), e))?;

    if !status.success() {
        // We don't have stderr here, so provide a generic error message
        return Err(CoreError::FilmGrainEncodingFailed(format!(
            "HandBrakeCLI failed for sample (start: {}, grain: {}) with status {}",
            start_secs, grain_value, status
        )));
    }

    // Get file size
    let metadata = fs::metadata(&output_path).map_err(CoreError::Io)?;
    Ok(metadata.len())

    // temp_dir and its contents are automatically cleaned up when `temp_dir` goes out of scope
}


// --- Helper Functions (New/Refactored) ---

// Helper function to calculate the median of a slice of u8
// Note: Sorts the input slice in place.
fn calculate_median(values: &mut [u8]) -> u8 {
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

// Helper function to calculate the standard deviation of a slice of u8
// Returns None if the slice has fewer than 2 elements (std dev is undefined/0)
// Uses sample standard deviation (n-1 denominator).
fn calculate_std_dev(values: &[u8]) -> Option<f64> {
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
fn calculate_knee_point_grain(
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


// --- Main Public Function ---

/// Analyzes the video file to determine the optimal film grain value.
/// Analyzes the video file to determine the optimal film grain value using the configured metric.
pub fn determine_optimal_grain<F, D, S>(
    input_path: &Path,
    config: &CoreConfig,
    mut log_callback: F,
    duration_fetcher: D, // Dependency injection for duration lookup
    sample_tester: S,    // Dependency injection for sample testing
) -> CoreResult<u8>
where
    F: FnMut(&str),
    D: Fn(&Path) -> CoreResult<f64>,
    S: Fn(&Path, f64, u32, u8, &CoreConfig) -> CoreResult<u64>, // Sample tester signature
{
    // --- Get Configuration ---
    let sample_duration = config.film_grain_sample_duration.unwrap_or(DEFAULT_SAMPLE_DURATION_SECS);
    let sample_count = config.film_grain_sample_count.unwrap_or(DEFAULT_SAMPLE_COUNT);
    let initial_grain_values_slice = config.film_grain_initial_values.as_deref().unwrap_or(DEFAULT_INITIAL_GRAIN_VALUES);
    let fallback_value = config.film_grain_fallback_value.unwrap_or(DEFAULT_FALLBACK_GRAIN_VALUE);
    // --- New Config ---
    let metric_type = config.film_grain_metric_type.clone().unwrap_or(crate::FilmGrainMetricType::KneePoint); // Default to KneePoint
    let knee_threshold = config.film_grain_knee_threshold.unwrap_or(DEFAULT_KNEE_THRESHOLD);
    // refinement_range_delta is now calculated adaptively below using standard deviation
    let max_value = config.film_grain_max_value.unwrap_or(DEFAULT_MAX_VALUE);
    let refinement_points_count = config.film_grain_refinement_points_count.unwrap_or(DEFAULT_REFINEMENT_POINTS_COUNT);

    // --- Convert initial values to HashSet for quick lookups ---
    let initial_grain_values_set: HashSet<u8> = initial_grain_values_slice.iter().cloned().collect();

    // Basic validation
    if sample_count == 0 || initial_grain_values_set.is_empty() {
        log_callback("[WARN] Film grain optimization requires at least one sample and initial value. Using fallback.");
        return Ok(fallback_value);
    }
     if !initial_grain_values_set.contains(&0) {
         log_callback("[WARN] Initial grain values must include 0 for baseline comparison. Using fallback.");
         return Ok(fallback_value);
     }
     if metric_type != crate::FilmGrainMetricType::KneePoint {
         log_callback(&format!("[WARN] Unsupported film grain metric type configured: {:?}. Only KneePoint is currently implemented. Using fallback.", metric_type));
         return Ok(fallback_value);
     }


    // --- Get Video Duration ---
    log_callback("[INFO] Detecting video duration...");
    let total_duration_secs = duration_fetcher(input_path)?;
    log_callback(&format!("[INFO] Video duration: {:.2} seconds", total_duration_secs));

    if total_duration_secs < (sample_duration * sample_count as u32) as f64 {
        log_callback("[WARN] Video duration is too short for the requested number of samples. Using fallback.");
        return Ok(fallback_value);
    }

    // --- Calculate Sample Positions ---
    let mut sample_start_times = Vec::with_capacity(sample_count);
    let interval = total_duration_secs / (sample_count + 1) as f64;
    for i in 1..=sample_count {
        sample_start_times.push(interval * i as f64);
    }

    // --- Phase 1: Test Initial Values ---
    log_callback(&format!("[INFO] Starting Film Grain Optimization - Phase 1: Initial Testing (Values: {:?})", initial_grain_values_slice));
    let mut phase1_results: AllResults = Vec::with_capacity(sample_count);

    for (i, &start_time) in sample_start_times.iter().enumerate() {
        log_callback(&format!(
            "[INFO] Analyzing sample {}/{} (at {:.2}s)...",
            i + 1, sample_count, start_time
        ));
        let mut sample_results: SampleResult = Vec::with_capacity(initial_grain_values_set.len());

        // Test initial values (iterate slice for order, use set for contains checks later)
        for &grain_value in initial_grain_values_slice {
            log_callback(&format!("[INFO]   Testing grain value {}...", grain_value));
            match sample_tester(input_path, start_time, sample_duration, grain_value, config) {
                Ok(file_size) => {
                    sample_results.push(GrainTest { grain_value, file_size });
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
    let mut initial_estimates: Vec<u8> = Vec::with_capacity(sample_count);
    for (i, sample_results) in phase1_results.iter().enumerate() {
        // Pass a mutable reference to log_callback
        let estimate = calculate_knee_point_grain(sample_results, knee_threshold, &mut log_callback, i);
        initial_estimates.push(estimate);
    }
    log_callback(&format!("[INFO] Phase 2 Initial estimates per sample: {:?}", initial_estimates));

    // --- Phase 3: Focused Refinement ---
    let mut phase3_results: AllResults = Vec::with_capacity(sample_count); // Initialize even if refinement is skipped
    let mut refined_grain_values: Vec<u8> = Vec::new(); // Initialize

    if initial_estimates.is_empty() {
        log_callback("[WARN] No initial estimates generated in Phase 2. Skipping refinement.");
    } else {
        let mut estimates_for_median = initial_estimates.clone();
        let median_estimate = calculate_median(&mut estimates_for_median);
        log_callback(&format!("[INFO] Phase 3: Median of initial estimates: {}", median_estimate));

        // --- Adaptive Refinement Delta Calculation ---
        let std_dev_opt = calculate_std_dev(&initial_estimates);
        const ADAPTIVE_DELTA_FACTOR: f64 = 1.5; // Factor to scale std dev by
        const MIN_ADAPTIVE_DELTA: u8 = 1; // Minimum delta when std dev > 0

        let adaptive_refinement_delta = match std_dev_opt {
            Some(std_dev) if std_dev.is_finite() && std_dev > 0.0 => {
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
                 log_callback(&format!("[DEBUG] Phase 3: Calculated standard deviation: {:.2}, Scaled delta factor: {}, Raw adaptive delta: {}", std_dev, ADAPTIVE_DELTA_FACTOR, delta));
                 delta
            }
            _ => {
                // Fallback if std dev is zero, NaN, infinite, or calculated from < 2 estimates
                log_callback(&format!("[DEBUG] Phase 3: Could not calculate valid standard deviation or std dev is zero/NaN/infinite. Using default delta: {}", DEFAULT_REFINEMENT_RANGE_DELTA));
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

        // Generate refinement points within the range, excluding initial values
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
                     refined_grain_values.push(clamped_point);
                 }
             }
             // Ensure refined values are sorted for logging/processing consistency
             refined_grain_values.sort_unstable();
        }


        if refined_grain_values.is_empty() {
            log_callback("[INFO] Phase 3: No suitable refinement points generated (or range too small). Skipping refined testing.");
        } else {
            log_callback(&format!("[INFO] Phase 3: Testing refined grain values: {:?}", refined_grain_values));

            // Initialize phase3_results with empty vectors for each sample
            for _ in 0..sample_count {
                phase3_results.push(Vec::with_capacity(refined_grain_values.len()));
            }

            // Test refined values for all samples
            for &grain_value in &refined_grain_values {
                 log_callback(&format!("[INFO]   Testing refined grain value {}...", grain_value));
                 for (i, &start_time) in sample_start_times.iter().enumerate() {
                     match sample_tester(input_path, start_time, sample_duration, grain_value, config) {
                         Ok(file_size) => {
                             // Add result to the correct sample's vector in phase3_results
                             phase3_results[i].push(GrainTest { grain_value, file_size });
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
    }


    // --- Phase 4: Final Selection ---
    log_callback("[INFO] Phase 4: Determining final optimal grain using Knee Point on combined results...");
    let mut final_estimates: Vec<u8> = Vec::with_capacity(sample_count);

    for i in 0..sample_count {
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
        let final_estimate = calculate_knee_point_grain(&combined_results, knee_threshold, &mut log_callback, i);
        final_estimates.push(final_estimate);
    }

    log_callback(&format!("[INFO] Phase 4 Final estimates per sample: {:?}", final_estimates));

    // --- Final Result ---
    if final_estimates.is_empty() {
        log_callback("[WARN] No final estimates generated. Using fallback value.");
        return Ok(fallback_value);
    }

    let mut final_estimates_for_median = final_estimates; // Can consume this now
    let median_value = calculate_median(&mut final_estimates_for_median);
    let capped_median_value = median_value.min(max_value); // Apply final cap

    log_callback(&format!(
        "[INFO] Final Result: Median optimal grain: {}. Capped at {}: {}",
        median_value, max_value, capped_median_value
    ));

    Ok(capped_median_value)
}


#[cfg(test)]
mod tests {
    use super::*; // Import items from the parent module (film_grain.rs)
    use std::sync::{Arc, Mutex};

    // Mock log callback that collects messages (copied from integration tests)
    fn collecting_log_callback(log_messages: Arc<Mutex<Vec<String>>>) -> impl FnMut(&str) {
        move |msg: &str| {
            let mut logs = log_messages.lock().unwrap();
            logs.push(msg.to_string());
            // Optionally print to console during test run for debugging
            // println!("LOG: {}", msg);
        }
    }

    #[test]
    fn test_calculate_std_dev() {
        // Now uses the function directly from the parent module
        assert_eq!(calculate_std_dev(&[]), None, "Empty slice");
        assert_eq!(calculate_std_dev(&[5]), None, "Single element");
        assert_eq!(calculate_std_dev(&[5, 5, 5]), Some(0.0), "Identical elements");

        // Known standard deviation (Sample std dev of [1, 2, 3])
        // Mean = 2. Variance = [(1-2)^2 + (2-2)^2 + (3-2)^2] / (3-1) = [1 + 0 + 1] / 2 = 1. Std Dev = sqrt(1) = 1.
        assert_eq!(calculate_std_dev(&[1, 2, 3]), Some(1.0), "Simple case [1, 2, 3]");

        // Another case [2, 4, 4, 4, 5, 5, 7, 9] -> Mean=5, Var=4, StdDev=2
        // Source: https://www.calculator.net/standard-deviation-calculator.html?numberinputs=2%2C+4%2C+4%2C+4%2C+5%2C+5%2C+7%2C+9&ctype=s&x=Calculate
         let values = vec![2u8, 4, 4, 4, 5, 5, 7, 9];
         let std_dev = calculate_std_dev(&values).expect("Std dev calculation failed");
         assert!((std_dev - 2.138089935299395).abs() < 1e-9, "Case [2, 4, 4, 4, 5, 5, 7, 9], expected ~2.138, got {}", std_dev);

         // Case with large numbers (still u8)
         let values_large = vec![100u8, 110, 120, 130, 140];
         // Mean = 120. Var = [( -20)^2 + (-10)^2 + 0^2 + 10^2 + 20^2] / 4 = [400+100+0+100+400]/4 = 1000/4 = 250. StdDev = sqrt(250) ~ 15.811
         let std_dev_large = calculate_std_dev(&values_large).expect("Std dev calculation failed");
         assert!((std_dev_large - 15.8113883).abs() < 1e-6, "Case large values, expected ~15.811, got {}", std_dev_large);
    }


    #[test]
    fn test_calculate_knee_point_grain_logic() {
         // Now uses the function and types directly from the parent module
         let log_messages = Arc::new(Mutex::new(Vec::new()));
         let mut logger = collecting_log_callback(log_messages.clone());
         let knee_threshold = 0.8; // Standard 80%

        // Case 1: Clear knee point
        let results1: SampleResult = vec![
            GrainTest { grain_value: 0, file_size: 10000 },
            GrainTest { grain_value: 5, file_size: 8000 }, // Eff: (2000)/5 = 400 (Max)
            GrainTest { grain_value: 10, file_size: 7500 }, // Eff: (2500)/10 = 250 (Below 0.8*400=320)
            GrainTest { grain_value: 15, file_size: 7400 }, // Eff: (2600)/15 = 173.3 (Below 320)
        ];
        assert_eq!(calculate_knee_point_grain(&results1, knee_threshold, &mut logger, 0), 5, "Case 1: Clear knee point at 5");

        // Case 2: Fallback - No candidates meet threshold
        let results2: SampleResult = vec![
            GrainTest { grain_value: 0, file_size: 10000 },
            GrainTest { grain_value: 5, file_size: 9000 }, // Eff: (1000)/5 = 200 (Max)
            GrainTest { grain_value: 10, file_size: 8500 }, // Eff: (1500)/10 = 150 (Below 0.8*200=160)
            GrainTest { grain_value: 15, file_size: 8200 }, // Eff: (1800)/15 = 120 (Below 160)
        ];
         // Expected: Max eff=200@5. Threshold=160. Candidates: [5(200)]. Lowest=5. Fallback NOT triggered.
        assert_eq!(calculate_knee_point_grain(&results2, knee_threshold, &mut logger, 1), 5, "Case 2: Single candidate meets threshold");
        { // Scope to ensure lock is dropped before next call
            // Fallback log message should NOT be present here.
            let mut logs2 = log_messages.lock().unwrap();
            // assert!(logs2.iter().any(|m| m.contains("Sample 2:") && m.contains("No candidates met threshold >= 160.0. Falling back to default grain: 0")), "Case 2: Log missing fallback message"); // This check was incorrect for this data
            logs2.clear(); // Clear logs for next case
        }

        // Case 3: No positive efficiency
         let results3: SampleResult = vec![
            GrainTest { grain_value: 0, file_size: 10000 },
            GrainTest { grain_value: 5, file_size: 10000 },
            GrainTest { grain_value: 10, file_size: 10500 }, // Increased size
        ];
        assert_eq!(calculate_knee_point_grain(&results3, knee_threshold, &mut logger, 2), 0, "Case 3: No positive efficiency");
         { // Scope to ensure lock is dropped before next call
             let mut logs3 = log_messages.lock().unwrap();
             assert!(logs3.iter().any(|m| m.contains("Sample 3:") && m.contains("No positive efficiency improvements found. Optimal grain: 0")), "Case 3: Log missing no efficiency message");
             logs3.clear();
         }

        // Case 4: Base size missing or zero
         let results4a: SampleResult = vec![ // Missing grain 0
            GrainTest { grain_value: 5, file_size: 8000 },
            GrainTest { grain_value: 10, file_size: 7500 },
        ];
         let results4b: SampleResult = vec![ // Grain 0 has size 0
            GrainTest { grain_value: 0, file_size: 0 },
            GrainTest { grain_value: 5, file_size: 8000 },
        ];
        assert_eq!(calculate_knee_point_grain(&results4a, knee_threshold, &mut logger, 3), 0, "Case 4a: Base size missing");
         { // Scope to ensure lock is dropped before next call
             let mut logs4a = log_messages.lock().unwrap();
             assert!(logs4a.iter().any(|m| m.contains("Sample 4:") && m.contains("Base size (grain=0) is zero or missing.")), "Case 4a: Log missing base size warning");
             logs4a.clear();
         }

        assert_eq!(calculate_knee_point_grain(&results4b, knee_threshold, &mut logger, 4), 0, "Case 4b: Base size zero");
         { // Scope to ensure lock is dropped before next call
             let mut logs4b = log_messages.lock().unwrap();
             assert!(logs4b.iter().any(|m| m.contains("Sample 5:") && m.contains("Base size (grain=0) is zero or missing.")), "Case 4b: Log missing base size warning");
             logs4b.clear();
         }

         // Case 5: Multiple candidates, choose lowest grain
         let results5: SampleResult = vec![
            GrainTest { grain_value: 0, file_size: 10000 },
            GrainTest { grain_value: 4, file_size: 8000 }, // Eff: 2000/4 = 500 (Max)
            GrainTest { grain_value: 5, file_size: 7900 }, // Eff: 2100/5 = 420 (>= 0.8*500=400)
            GrainTest { grain_value: 7, file_size: 7800 }, // Eff: 2200/7 = 314 (Below 400)
            GrainTest { grain_value: 6, file_size: 7850 }, // Eff: 2150/6 = 358 (Below 400) - Add out of order
        ];
         // Need to sort results before passing if function expects it (it does internally via filter/collect/sort)
         // Expected: Max=500@4. Threshold=400. Candidates: [4(500), 5(420)]. Lowest grain = 4.
         // Note: The internal logic sorts candidates by grain, so order in input vec doesn't matter here.
         assert_eq!(calculate_knee_point_grain(&results5, knee_threshold, &mut logger, 5), 4, "Case 5: Multiple candidates, choose lowest grain 4");
// Case 6 removed as the fallback condition is unreachable with knee_threshold <= 1.0
    }
}

// NOTE: The original call to determine_final_grain_value at the very end of the function needs to be removed. (Comment remains relevant if needed)

    

    // Obsolete Phase 2 logic removed. Refined testing is now handled within the main function body earlier.


// Redundant final analysis block removed. It's now handled within the main function body after Phase 3.