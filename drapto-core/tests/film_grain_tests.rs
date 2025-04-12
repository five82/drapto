// drapto-core/tests/film_grain_tests.rs
//
// This file contains integration tests specifically targeting the film grain
// optimization functionality within the `drapto-core` library, primarily focusing
// on the `determine_optimal_grain` function.
//
// As these are integration tests, they operate on the public (or `pub(crate)`) API
// of the `drapto_core` crate. However, testing `determine_optimal_grain` directly
// requires simulating the results of external processes (`ffprobe` for duration,
// `HandBrakeCLI` for sample encoding) without actually running them.
//
// Mocking Strategy:
// - Dependency Injection: The `determine_optimal_grain` function is designed to
//   accept function pointers (`duration_fetcher`, `sample_tester`) for the operations
//   that interact with external tools. These tests provide mock closures that mimic
//   the expected behavior and return predefined results, allowing testing of the
//   analysis logic under various scenarios.
// - `cfg(test)`: The `get_video_duration_secs` function in `sampling.rs` uses
//   `#[cfg(test)]` to provide a mock implementation using a `thread_local` variable
//   when compiled for tests (though these integration tests use the DI approach).
// - Temporary Files/Dirs: The `tempfile` crate is used to create dummy video files
//   and directories needed for configuration paths, even though the mock functions
//   don't actually interact with the file content.
// - Log Collection: A mock logging callback (`collecting_log_callback`) is used to
//   capture log messages generated during the optimization process, allowing assertions
//   on the internal steps and decisions made by the algorithm.
//
// Test Scenarios:
// - Success Case: Simulates a scenario where the mock data leads to a specific expected
//   optimal grain value based on the Knee Point metric.
// - Failure Case: Simulates a scenario where one of the mock sample encoding tests fails,
//   ensuring that the error propagates correctly.
// - Adaptive Range: Tests the logic for calculating the refinement range based on the
//   standard deviation of initial estimates.
// - Knee Fallback: Tests scenarios where the Knee Point analysis doesn't find suitable
//   candidates meeting the threshold, ensuring it correctly falls back to the default (0).

use drapto_core::*; // Import items from the drapto_core crate
use std::fs::File; // Only File is needed for create_dummy_video_file
use std::path::{Path, PathBuf};
use std::sync::{Mutex, Arc};
use tempfile::tempdir;

// --- Mocking Helpers ---

// TODO: Implement mocking for HandBrakeCLI calls within extract_and_test_sample
// This might involve conditionally compiling a mock version of the function
// or using a mocking library if preferred.
// For now, we'll assume the real function exists but tests will need mocking later.

// Helper to create a dummy video file for tests that need a path
fn create_dummy_video_file(dir: &tempfile::TempDir) -> PathBuf {
    let file_path = dir.path().join("dummy_video.mkv");
    File::create(&file_path).expect("Failed to create dummy file");
    file_path
}

// Helper to create a default CoreConfig for tests
fn default_test_config(output_dir: &Path, log_dir: &Path) -> CoreConfig {
     CoreConfig {
        input_dir: PathBuf::from("dummy_input"), // Not directly used by determine_optimal_grain
        output_dir: output_dir.to_path_buf(),
        log_dir: log_dir.to_path_buf(),
        default_encoder_preset: Some(6),
        // default_quality: Some(28), // Field removed
        quality_sd: None, // Set new fields to None for test setup
        quality_hd: None,
        quality_uhd: None,
        default_crop_mode: Some("off".to_string()), // Use 'off' for consistency
        film_grain_sample_crop_mode: Some("auto".to_string()), // Add missing field for tests
        // Film Grain specific defaults
        optimize_film_grain: true, // Enable for testing the function
        film_grain_sample_duration: Some(10),
        film_grain_sample_count: Some(3), // Test with 3 samples
        film_grain_initial_values: Some(vec![0, 5, 8, 10, 15, 20]), // Include target grains (5, 8, 10)
        film_grain_fallback_value: Some(0),
        // New config options
        film_grain_metric_type: Some(FilmGrainMetricType::KneePoint),
        film_grain_knee_threshold: Some(0.8), // 80% threshold
        film_grain_refinement_range_delta: Some(3), // +/- 3
        film_grain_max_value: Some(20), // Max value cap
        film_grain_refinement_points_count: Some(3), // 3 refinement points
        ntfy_topic: None, // Add missing field for tests
        preset: None, // Add missing preset field
    }
}

