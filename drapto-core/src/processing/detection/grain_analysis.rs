// drapto-core/src/processing/detection/grain_analysis.rs
use colored::*; // Import colored for formatting

use crate::config::CoreConfig;
use crate::error::{CoreError, CoreResult};
use crate::external::ffmpeg::{EncodeParams}; // Keep EncodeParams import
use crate::external::ffmpeg_executor::extract_sample; // Keep extract_sample, remove encode_sample_for_grain_test
use crate::external::{FileMetadataProvider, FfmpegSpawner}; // Add missing traits, removed FfmpegProcess
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tempfile::Builder as TempFileBuilder;

/// Represents the detected level of grain in the video, determined by relative encode comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum GrainLevel {
    VeryClean, // Corresponds to no significant benefit from denoising
    VeryLight, // Corresponds to benefit from very light denoising
    Light,     // Corresponds to benefit from light denoising
    Visible,   // Corresponds to benefit from medium denoising
    Heavy,     // Corresponds to benefit from heavy denoising
}

/// Holds the final result of the grain analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrainAnalysisResult {
    /// The final detected grain level based on median of sample analyses.
    pub detected_level: GrainLevel,
}

// Map GrainLevel to specific hqdn3d parameter strings for testing and final application
// Ordered by strength for iteration in analysis logic
const HQDN3D_PARAMS: [(GrainLevel, &str); 4] = [
    (GrainLevel::VeryLight, "hqdn3d=0.5:0.3:3:3"),
    (GrainLevel::Light, "hqdn3d=1:0.7:4:4"),
    (GrainLevel::Visible, "hqdn3d=1.5:1.0:6:6"),
    (GrainLevel::Heavy, "hqdn3d=2:1.3:8:8"),
];

/// Determines the appropriate hqdn3d filter parameter string based on the detected grain level.
/// Returns None if the level is VeryClean.
pub fn determine_hqdn3d_params(level: GrainLevel) -> Option<String> {
    if level == GrainLevel::VeryClean {
        return None;
    }
    // Find the corresponding string in the map
    HQDN3D_PARAMS
        .iter()
        .find(|(l, _)| *l == level)
        .map(|(_, s)| s.to_string())
        .or_else(|| {
            log::warn!("{} Could not find hqdn3d params for level {:?}, this is unexpected.", "Warning:".yellow().bold(), level);
            None
        })
}

// --- Constants for Sampling (Adapted from reference/mod.rs) ---
const DEFAULT_SAMPLE_DURATION_SECS: u32 = 10;
const MIN_SAMPLES: usize = 3; // Updated to match reference
const MAX_SAMPLES: usize = 9; // Updated to match reference
const SECS_PER_SAMPLE_TARGET: f64 = 600.0;

// --- Constants for Relative Analysis (Placeholder) ---
const DIMINISHING_RETURN_FACTOR: f64 = 0.25; // Needs tuning
const MIN_ABSOLUTE_REDUCTION_BYTES: u64 = 50 * 1024; // Needs tuning

