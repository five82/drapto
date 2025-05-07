// ============================================================================
// drapto-core/src/processing/detection/grain_analysis/mod.rs
// ============================================================================
//
// GRAIN ANALYSIS: Film Grain Detection and Denoising Parameter Optimization
//
// This module handles the detection and analysis of film grain/noise in video
// files to determine optimal denoising parameters. It uses a multi-phase approach
// with sample extraction, encoding tests, and knee point analysis to find the
// best balance between file size reduction and visual quality preservation.
//
// KEY COMPONENTS:
// - Configurable grain analysis parameters (sample duration, knee threshold, etc.)
// - Sample extraction and encoding with different denoising levels
// - Knee point analysis using "VeryClean" as the baseline
// - Adaptive refinement with direct parameter testing
// - Result aggregation with maximum level constraints
//
// WORKFLOW:
// 1. Extract multiple short samples from different parts of the video
// 2. Encode each sample with various denoising levels (always including "VeryClean")
// 3. Analyze file size reductions to find the knee point
// 4. Perform adaptive refinement around the initial estimates
// 5. Determine the final optimal denoising level with constraints
//
// AI-ASSISTANT-INFO: Film grain analysis for optimal denoising parameter selection

// ---- External crate imports ----
use colored::*;
use rand::{thread_rng, Rng};
use tempfile::Builder as TempFileBuilder;

// ---- Standard library imports ----
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ---- Internal crate imports ----
use crate::config::CoreConfig;
use crate::error::{CoreError, CoreResult};
use crate::external::ffmpeg::EncodeParams;
use crate::external::ffmpeg_executor::extract_sample;
use crate::external::{FileMetadataProvider, FfmpegSpawner};

// ============================================================================
// SUBMODULES
// ============================================================================

/// Constants used throughout the grain analysis process
mod constants;

/// Knee point detection algorithm for finding optimal denoising level
mod knee_point;

/// Adaptive refinement for improving accuracy of grain analysis
mod refinement;

/// Type definitions for grain analysis results
mod types;

/// Utility functions for grain analysis
mod utils;

// ============================================================================
// PUBLIC EXPORTS
// ============================================================================

/// Result types for grain analysis
pub use types::{GrainAnalysisResult, GrainLevel};

/// Functions to determine and generate hqdn3d parameters
pub use utils::{
    determine_hqdn3d_params,
    generate_hqdn3d_params,
    grain_level_to_strength,
    strength_to_grain_level,
};

// ============================================================================
// INTERNAL IMPORTS
// ============================================================================

/// Constants for grain analysis
use constants::*;

/// Knee point analysis function
use knee_point::analyze_sample_with_knee_point;

/// Refinement functions
use refinement::{calculate_refinement_range, generate_refinement_params};

/// Utility function for calculating median grain level
use utils::calculate_median_level;


// ============================================================================
// MAIN ANALYSIS FUNCTION
// ============================================================================