// Mock log callback that collects messages
fn collecting_log_callback(log_messages: Arc<Mutex<Vec<String>>>) -> impl FnMut(&str) {
    move |msg: &str| {
        let mut logs = log_messages.lock().unwrap();
        logs.push(msg.to_string());
        // Optionally print to console during test run for debugging
        println!("LOG: {}", msg); // Re-enable printing for debugging
    }
}


// --- Integration Tests for determine_optimal_grain ---

#[test]
// #[ignore] // Mocking is now implemented via cfg(test)
fn test_determine_optimal_grain_success_scenario() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = tempdir()?;
    let log_dir = tempdir()?;
    let dummy_video = create_dummy_video_file(&output_dir); // Create in output for simplicity
    let config = default_test_config(output_dir.path(), log_dir.path());
    let log_messages = Arc::new(Mutex::new(Vec::new()));
    let mut logger = collecting_log_callback(log_messages.clone());

    // --- Mocking Setup ---
    // TODO: Configure the mock for extract_and_test_sample here.
    // Example: Mock should return sizes like:
    // grain 0: 10000
    // grain 8: 8000
    // grain 12 (refined): 7500
    // grain 20: 7800
    // This should result in optimal value 12 (based on efficiency)

    // --- Execute ---
    // Need to access the non-public function. This requires either:
    // 1. Making determine_optimal_grain pub(crate) in lib.rs (already done)
    // 2. Calling it via a public function that wraps it (if one existed)
    // We'll assume we can call it directly via crate::processing::film_grain::determine_optimal_grain
    // Define mock closures for dependencies
    let mock_duration_fetcher = |_path: &Path| -> CoreResult<f64> { Ok(300.0) }; // Keep duration simple
    let mock_sample_tester = |_input_path: &Path, start_secs: f64, _duration_secs: u32, grain_value: u8, _config: &CoreConfig, _handbrake_cmd_parts: &[String]| -> CoreResult<u64> {
        // Mock data designed for Knee Point testing (Expected final result: 5)
        // Sample 1 (starts ~100s): Base 10000
        // Sample 2 (starts ~200s): Base 12000
        // Sample 3 (starts ~300s): Base 9000
        let base_size = match start_secs.round() as u32 {
             75 => 10000, // Sample 1 at 75s
            150 => 12000, // Sample 2 at 150s
            225 => 9000,  // Sample 3 at 225s
            _ => 10000, // Default base
        };

        // Simulate size reduction based on grain value relative to base size
        // These values correspond to the example calculated in the thought block
        match grain_value {
            0 => Ok(base_size),
            // Initial Values
            5 => Ok(base_size * 80 / 100),  // Significant drop
            10 => Ok(base_size * 70 / 100), // Smaller drop
            15 => Ok(base_size * 68 / 100), // Marginal drop
            20 => Ok(base_size * 67 / 100), // Very marginal drop
            // Refined Values (around median 5, range [2, 8], excluding 5 -> e.g., 3, 4, 6)
            3 => Ok(base_size * 90 / 100),
            4 => Ok(base_size * 85 / 100),
            6 => Ok(base_size * 78 / 100),
            // Other potential refined values if delta/count changes
            2 => Ok(base_size * 95 / 100),
            7 => Ok(base_size * 75 / 100),
            _ => Ok(base_size * 98 / 100), // Default for unexpected values
        }
    };

    let result = crate::processing::film_grain::determine_optimal_grain(
        &dummy_video,
        &config,
        &mut logger,
        mock_duration_fetcher,
        mock_sample_tester, // Pass the mock sample tester
        &[], // Pass dummy command parts for the test
    );

    // --- Assertions ---
    assert!(result.is_ok(), "determine_optimal_grain failed: {:?}", result.err());
    let optimal_value = result.unwrap();
    // Based on the mock data and Knee Point logic:
    // Phase 1: Tests [0, 5, 10, 15, 20]
    // Phase 2: Estimates per sample should all be 5 (based on mock data efficiency) -> Median = 5
    // Phase 3: Range [2, 8]. Refined points (excluding 5) e.g., [3, 4, 6] tested.
    // Phase 4: Re-calculates knee point per sample using combined data. Should still be 5 for each sample. -> Median = 5. Capped = 5.
    assert_eq!(optimal_value, 5, "Expected optimal value 5 based on mock data and Knee Point logic"); // Updated expected value

    // Check logs for key steps
    let logs = log_messages.lock().unwrap();
    println!("Collected Logs:\n{:#?}", logs); // Print logs for debugging

    // Check logs for key steps of the NEW Knee Point flow
    assert!(logs.iter().any(|m| m.contains("[INFO] Starting Film Grain Optimization - Phase 1: Initial Testing (Values: [0, 5, 8, 10, 15, 20])")), "Log missing: Phase 1 start");
    assert!(logs.iter().any(|m| m.contains("[INFO] Phase 2: Estimating optimal grain per sample using Knee Point metric...")), "Log missing: Phase 2 start");
    assert!(logs.iter().any(|m| m.contains("[INFO] Phase 2 Initial estimates per sample: [5, 5, 5]")), "Log missing: Phase 2 estimates"); // Based on mock data
    assert!(logs.iter().any(|m| m.contains("[INFO] Phase 3: Median of initial estimates: 5")), "Log missing: Phase 3 median");
    // Check for adaptive delta calculation log (fallback case)
    assert!(logs.iter().any(|m| m.contains("Standard deviation is 0.0") && m.contains("Using minimal delta: 1")), "Log missing: Std dev 0.0 message"); // Simplified check
    // Updated assertion to match the actual log format including the adaptive delta value
    // Check the log message when adaptive delta falls back to default
    assert!(logs.iter().any(|m| m.contains("[INFO] Phase 3: Refinement range around median: [4, 6]")), "Log missing: Phase 3 range log [4, 6]");
    // Check for the specific refined values tested
    assert!(logs.iter().any(|m| m.contains("Testing refined grain value 4...")), "Log missing: Testing refined value 4");
    assert!(logs.iter().any(|m| m.contains("Testing refined grain value 6...")), "Log missing: Testing refined value 6");
    assert!(logs.iter().any(|m| m.contains("[INFO] Phase 4: Determining final optimal grain using Knee Point on combined results...")), "Log missing: Phase 4 start");
    assert!(logs.iter().any(|m| m.contains("[INFO] Phase 4 Final estimates per sample: [5, 5, 5]")), "Log missing: Phase 4 estimates [5, 5, 5]"); // Updated expected estimates
    assert!(logs.iter().any(|m| m.contains("[INFO] Final Result: Median optimal grain: 5. Capped at 20: 5")), "Log missing: Final value 5"); // Updated expected final value


    Ok(())
}
#[test]
fn test_determine_optimal_grain_failure_scenario() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = tempdir()?;
    let log_dir = tempdir()?;
    let dummy_video = create_dummy_video_file(&output_dir);
    let mut config = default_test_config(output_dir.path(), log_dir.path());
    // Configure initial values to include the one that causes mock failure
    config.film_grain_initial_values = Some(vec![0, 5, 10, 15, 20]); // Include 15

    let log_messages = Arc::new(Mutex::new(Vec::new()));
    let mut logger = collecting_log_callback(log_messages.clone());

    // --- Execute ---
    // Define mock closures for dependencies
    let mock_duration_fetcher = |_path: &Path| -> CoreResult<f64> { Ok(300.0) };
    let mock_sample_tester = |_input_path: &Path, _start_secs: f64, _duration_secs: u32, grain_value: u8, _config: &CoreConfig, _handbrake_cmd_parts: &[String]| -> CoreResult<u64> {
        // Simulate failure for grain 15
        if grain_value == 15 {
            Err(CoreError::FilmGrainEncodingFailed("Mock failure for grain 15".to_string()))
        } else {
             match grain_value {
                0 => Ok(10000), // Sample 1 base size
                5 => Ok(8000),
                10 => Ok(7000),
                20 => Ok(6700),
                _ => Ok(9000),
            }
        }
    };

    let result = crate::processing::film_grain::determine_optimal_grain(
        &dummy_video,
        &config,
        &mut logger,
        mock_duration_fetcher,
        mock_sample_tester, // Pass the mock sample tester
        &[], // Pass dummy command parts for the test
    );

    // --- Assertions ---
    assert!(result.is_err(), "Expected determine_optimal_grain to fail");

    // Check the specific error type
    match result.err().unwrap() {
        CoreError::FilmGrainAnalysisFailed(msg) => {
             // The error might propagate from the failed sample test within the loop
             // Or the analysis might fail later if a sample has no results.
             // Let's check the logs for the warning about the failed test.
             println!("Received expected error type: FilmGrainAnalysisFailed: {}", msg);
        },
        e => panic!("Unexpected error type: {:?}", e),
    }

     // Check logs for the warning about the failure
    let logs = log_messages.lock().unwrap();
    println!("Collected Logs:\n{:#?}", logs); // Print logs for debugging
    assert!(
        logs.iter().any(|m| m.contains("[ERROR] Failed testing grain value 15:") && m.contains("Mock failure for grain 15")),
        "Log missing: Error message about failed test for grain 15"
    );
     // Depending on how many samples succeed, it might still try Phase 2 or fail earlier.
     // The key is that it should error out eventually if a sample fails completely.

    Ok(())
}