/// Analyzes the grain in a video file by encoding samples with different denoise levels
/// and comparing the relative file size reductions.
///
/// Returns `Ok(Some(GrainAnalysisResult))` if analysis is successful,
/// `Ok(None)` if analysis is skipped (e.g., short video),
/// or `Err(CoreError)` if a critical error occurs.
pub fn analyze_grain<S: FfmpegSpawner, P: FileMetadataProvider>(
    file_path: &Path,
    config: &CoreConfig,
    base_encode_params: &EncodeParams,
    duration_secs: f64,
    spawner: &S, // Add spawner back
    metadata_provider: &P, // Add metadata provider
) -> CoreResult<Option<GrainAnalysisResult>> {
    // Extract filename for logging using to_string_lossy for consistent Cow<'_, str> type
    let filename_cow = file_path
        .file_name()
        .map(|name| name.to_string_lossy()) // Returns Cow<'_, str>
        .unwrap_or_else(|| file_path.to_string_lossy()); // Also returns Cow<'_, str>

    log::info!(
        "{} {}",
        "Starting grain analysis (relative sample comparison) for:".cyan().bold(),
        filename_cow.yellow() // Use the Cow (implicitly derefs to &str)
    );
    log::info!(
        "  {:<12} {}", // Left-align label
        "Duration:".cyan(),
        format!("{:.2}s", duration_secs).green()
    );

    // --- Determine Sample Count ---
    let base_samples = (duration_secs / SECS_PER_SAMPLE_TARGET).ceil() as usize;
    let mut num_samples = base_samples.clamp(MIN_SAMPLES, MAX_SAMPLES);
    // Ensure odd number of samples (matches reference logic)
    if num_samples % 2 == 0 {
        num_samples = (num_samples + 1).min(MAX_SAMPLES);
    }
    log::debug!("Calculated number of samples: {}", num_samples);

    // --- Validate Duration for Sampling ---
    let sample_duration = DEFAULT_SAMPLE_DURATION_SECS;
    let min_required_duration = (sample_duration * num_samples as u32) as f64;
    if duration_secs < min_required_duration {
        log::warn!(
            "{} Video duration ({:.2}s) is too short for the minimum required duration ({:.2}s) for {} samples. Skipping grain analysis.",
            "Warning:".yellow().bold(), duration_secs, min_required_duration, num_samples
        );
        return Ok(None);
    }

    // --- Calculate Randomized Sample Positions ---
    let sample_duration_f64 = sample_duration as f64;
    let start_boundary = duration_secs * 0.15;
    let end_boundary = duration_secs * 0.85;
    let latest_possible_start = end_boundary - sample_duration_f64;

    if latest_possible_start <= start_boundary {
        log::warn!(
            "{} Video duration ({:.2}s) results in an invalid sampling window ({:.2}s - {:.2}s) for sample duration {}. Skipping grain analysis.",
            "Warning:".yellow().bold(), duration_secs, start_boundary, end_boundary, sample_duration
        );
        return Ok(None);
    }

    let mut sample_start_times = Vec::with_capacity(num_samples);
    let mut rng = thread_rng();
    for _ in 0..num_samples {
        let start_time = rng.gen_range(start_boundary..=latest_possible_start);
        sample_start_times.push(start_time);
    }
    sample_start_times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    log::debug!("Generated random sample start times: {:?}", sample_start_times);

    // --- Create Temporary Directory ---
    let samples_tmp_base_dir = config.output_dir.join("grain_samples_tmp");
    fs::create_dir_all(&samples_tmp_base_dir).map_err(CoreError::Io)?;
    let temp_dir = TempFileBuilder::new()
        .prefix("analysis_rel_")
        .tempdir_in(&samples_tmp_base_dir)
        .map_err(CoreError::Io)?;
    let temp_dir_path = temp_dir.path();
    log::debug!("Created temporary directory for samples: {}", temp_dir_path.display());

    // --- Process Samples ---
    let mut sample_results: Vec<HashMap<Option<GrainLevel>, u64>> = Vec::with_capacity(num_samples);
    // Define test levels including baseline (None) and mapped levels
    let test_levels: Vec<(Option<GrainLevel>, Option<&str>)> = std::iter::once((None, None)) // Baseline (no filter)
        .chain(HQDN3D_PARAMS.iter().map(|(level, params)| (Some(*level), Some(*params))))
        .collect();

    for (i, &start_time) in sample_start_times.iter().enumerate() {
        log::info!(
            "{} {}/{} (Start: {:.2}s, Duration: {}s)...",
            "Analyzing sample".cyan(),
            (i + 1).to_string().cyan().bold(),
            num_samples.to_string().cyan().bold(),
            start_time, sample_duration
        );

        // 1. Extract Sample (once per sample point)
        let raw_sample_path = match extract_sample(
            spawner, // Pass the spawner
            file_path,
            start_time,
            sample_duration,
            temp_dir_path,
        ) {
            Ok(path) => path,
            Err(e) => {
                log::error!("Failed to extract sample {}: {}", i + 1, e);
                // Use FilmGrainAnalysisFailed as the error variant
                return Err(CoreError::FilmGrainAnalysisFailed(format!(
                    "Failed to extract sample {}: {}", i + 1, e
                )));
            }
        };

        let mut results_for_this_sample = HashMap::new();

        // 2. Encode Sample for each test level and get size via metadata_provider
        for (level_opt, hqdn3d_override) in &test_levels {
            let level_desc = level_opt.map_or("None".to_string(), |l| format!("{:?}", l));
            log::debug!("  Encoding sample {} with hqdn3d level: {}...", i + 1, level_desc);

            // Construct output path for this specific sample encode
            let output_filename = format!(
                "sample_{}_level_{}.mkv",
                i + 1,
                level_desc.replace([':', '='], "_") // Sanitize filename
            );
            let encoded_sample_path = temp_dir_path.join(&output_filename); // Use reference to filename

            // Create encode parameters specific to this sample test
            let mut sample_params = base_encode_params.clone();
            sample_params.input_path = raw_sample_path.clone(); // Use the extracted raw sample
            sample_params.output_path = encoded_sample_path.clone();
            sample_params.hqdn3d_params = hqdn3d_override.map(|s| s.to_string());
            // Ensure duration is set for the sample, not the full video
            sample_params.duration = sample_duration_f64;
            // Note: Audio is disabled via the flag passed to run_ffmpeg_encode below
 
            // Run the ffmpeg encode process using the provided spawner, disabling audio and marking as grain sample
            if let Err(e) = crate::external::ffmpeg::run_ffmpeg_encode(
                spawner,
                &sample_params,
                true, /* disable_audio */
                true, /* is_grain_analysis_sample */
                *level_opt, // Pass the grain level being tested
            ) {
                log::error!("Failed to encode sample {} with hqdn3d level {}: {}", i + 1, level_desc, e);
                // Removed: let _ = fs::remove_file(&raw_sample_path); // Cleanup handled by TempDir
                // Use FilmGrainAnalysisFailed as the error variant
                return Err(CoreError::FilmGrainAnalysisFailed(format!(
                    "Failed to encode sample {} with hqdn3d level {}: {}", i + 1, level_desc, e
                 )));
            }

            // Get the size using the metadata provider AFTER encode succeeds
            let encoded_size = match metadata_provider.get_size(&encoded_sample_path) {
                 Ok(size) => size,
                 Err(e) => {
                    log::error!("Failed to get size for encoded sample {} level {} (path: {}): {}", i + 1, level_desc, encoded_sample_path.display(), e);
                    // Removed: let _ = fs::remove_file(&raw_sample_path); // Cleanup handled by TempDir
                    // If we can't get the size, the analysis for this sample fails
                    return Err(CoreError::FilmGrainAnalysisFailed(format!(
                        "Failed to get size for encoded sample {} level {} (path: {}): {}", i + 1, level_desc, encoded_sample_path.display(), e
                     )));
                 }
            };

            // Log the resulting size in MB at INFO level
            let size_mb = encoded_size as f64 / (1024.0 * 1024.0);
            log::info!("    -> Encoded size: {}", format!("{:.2} MB", size_mb).green());
            results_for_this_sample.insert(*level_opt, encoded_size);
        }

        sample_results.push(results_for_this_sample);

   } // End loop through sample points - raw_sample_path cleanup handled by TempDir drop

   // --- Analyze Results Across Samples ---
    if sample_results.is_empty() {
        log::error!("No samples were successfully processed for analysis.");
        return Ok(None);
    }

    let mut determined_levels_per_sample: Vec<GrainLevel> = Vec::with_capacity(num_samples);
    for (i, results_for_sample) in sample_results.iter().enumerate() {
        let determined_level = analyze_sample_results(results_for_sample);
        // Log the determined level for this sample at INFO level
        log::info!("  Sample {} determined optimal level: {}", (i + 1).to_string().cyan().bold(), format!("{:?}", determined_level).green().bold());
        determined_levels_per_sample.push(determined_level);
    }

    // --- Aggregate Results (Median) ---
    let final_level = calculate_median_level(&mut determined_levels_per_sample);

    log::info!(
        "{} {}: {}",
        "Final detected grain level for".cyan().bold(),
        filename_cow.yellow(), // Use the corrected variable name 'filename_cow'
        format!("{:?}", final_level).green().bold()
    );

    // Temporary directory `temp_dir` is automatically cleaned up when it goes out of scope here

    Ok(Some(GrainAnalysisResult {
        detected_level: final_level,
    }))
}