/// Analyzes the grain/noise in a video file to determine optimal denoising parameters.
///
/// This function implements a multi-phase approach to grain analysis:
/// 1. Extract multiple short samples from different parts of the video
/// 2. Encode each sample with various denoising levels (Phase 1)
/// 3. Analyze file size reductions to find the knee point (Phase 2)
/// 4. Perform adaptive refinement around the initial estimates (Phase 3)
/// 5. Determine the final optimal denoising level (Phase 4)
///
/// The analysis is based on the principle that there's a point of diminishing returns
/// in denoising, where additional strength provides minimal file size reduction but
/// may degrade visual quality. This function finds that optimal balance point.
///
/// # Arguments
///
/// * `file_path` - Path to the video file to analyze
/// * `config` - Core configuration
/// * `base_encode_params` - Base encoding parameters
/// * `duration_secs` - Duration of the video in seconds
/// * `spawner` - Implementation of FfmpegSpawner for executing ffmpeg
/// * `metadata_provider` - Implementation of FileMetadataProvider for file operations
///
/// # Returns
///
/// * `Ok(Some(GrainAnalysisResult))` - If analysis is successful
/// * `Ok(None)` - If analysis is skipped (e.g., video too short)
/// * `Err(CoreError)` - If a critical error occurs during analysis
///
/// # Example
///
/// ```rust,no_run
/// use drapto_core::processing::detection::grain_analysis::analyze_grain;
/// use drapto_core::processing::detection::GrainLevel;
/// use drapto_core::external::{SidecarSpawner, StdFsMetadataProvider};
/// use drapto_core::external::ffmpeg::EncodeParams;
/// use drapto_core::CoreConfig;
/// use std::path::Path;
///
/// let file_path = Path::new("/path/to/video.mkv");
/// let config = CoreConfig {
///     // Basic configuration
///     input_dir: Path::new("/path/to/input").to_path_buf(),
///     output_dir: Path::new("/path/to/output").to_path_buf(),
///     log_dir: Path::new("/path/to/logs").to_path_buf(),
///     enable_denoise: true,
///
///     // Grain analysis configuration
///     film_grain_sample_duration: Some(5),
///     film_grain_knee_threshold: Some(0.8),
///     film_grain_fallback_level: Some(GrainLevel::VeryClean),
///     film_grain_max_level: Some(GrainLevel::Visible),
///     film_grain_refinement_points_count: Some(5),
///
///     // Other fields...
///     default_encoder_preset: Some(6),
///     preset: None,
///     quality_sd: Some(24),
///     quality_hd: Some(26),
///     quality_uhd: Some(28),
///     default_crop_mode: Some("auto".to_string()),
///     ntfy_topic: None,
/// };
///
/// // Create a complete EncodeParams instance
/// let base_encode_params = EncodeParams {
///     input_path: Path::new("/path/to/video.mkv").to_path_buf(),
///     output_path: Path::new("/path/to/output.mkv").to_path_buf(),
///     quality: 24,
///     preset: 6,
///     crop_filter: None,
///     audio_channels: vec![2],
///     duration: 3600.0,
///     hqdn3d_params: None,
/// };
///
/// let duration_secs = 3600.0; // 1 hour
/// let spawner = SidecarSpawner;
/// let metadata_provider = StdFsMetadataProvider;
///
/// match analyze_grain(
///     file_path,
///     &config,
///     &base_encode_params,
///     duration_secs,
///     &spawner,
///     &metadata_provider,
/// ) {
///     Ok(Some(result)) => {
///         println!("Detected grain level: {:?}", result.detected_level);
///     },
///     Ok(None) => {
///         println!("Grain analysis skipped");
///     },
///     Err(e) => {
///         eprintln!("Error during grain analysis: {}", e);
///     }
/// }
/// ```
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

    // --- Get Configuration Parameters ---
    // Use configuration values if provided, otherwise use defaults
    let sample_duration = config.film_grain_sample_duration.unwrap_or(DEFAULT_SAMPLE_DURATION_SECS);
    let knee_threshold = config.film_grain_knee_threshold.unwrap_or(KNEE_THRESHOLD);
    let fallback_level = config.film_grain_fallback_level.unwrap_or(GrainLevel::VeryClean);
    let max_level = config.film_grain_max_level.unwrap_or(GrainLevel::Medium);

    // --- Determine Sample Count ---
    let base_samples = (duration_secs / SECS_PER_SAMPLE_TARGET).ceil() as usize;
    let mut num_samples = base_samples.clamp(MIN_SAMPLES, MAX_SAMPLES);
    if num_samples % 2 == 0 {
        num_samples = (num_samples + 1).min(MAX_SAMPLES);
    }
    log::debug!("Calculated number of samples: {}", num_samples);

    // --- Validate Duration for Sampling ---
    let min_required_duration = (sample_duration * num_samples as u32) as f64;
    if duration_secs < min_required_duration {
        log::warn!(
            "{} Video duration ({:.2}s) is too short for the minimum required duration ({:.2}s) for {} samples. Skipping grain analysis.",
            "Warning:".yellow().bold(), duration_secs, min_required_duration, num_samples
        );
        return Ok(None);
    }

    // --- Ensure we have a baseline test level (None) ---
    // Always include VeryClean (no denoising) as the baseline for comparison
    // This is critical for grain analysis to work correctly
    log::debug!("Ensuring baseline 'VeryClean' (no denoising) is included for grain analysis");

    // Create the initial test levels with VeryClean (no denoising) as the first test
    let initial_test_levels: Vec<(Option<GrainLevel>, Option<&str>)> = std::iter::once((None, None))
        .chain(HQDN3D_PARAMS.iter().map(|(level, params)| (Some(*level), Some(*params))))
        .collect();

    // Log the test levels for debugging
    log::debug!("Initial test levels: {:?}", initial_test_levels.iter()
        .map(|(level, _)| level.map_or("VeryClean".to_string(), |l| format!("{:?}", l)))
        .collect::<Vec<_>>());

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
            Err(CoreError::NoStreamsFound(_)) => {
                log::warn!("Sample {} extraction found no streams. Skipping grain analysis.", i + 1);
                return Ok(Some(GrainAnalysisResult {
                    detected_level: fallback_level,
                }));
            },
            Err(e) => {
                log::error!("Failed to extract sample {}: {}", i + 1, e);

                // If this is the first sample, return an error
                if i == 0 {
                    return Err(CoreError::FilmGrainAnalysisFailed(format!(
                        "Failed to extract first sample: {}", e
                    )));
                }

                // If we already have at least one sample, we can continue with what we have
                if !raw_sample_paths.is_empty() {
                    log::warn!(
                        "{} Failed to extract sample {}, but continuing with {} existing samples.",
                        "Warning:".yellow().bold(),
                        i + 1,
                        raw_sample_paths.len()
                    );
                    break;
                }

                // Otherwise, return the fallback level
                log::warn!("No samples could be extracted. Using fallback grain level.");
                return Ok(Some(GrainAnalysisResult {
                    detected_level: fallback_level,
                }));
            }
        };
        raw_sample_paths.push(raw_sample_path.clone()); // Store the path

        let mut results_for_this_sample = HashMap::new();

        for (level_opt, hqdn3d_override) in &initial_test_levels {
            let level_desc = level_opt.map_or("VeryClean".to_string(), |l| format!("{:?}", l));
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
            knee_threshold,
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
            knee_threshold,
            &mut |msg| log::info!("  Sample {}: {}", (i + 1).to_string().cyan().bold(), msg)
        );
        final_estimates.push(final_estimate);
    }

    log::info!("  Final estimates per sample (after refinement): {:?}", final_estimates.iter().map(|l| format!("{:?}", l).green()).collect::<Vec<_>>());

    // --- Determine Final Result ---
    if final_estimates.is_empty() {
        log::error!("No final estimates generated after all phases. Analysis failed.");
        return Ok(Some(GrainAnalysisResult {
            detected_level: fallback_level,
        }));
    }

    let mut final_level = calculate_median_level(&mut final_estimates);

    // Apply max level constraint if configured
    if final_level > max_level {
        log::info!(
            "{} Detected level {:?} exceeds maximum allowed level {:?}. Using maximum level.",
            "Note:".yellow().bold(),
            final_level,
            max_level
        );
        final_level = max_level;
    }

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