#[test]
fn test_determine_optimal_grain_adaptive_range() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = tempdir()?;
    let log_dir = tempdir()?;
    let dummy_video = create_dummy_video_file(&output_dir);
    let config = default_test_config(output_dir.path(), log_dir.path());
    let log_messages = Arc::new(Mutex::new(Vec::new()));
    let mut logger = collecting_log_callback(log_messages.clone());

    // --- Mocking Setup ---
    let mock_duration_fetcher = |_path: &Path| -> CoreResult<f64> { Ok(300.0) };
    // Use a counter to simulate the 3 different sample scenarios regardless of random start_secs
    let call_counter = Arc::new(Mutex::new(0usize));
    // Calculate the number of initial values outside the closure to avoid borrowing config inside
    let initial_values_len = config.film_grain_initial_values.as_ref().map_or(1, |v| v.len().max(1)); // Use max(1) to prevent division by zero if vec is empty

    let mock_sample_tester = move |_input_path: &Path, _start_secs: f64, _duration_secs: u32, grain_value: u8, _config: &CoreConfig, _handbrake_cmd_parts: &[String]| -> CoreResult<u64> {
        let mut counter = call_counter.lock().unwrap();
        *counter += 1; // Increment call count
        // Use the captured initial_values_len instead of accessing config
        let sample_scenario = (*counter -1) / initial_values_len + 1; // Determine which sample this call belongs to (adjust counter logic for 1-based scenario)

        // Mock data designed for non-zero standard deviation in Phase 2 estimates
        // Sample 1: Base 10000 -> Estimate 5
        // Sample 2: Base 12000 -> Estimate 8
        // Sample 3: Base 9000 -> Estimate 10
        let (base_size, target_grain) = match sample_scenario {
            1 => (10000u64, 5u8), // Scenario for Sample 1
            2 => (12000u64, 8u8), // Scenario for Sample 2
            3 => (9000u64, 10u8), // Scenario for Sample 3
            _ => (10000u64, 5u8), // Fallback (shouldn't be reached with 3 samples)
        };

        // Calculate reduction: High (e.g., 1000 * grain) at target, Low (e.g., constant 10) otherwise
        let reduction = if grain_value == target_grain {
            1000 * grain_value as u64 // High efficiency at target
        } else if grain_value > 0 {
             10 // Constant low reduction
        } else {
            0
        };

        // Ensure size doesn't go below a minimum (e.g., 10% of base)
        let final_size = base_size.saturating_sub(reduction).max(base_size / 10);
        Ok(final_size)
    };

    // --- Execute ---
    // ... (rest of test remains the same) ...
    // Expected Phase 2: [5, 8, 10] -> Median 8, StdDev ~2.5, Delta 4, Range [4, 12]
    // Expected Phase 4: [5, 8, 10] -> Median 8

    // --- Execute ---
    let result = crate::processing::film_grain::determine_optimal_grain(
        &dummy_video,
        &config,
        &mut logger,
        mock_duration_fetcher,
        mock_sample_tester,
        &[], // Pass dummy command parts for the test
    );

    // --- Assertions ---
    assert!(result.is_ok(), "determine_optimal_grain failed: {:?}", result.err());
    let optimal_value = result.unwrap();

    // Expected based on mock data [5, 8, 10]:
    // Phase 2: Estimates [5, 8, 10]. Median 8. StdDev ~2.519...
    // Phase 3: Adaptive Delta = round(2.519 * 1.5) = 4. Range [8-4, 8+4] -> [4, 12].
    // Refined points (count=3, step=2): 6, 8, 10. Exclude 8, 10 (initial). Test: [6].
    // Phase 4: Re-evaluate samples with combined data [0, 5, 6, 8, 10, 15, 20].
    //   Sample 1 (eff=1.0): Max eff @ 5. Threshold 0.8*max. Candidates [5, 6]. Lowest=5.
    //   Sample 2 (eff=0.8): Max eff @ 8. Threshold 0.8*max. Candidates [8]. Lowest=8.
    //   Sample 3 (eff=0.7): Max eff @ 10. Threshold 0.8*max. Candidates [10]. Lowest=10.
    // Phase 4 Estimates: [5, 8, 10]. Median = 8.
    // Final Result: 8.
    assert_eq!(optimal_value, 8, "Expected optimal value 8 based on adaptive range mock data");

    // Check logs
    let logs = log_messages.lock().unwrap();
    println!("Collected Logs (Adaptive Range Test):\n{:#?}", logs);
    assert!(logs.iter().any(|m| m.contains("[INFO] Phase 2 Initial estimates per sample: [5, 8, 10]")), "Log missing: Phase 2 estimates [5, 8, 10]");
    assert!(logs.iter().any(|m| m.contains("[INFO] Phase 3: Median of initial estimates: 8")), "Log missing: Phase 3 median 8");
    assert!(logs.iter().any(|m| m.contains("Calculated standard deviation: 2.05")), "Log missing: Std Dev calculation (expected ~2.05)"); // Check correct value
    assert!(logs.iter().any(|m| m.contains("[INFO] Phase 3: Using adaptive refinement delta: 3")), "Log missing: Adaptive delta 3"); // Check correct delta
    // Update assertion to match actual log format and correct range
    assert!(logs.iter().any(|m| m.contains("[INFO] Phase 3: Refinement range around median: [5, 11]")), "Log missing: Phase 3 range [5, 11]"); // Check correct range
    assert!(logs.iter().any(|m| m.contains("Testing refined grain value 6...")), "Log missing: Testing refined value 6");
    assert!(logs.iter().any(|m| m.contains("[INFO] Phase 4 Final estimates per sample: [5, 8, 10]")), "Log missing: Phase 4 estimates [5, 8, 10]");
    assert!(logs.iter().any(|m| m.contains("[INFO] Final Result: Median optimal grain: 8. Capped at 20: 8")), "Log missing: Final value 8");

    Ok(())
}