/// Analyzes the encode sizes for a single sample point to determine the best GrainLevel.
/// Uses a simple diminishing returns heuristic.
fn analyze_sample_results(results: &HashMap<Option<GrainLevel>, u64>) -> GrainLevel {
    // Helper closure to format bytes as MB string
    let bytes_to_mb_str = |bytes: u64| -> String {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    };
    let bytes_to_mb_str_signed = |bytes: i64| -> String {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    };


    let baseline_size = match results.get(&None) {
        Some(&size) => size,
        None => {
            log::error!("Baseline encode (hqdn3d=None) size missing from results. Cannot analyze.");
            return GrainLevel::VeryClean;
        }
    };

    // Calculate reductions and gains for each level compared to baseline
    let mut reductions: HashMap<GrainLevel, u64> = HashMap::new();
    let mut gains: HashMap<GrainLevel, i64> = HashMap::new();

    let mut prev_reduction: u64 = 0;

    // Iterate through levels in order: VeryLight, Light, Visible, Heavy
    for (level, _) in HQDN3D_PARAMS.iter() { // HQDN3D_PARAMS is ordered by strength
        if let Some(&current_size) = results.get(&Some(*level)) {
            if current_size >= baseline_size {
                 log::warn!("{} Denoised size ({:?}: {}) >= baseline size ({}). Treating reduction as 0.", "Warning:".yellow().bold(), level, current_size, baseline_size);
                 reductions.insert(*level, 0);
                 gains.insert(*level, 0i64.saturating_sub(prev_reduction as i64));
            } else {
                let reduction = baseline_size - current_size;
                reductions.insert(*level, reduction);
                gains.insert(*level, (reduction as i64).saturating_sub(prev_reduction as i64));
                prev_reduction = reduction;
            }
        } else {
            log::warn!("{} Size missing for level {:?}. Cannot calculate reduction/gain.", "Warning:".yellow().bold(), level);
            gains.insert(*level, 0);
        }
    }

    log::trace!("Sample reductions: {:?}", reductions);
    log::trace!("Sample gains: {:?}", gains);

    // --- Apply Diminishing Returns Logic ---

    // Check if even VeryLight provides minimal absolute benefit
    let very_light_reduction = reductions.get(&GrainLevel::VeryLight).copied().unwrap_or(0);
    if very_light_reduction < MIN_ABSOLUTE_REDUCTION_BYTES {
        // Log decision at INFO level, using MB
        log::info!(
            "    {}: VeryLight reduction ({}) is less than the minimum required ({}), indicating minimal benefit. Choosing {}.",
            "Reason".cyan(),
            bytes_to_mb_str(very_light_reduction).yellow(),
            bytes_to_mb_str(MIN_ABSOLUTE_REDUCTION_BYTES).yellow(),
            "VeryClean".green().bold()
        );
        return GrainLevel::VeryClean;
    }

    // Check diminishing returns for Light vs VeryLight
    let very_light_gain = gains.get(&GrainLevel::VeryLight).copied().unwrap_or(0); // Should be > 0 based on check above
    let light_gain = gains.get(&GrainLevel::Light).copied().unwrap_or(0);
    let threshold_gain = very_light_gain as f64 * DIMINISHING_RETURN_FACTOR;
    if very_light_gain > 0 && (light_gain <= 0 || (light_gain as f64) < threshold_gain) {
         // Log decision at INFO level, using MB and explaining the factor
         log::info!(
            "    {}: Gain from Light ({}) is less than {:.0}% of gain from VeryLight ({}). Diminishing returns. Choosing {}.",
            "Reason".cyan(),
            bytes_to_mb_str_signed(light_gain).yellow(),
            DIMINISHING_RETURN_FACTOR * 100.0,
            bytes_to_mb_str_signed(very_light_gain).yellow(),
            "VeryLight".green().bold()
         );
        return GrainLevel::VeryLight;
    }

    // Check diminishing returns for Visible vs Light
    let visible_gain = gains.get(&GrainLevel::Visible).copied().unwrap_or(0);
    let threshold_gain = light_gain as f64 * DIMINISHING_RETURN_FACTOR;
    if light_gain > 0 && (visible_gain <= 0 || (visible_gain as f64) < threshold_gain) {
         // Log decision at INFO level, using MB and explaining the factor
         log::info!(
            "    {}: Gain from Visible ({}) is less than {:.0}% of gain from Light ({}). Diminishing returns. Choosing {}.",
            "Reason".cyan(),
            bytes_to_mb_str_signed(visible_gain).yellow(),
            DIMINISHING_RETURN_FACTOR * 100.0,
            bytes_to_mb_str_signed(light_gain).yellow(),
            "Light".green().bold()
         );
        return GrainLevel::Light;
    }

    // Check diminishing returns for Heavy vs Visible
    let heavy_gain = gains.get(&GrainLevel::Heavy).copied().unwrap_or(0);
    let threshold_gain = visible_gain as f64 * DIMINISHING_RETURN_FACTOR;
    if visible_gain > 0 && (heavy_gain <= 0 || (heavy_gain as f64) < threshold_gain) {
         // Log decision at INFO level, using MB and explaining the factor
         log::info!(
            "    {}: Gain from Heavy ({}) is less than {:.0}% of gain from Visible ({}). Diminishing returns. Choosing {}.",
            "Reason".cyan(),
            bytes_to_mb_str_signed(heavy_gain).yellow(),
            DIMINISHING_RETURN_FACTOR * 100.0,
            bytes_to_mb_str_signed(visible_gain).yellow(),
            "Visible".green().bold()
         );
        return GrainLevel::Visible;
    }

    // If we reach here, Heavy provided sufficient gain over Visible
    // Log decision at INFO level, using MB
    log::info!(
        "    {}: Gain from Heavy ({}) is sufficient compared to Visible ({}). Choosing {}.",
        "Reason".cyan(),
        bytes_to_mb_str_signed(heavy_gain).yellow(),
        bytes_to_mb_str_signed(visible_gain).yellow(),
        "Heavy".green().bold()
    );
    GrainLevel::Heavy
}


/// Calculates the median GrainLevel from a list of levels.
fn calculate_median_level(levels: &mut [GrainLevel]) -> GrainLevel {
    if levels.is_empty() {
        return GrainLevel::VeryClean;
    }
    levels.sort_unstable();
    // Use (len - 1) / 2 to get the lower median index for even lengths
    let mid = (levels.len() - 1) / 2;
    levels[mid]
}

