// drapto-core/src/processing/detection/grain_analysis/mod.rs
use colored::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use tempfile::Builder as TempFileBuilder;
use rand::{thread_rng, Rng};

use crate::config::CoreConfig;
use crate::error::{CoreError, CoreResult};
use crate::external::ffmpeg::{EncodeParams};
use crate::external::ffmpeg_executor::extract_sample;
use crate::external::{FileMetadataProvider, FfmpegSpawner};

// Declare submodules
mod constants;
mod knee_point;
mod refinement;
mod types;
mod utils;

// Publicly export types and the main utility function
pub use types::{GrainAnalysisResult, GrainLevel};
pub use utils::determine_hqdn3d_params;

// Import necessary items from submodules for use in analyze_grain
use constants::*;
use knee_point::analyze_sample_with_knee_point;
use refinement::{calculate_refinement_range, generate_refinement_params};
use utils::calculate_median_level;


/// Analyzes the grain in a video file by encoding samples with different denoise levels
/// and comparing the relative file size reductions using knee point analysis and adaptive refinement.
///
/// Returns `Ok(Some(GrainAnalysisResult))` if analysis is successful,
/// `Ok(None)` if analysis is skipped (e.g., short video),
/// or `Err(CoreError)` if a critical error occurs.
pub fn analyze_grain<S: FfmpegSpawner, P: FileMetadataProvider>(
    file_path: &Path,
    config: &CoreConfig,
    base_encode_params: &EncodeParams,
    duration_secs: f64,
    spawner: &S,
    metadata_provider: &P,
) -> CoreResult<Option<GrainAnalysisResult>> {
    let filename_cow = file_path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| file_path.to_string_lossy());

    log::info!(
        "{} {}",
        "Starting grain analysis (knee point + refinement) for:".cyan().bold(),
        filename_cow.yellow()
    );
    log::info!(
        "  {:<12} {}",
        "Duration:".cyan(),
        format!("{:.2}s", duration_secs).green()
    );

    // --- Determine Sample Count ---
    let base_samples = (duration_secs / SECS_PER_SAMPLE_TARGET).ceil() as usize;
    let mut num_samples = base_samples.clamp(MIN_SAMPLES, MAX_SAMPLES);
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
        .prefix("analysis_grain_") // Updated prefix
        .tempdir_in(&samples_tmp_base_dir)
        .map_err(CoreError::Io)?;
    let temp_dir_path = temp_dir.path();
    log::debug!("Created temporary directory for samples: {}", temp_dir_path.display());

    // --- Phase 1: Test Initial Values ---
    log::info!("{}", "Phase 1: Testing initial grain levels...".cyan().bold());
    let mut phase1_results: Vec<HashMap<Option<GrainLevel>, u64>> = Vec::with_capacity(num_samples);
    let mut raw_sample_paths: Vec<PathBuf> = Vec::with_capacity(num_samples); // Store raw sample paths
    let initial_test_levels: Vec<(Option<GrainLevel>, Option<&str>)> = std::iter::once((None, None))
        .chain(HQDN3D_PARAMS.iter().map(|(level, params)| (Some(*level), Some(*params))))
        .collect();

    for (i, &start_time) in sample_start_times.iter().enumerate() {
        log::info!(
            "  {} {}/{} (Start: {:.2}s, Duration: {}s)...",
            "Processing sample".cyan(),
            (i + 1).to_string().cyan().bold(),
            num_samples.to_string().cyan().bold(),
            start_time, sample_duration
        );

        let raw_sample_path = match extract_sample(
            spawner,
            file_path,
            start_time,
            sample_duration,
            temp_dir_path,
        ) {
            Ok(path) => path,
            Err(e) => {
                log::error!("Failed to extract sample {}: {}", i + 1, e);
                return Err(CoreError::FilmGrainAnalysisFailed(format!(
                    "Failed to extract sample {}: {}", i + 1, e
                )));
            }
        };
        raw_sample_paths.push(raw_sample_path.clone()); // Store the path

        let mut results_for_this_sample = HashMap::new();

        for (level_opt, hqdn3d_override) in &initial_test_levels {
            let level_desc = level_opt.map_or("None".to_string(), |l| format!("{:?}", l));
            log::debug!("    Encoding sample {} with initial level: {}...", i + 1, level_desc);

            let output_filename = format!(
                "sample_{}_initial_{}.mkv",
                i + 1,
                level_desc.replace([':', '='], "_")
            );
            let encoded_sample_path = temp_dir_path.join(&output_filename);

            let mut sample_params = base_encode_params.clone();
            sample_params.input_path = raw_sample_path.clone();
            sample_params.output_path = encoded_sample_path.clone();
            sample_params.hqdn3d_params = hqdn3d_override.map(|s| s.to_string());
            sample_params.duration = sample_duration_f64;

            if let Err(e) = crate::external::ffmpeg::run_ffmpeg_encode(
                spawner,
                &sample_params,
                true, true, *level_opt,
            ) {
                log::error!("Failed to encode sample {} with initial level {}: {}", i + 1, level_desc, e);
                return Err(CoreError::FilmGrainAnalysisFailed(format!(
                    "Failed to encode sample {} with initial level {}: {}", i + 1, level_desc, e
                 )));
            }

            let encoded_size = match metadata_provider.get_size(&encoded_sample_path) {
                 Ok(size) => size,
                 Err(e) => {
                    log::error!("Failed to get size for initial sample {} level {} (path: {}): {}", i + 1, level_desc, encoded_sample_path.display(), e);
                    return Err(CoreError::FilmGrainAnalysisFailed(format!(
                        "Failed to get size for initial sample {} level {} (path: {}): {}", i + 1, level_desc, encoded_sample_path.display(), e
                     )));
                 }
            };

            let size_mb = encoded_size as f64 / (1024.0 * 1024.0);
            log::info!("      -> {:<10} size: {}", level_desc.green(), format!("{:.2} MB", size_mb).yellow());
            results_for_this_sample.insert(*level_opt, encoded_size);
        }
        phase1_results.push(results_for_this_sample);
    }

    // --- Phase 2: Initial Estimation with Knee Point ---
    log::info!("{}", "Phase 2: Estimating optimal grain per sample using Knee Point...".cyan().bold());
    let mut initial_estimates: Vec<GrainLevel> = Vec::with_capacity(num_samples);

    for (i, sample_results) in phase1_results.iter().enumerate() {
        let estimate = analyze_sample_with_knee_point(
            sample_results,
            KNEE_THRESHOLD,
            &mut |msg| log::info!("  Sample {}: {}", (i + 1).to_string().cyan().bold(), msg)
        );
        initial_estimates.push(estimate);
    }
    log::info!("  Initial estimates per sample: {:?}", initial_estimates.iter().map(|l| format!("{:?}", l).green()).collect::<Vec<_>>());


    // --- Phase 3: Adaptive Refinement ---
    log::info!("{}", "Phase 3: Adaptive Refinement...".cyan().bold());
    let mut phase3_results: Vec<HashMap<Option<GrainLevel>, u64>> = vec![HashMap::new(); num_samples];

    if initial_estimates.len() < 3 {
        log::info!("  Too few samples ({}) for reliable refinement. Skipping Phase 3.", initial_estimates.len());
    } else {
        let (lower_bound, upper_bound) = calculate_refinement_range(&initial_estimates);
        log::info!(
            "  Calculated refinement range based on initial estimates: {} to {}",
            format!("{:?}", lower_bound).yellow(), format!("{:?}", upper_bound).yellow()
        );

        let refined_params = generate_refinement_params(
            lower_bound, upper_bound, &initial_test_levels
        );

        if refined_params.is_empty() {
            log::info!("  No refinement parameters generated within the range. Skipping Phase 3 testing.");
        } else {
            log::info!(
                "  Testing {} refined parameter sets: {:?}",
                refined_params.len(),
                refined_params.iter().map(|(l, p)| format!("{:?}: '{}'", l.unwrap_or(GrainLevel::VeryClean), p)).collect::<Vec<_>>()
            );
            log::debug!("  (Note: Parameter validation, like ensuring non-negative values, occurs during generation/parsing)");

            for (level_opt, params_str) in &refined_params {
                let level_desc = level_opt.map_or("Unknown".to_string(), |l| format!("{:?}", l));
                log::info!("    Testing refined level {}...", level_desc.green());

                // Iterate through the *indices* and use stored raw paths
                for i in 0..num_samples {
                    let raw_sample_path = &raw_sample_paths[i]; // Get stored raw path
                    let mut sample_params = base_encode_params.clone();
                    sample_params.input_path = raw_sample_path.clone(); // Use raw sample as input
                    let output_filename = format!(
                        "sample_{}_refined_{}.mkv",
                        i + 1,
                        level_desc.replace([':', '='], "_")
                    );
                    sample_params.output_path = temp_dir_path.join(output_filename);
                    sample_params.hqdn3d_params = Some(params_str.clone());
                    // sample_params.start_time = Some(start_time); // REMOVED - Not needed/valid
                    sample_params.duration = sample_duration_f64; // Duration is still relevant for encode

                    if let Err(e) = crate::external::ffmpeg::run_ffmpeg_encode(
                        spawner,
                        &sample_params,
                        true, true, *level_opt,
                    ) {
                        log::error!("Failed to encode refined sample {} with level {}: {}",
                                   i + 1, level_desc, e);
                        continue;
                    }

                    match metadata_provider.get_size(&sample_params.output_path) {
                        Ok(size) => {
                            let size_mb = size as f64 / (1024.0 * 1024.0);
                            log::info!("      -> Refined level {:<10} (Sample {}) size: {}", level_desc.green(), i + 1, format!("{:.2} MB", size_mb).yellow());
                            phase3_results[i].insert(*level_opt, size);
                        },
                        Err(e) => {
                            log::error!("Failed to get size for refined sample {} level {} (path: {}): {}", i + 1, level_desc, sample_params.output_path.display(), e);
                        }
                    }
                }
            }
        }
    }

    // --- Phase 4: Final Analysis with Combined Results ---
    log::info!("{}", "Phase 4: Final analysis with knee point on combined results...".cyan().bold());
    let mut final_estimates: Vec<GrainLevel> = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let mut combined_results = phase1_results[i].clone();
        combined_results.extend(phase3_results[i].iter());

        let final_estimate = analyze_sample_with_knee_point(
            &combined_results,
            KNEE_THRESHOLD,
            &mut |msg| log::info!("  Sample {}: {}", (i + 1).to_string().cyan().bold(), msg)
        );
        final_estimates.push(final_estimate);
    }

    log::info!("  Final estimates per sample (after refinement): {:?}", final_estimates.iter().map(|l| format!("{:?}", l).green()).collect::<Vec<_>>());

    // --- Determine Final Result ---
    if final_estimates.is_empty() {
        log::error!("No final estimates generated after all phases. Analysis failed.");
        return Ok(None);
    }

    let final_level = calculate_median_level(&mut final_estimates);

    log::info!(
        "{} {}: {}",
        "Final detected grain level for".cyan().bold(),
        filename_cow.yellow(),
        format!("{:?}", final_level).green().bold()
    );

    // temp_dir cleanup happens automatically on drop

    Ok(Some(GrainAnalysisResult {
        detected_level: final_level,
    }))
}