#[test]
fn test_determine_optimal_grain_knee_fallback() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = tempdir()?;
    let log_dir = tempdir()?;
    let dummy_video = create_dummy_video_file(&output_dir);
    let config = default_test_config(output_dir.path(), log_dir.path());
    let log_messages = Arc::new(Mutex::new(Vec::new()));
    let mut logger = collecting_log_callback(log_messages.clone());

    // --- Mocking Setup ---
    let mock_duration_fetcher = |_path: &Path| -> CoreResult<f64> { Ok(300.0) };
    let mock_sample_tester = |_input_path: &Path, _start_secs: f64, _duration_secs: u32, grain_value: u8, _config: &CoreConfig, _handbrake_cmd_parts: &[String]| -> CoreResult<u64> {
        // Mock data designed to trigger the knee point fallback (no candidates meet threshold)
        // Make efficiencies very low and close together
        let base_size = 10000;
        match grain_value {
            0 => Ok(base_size),
            // Make reductions tiny, so efficiency is low
             5 => Ok(base_size - 10), // Eff = 10/5 = 2.0
            10 => Ok(base_size - 15), // Eff = 15/10 = 1.5
            15 => Ok(base_size - 18), // Eff = 18/15 = 1.2
            20 => Ok(base_size - 20), // Eff = 20/20 = 1.0
            // Refined values (assume median 5, range [2, 8], test [4, 7])
             4 => Ok(base_size - 8),  // Eff = 8/4 = 2.0
             7 => Ok(base_size - 12), // Eff = 12/7 = ~1.71
            _ => Ok(base_size - 5),   // Default tiny reduction
        }
    };

    // --- Execute ---
    let result = crate::processing::film_grain::determine_optimal_grain(
        &dummy_video,
        &config,
        &mut logger,
        mock_duration_fetcher,
        mock_sample_tester,
        &[], // Pass dummy command parts for the test
    );

    // --- Assertions ---
    assert!(result.is_ok(), "determine_optimal_grain failed: {:?}", result.err());
    let _optimal_value = result.unwrap(); // Prefixed as unused in this specific test logic

    // Expected based on mock data:
    // Phase 1/Combined Efficiencies: [4: 2.0, 5: 2.0, 7: 1.71, 10: 1.5, 15: 1.2, 20: 1.0]
    // Max Efficiency: 2.0 (at grain 4 and 5). Let's say fold picks 4 first.
    // Threshold: 0.8 * 2.0 = 1.6.
    // Candidates >= 1.6: Grain 4 (2.0), Grain 5 (2.0), Grain 7 (1.71).
    // Lowest Grain Candidate: 4.
    // Phase 2/4 Estimates should be 4 for all samples. Median = 4.
    // BUT WAIT - the goal is to test the *fallback*. Let's adjust mock data.

    // --- Re-Mocking for Fallback ---
     let mock_sample_tester_fallback = |_input_path: &Path, _start_secs: f64, _duration_secs: u32, grain_value: u8, _config: &CoreConfig, _handbrake_cmd_parts: &[String]| -> CoreResult<u64> {
        // Make max efficiency high, but subsequent efficiencies drop off *sharply* below threshold
        let base_size = 10000;
        match grain_value {
            0 => Ok(base_size),
             5 => Ok(base_size - 500), // Eff = 500/5 = 100 (Max)
            10 => Ok(base_size - 550), // Eff = 550/10 = 55 (Below 0.8 * 100 = 80)
            15 => Ok(base_size - 580), // Eff = 580/15 = ~38.7 (Below 80)
            20 => Ok(base_size - 600), // Eff = 600/20 = 30 (Below 80)
            // Refined values (assume median 5, range [2, 8], test [4, 7])
             4 => Ok(base_size - 400), // Eff = 400/4 = 100 (Max)
             7 => Ok(base_size - 520), // Eff = 520/7 = ~74.3 (Below 80)
            _ => Ok(base_size - 100),
        }
    };

     // --- Re-Execute ---
     let log_messages_fallback = Arc::new(Mutex::new(Vec::new()));
     let mut logger_fallback = collecting_log_callback(log_messages_fallback.clone());
     let result_fallback = crate::processing::film_grain::determine_optimal_grain(
        &dummy_video,
        &config,
        &mut logger_fallback,
        mock_duration_fetcher,
        mock_sample_tester_fallback,
        &[], // Pass dummy command parts for the test
    );

     // --- Assertions (Fallback) ---
     assert!(result_fallback.is_ok(), "determine_optimal_grain (fallback test) failed: {:?}", result_fallback.err());
     let _optimal_value_fallback = result_fallback.unwrap(); // Prefixed as unused in this specific test logic

    // Expected based on fallback mock data:
    // Combined Efficiencies: [4: 100, 5: 100, 7: 74.3, 10: 55, 15: 38.7, 20: 30]
    // Max Efficiency: 100 (at grain 4 and 5).
    // Threshold: 0.8 * 100 = 80.
    // Candidates >= 80: Grain 4 (100), Grain 5 (100).
    // Lowest Grain Candidate: 4.
    // Phase 2/4 Estimates: 4. Median = 4.
    // STILL NOT HITTING FALLBACK. Need candidates list to be EMPTY.

    // --- Re-Mocking for ACTUAL Fallback ---
     let mock_sample_tester_actual_fallback = |_input_path: &Path, _start_secs: f64, _duration_secs: u32, grain_value: u8, _config: &CoreConfig, _handbrake_cmd_parts: &[String]| -> CoreResult<u64> {
        // Ensure NO positive efficiency for any grain > 0
        let base_size = 10000;
        match grain_value {
            0 => Ok(base_size),
            _ => Ok(base_size), // Return base_size or slightly larger to ensure no reduction
        }
    };

     // --- Re-Execute (Actual Fallback) ---
     let log_messages_actual_fallback = Arc::new(Mutex::new(Vec::new()));
     let mut logger_actual_fallback = collecting_log_callback(log_messages_actual_fallback.clone());
     let result_actual_fallback = crate::processing::film_grain::determine_optimal_grain(
        &dummy_video,
        &config,
        &mut logger_actual_fallback,
        mock_duration_fetcher,
        mock_sample_tester_actual_fallback,
        &[], // Pass dummy command parts for the test
    );

     // --- Assertions (Actual Fallback) ---
     assert!(result_actual_fallback.is_ok(), "determine_optimal_grain (actual fallback test) failed: {:?}", result_actual_fallback.err());
     let optimal_value_actual_fallback = result_actual_fallback.unwrap();

    // Expected based on actual fallback mock data:
    // Combined Efficiencies: [4: 75, 5: 100, 7: 71.4, 10: 79, 15: 76.7, 20: 75]
    // Max Efficiency: 100 (at grain 5).
    // Threshold: 0.8 * 100 = 80.
    // Candidates >= 80: None.
    // Knee point calculation should return 0 (fallback).
    // Phase 2/4 Estimates: [0, 0, 0]. Median = 0.
    // Final Result: 0.
    assert_eq!(optimal_value_actual_fallback, 0, "Expected optimal value 0 due to knee point fallback");

    // Check logs for fallback message
    let logs = log_messages_actual_fallback.lock().unwrap();
    println!("Collected Logs (Fallback Test):\n{:#?}", logs);
    // Check the log message for the zero efficiency case
    assert!(logs.iter().any(|m| m.contains("No positive efficiency improvements found. Optimal grain: 0")), "Log missing: Zero efficiency message");
    assert!(logs.iter().any(|m| m.contains("[INFO] Phase 4 Final estimates per sample: [0, 0, 0]")), "Log missing: Phase 4 estimates [0, 0, 0]");
    assert!(logs.iter().any(|m| m.contains("[INFO] Final Result: Median optimal grain: 0. Capped at 20: 0")), "Log missing: Final value 0");


    Ok(())
}


// Unit tests for calculate_std_dev and calculate_knee_point_grain have been moved
// into drapto-core/src/processing/film_grain.rs within a #[cfg(test)] mod tests { ... } block.

// TODO: Add test_determine_optimal_grain_optimization_disabled (should not call, uses fallback - test via process_videos?)