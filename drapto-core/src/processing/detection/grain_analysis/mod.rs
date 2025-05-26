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
// - Knee point analysis using "Baseline" as the reference point
// - Adaptive refinement with direct parameter testing
// - Result aggregation with maximum level constraints
//
// WORKFLOW:
// 1. Extract multiple short samples from different parts of the video
// 2. Encode each sample with various denoising levels (always including "Baseline")
// 3. Analyze file size reductions to find the knee point
// 4. Perform adaptive refinement around the initial estimates
// 5. Determine the final optimal denoising level with constraints
//
// AI-ASSISTANT-INFO: Film grain analysis for optimal denoising parameter selection

// ---- External crate imports ----
use rand::{Rng, thread_rng};

// ---- Standard library imports ----
use std::collections::HashMap;
use std::path::Path;

// ---- Internal crate imports ----
use crate::config::CoreConfig;
use crate::error::{CoreError, CoreResult};
use crate::external::ffmpeg::EncodeParams;
use crate::external::{extract_sample, get_file_size};
use crate::hardware_accel::is_hardware_acceleration_available;
// Report progress functions are imported directly from the module
use crate::temp_files;

// ============================================================================
// SUBMODULES
// ============================================================================

/// Constants used throughout the grain analysis process
mod constants;

/// Knee point detection algorithm for finding optimal denoising level
mod knee_point;

// Refinement module is not currently used after simplification
// mod refinement;

/// Type definitions for grain analysis results
mod types;

/// Utility functions for grain analysis
mod utils;

// ============================================================================
// PUBLIC EXPORTS
// ============================================================================

/// Result types for grain analysis
pub use types::{GrainAnalysisResult, GrainLevel, GrainLevelParseError};

/// Functions to determine and generate hqdn3d parameters
pub use utils::{
    determine_hqdn3d_params, generate_hqdn3d_params, grain_level_to_strength,
    strength_to_grain_level,
};

// ============================================================================
// INTERNAL IMPORTS
// ============================================================================

/// Constants for grain analysis
use constants::*;

/// Knee point analysis function
use knee_point::analyze_sample_with_knee_point;


/// Utility function for calculating median grain level
use utils::calculate_median_level;

// ============================================================================
// MAIN ANALYSIS FUNCTION
// ============================================================================

/// Analyzes the grain/noise in a video file to determine optimal denoising parameters.
///
/// This function implements a streamlined approach to grain analysis:
/// 1. Extract multiple short samples from different parts of the video
/// 2. Encode each sample with various denoising levels
/// 3. Analyze file size reductions using knee point detection
/// 4. Determine the final optimal denoising level using median
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
///     temp_dir: None,
///     enable_denoise: true,
///
///     // Encoder settings
///     encoder_preset: 6,
///     quality_sd: 24,
///     quality_hd: 26,
///     quality_uhd: 28,
///     crop_mode: "auto".to_string(),
///
///     // Notification settings
///     ntfy_topic: None,
///
///     // Grain analysis configuration
///     film_grain_sample_duration: 5,
///     film_grain_knee_threshold: 0.8,
///     film_grain_max_level: GrainLevel::Moderate,
///     film_grain_refinement_points_count: 5,
/// };
///
/// // Create a complete EncodeParams instance
/// let base_encode_params = EncodeParams {
///     input_path: Path::new("/path/to/video.mkv").to_path_buf(),
///     output_path: Path::new("/path/to/output.mkv").to_path_buf(),
///     quality: 24,
///     preset: 6,
///     use_hw_decode: true,
///     crop_filter: None,
///     audio_channels: vec![2],
///     duration: 3600.0,
///     hqdn3d_params: None,
/// };
///
/// let duration_secs = 3600.0; // 1 hour
/// match analyze_grain(
///     file_path,
///     &config,
///     &base_encode_params,
///     duration_secs,
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
pub fn analyze_grain(
    file_path: &Path,
    config: &CoreConfig,
    base_encode_params: &EncodeParams,
    duration_secs: f64,
) -> CoreResult<Option<GrainAnalysisResult>> {
    // We're no longer using filename_cow, removed to avoid warnings

    // Main grain analysis start using standard processing step format
    // (spacing is automatically added by report_processing_step)
    crate::progress_reporting::report_processing_step("Analyzing grain levels");

    // Duration is already shown in the main video analysis section, no need to repeat

    // Inform user about hardware acceleration status for the main encode
    if base_encode_params.use_hw_decode {
        let hw_accel_available = is_hardware_acceleration_available();
        if hw_accel_available {
            // Report hardware acceleration status as a sub-item in verbose mode
            log::debug!("VideoToolbox hardware decoding will be used for main encode (disabled during analysis)");
            crate::progress_reporting::report_sub_item(
                "VideoToolbox hardware decoding will be used for main encode (disabled during analysis)",
            );
        } else {
            // Hardware acceleration info is verbose
            crate::progress_reporting::report_debug_info(
                "Software decoding will be used (hardware acceleration not available on this platform)",
            );
        }
    }

    // --- Get Configuration Parameters ---
    let sample_duration = config.film_grain_sample_duration;
    let knee_threshold = config.film_grain_knee_threshold;
    let max_level = config.film_grain_max_level;

    // --- Determine Sample Count ---
    let base_samples = (duration_secs / SECS_PER_SAMPLE_TARGET).ceil() as usize;
    let mut num_samples = base_samples.clamp(MIN_SAMPLES, MAX_SAMPLES);
    if num_samples % 2 == 0 {
        num_samples = (num_samples + 1).min(MAX_SAMPLES);
    }
    log::debug!("Calculated number of samples: {}", num_samples);

    // Use sub-item format for the extraction message - include number of samples
    crate::progress_reporting::report_sub_item(&format!(
        "Extracting {} samples for analysis...",
        num_samples
    ));

    // --- Validate Duration for Sampling ---
    let min_required_duration = (sample_duration * num_samples as u32) as f64;
    if duration_secs < min_required_duration {
        log::warn!(
            "Warning: Video duration ({:.2}s) is too short for the minimum required duration ({:.2}s) for {} samples. Skipping grain analysis.",
            duration_secs,
            min_required_duration,
            num_samples
        );
        return Ok(None);
    }

    // --- Ensure we have a baseline test level (None) ---
    // Always include Baseline (no denoising) as the reference for comparison
    // This is critical for grain analysis to work correctly
    log::debug!("Ensuring 'Baseline' (no denoising) is included for grain analysis");

    // Create the initial test levels with Baseline (no denoising) as the first test
    let initial_test_levels: Vec<(Option<GrainLevel>, Option<&str>)> =
        std::iter::once((None, None))
            .chain(
                HQDN3D_PARAMS
                    .iter()
                    .map(|(level, params)| (Some(*level), Some(*params))),
            )
            .collect();

    // Log the test levels for debugging
    log::debug!(
        "Initial test levels: {:?}",
        initial_test_levels
            .iter()
            .map(|(level, _)| level.map_or("Baseline".to_string(), |l| format!("{:?}", l)))
            .collect::<Vec<_>>()
    );

    // --- Calculate Randomized Sample Positions ---
    let sample_duration_f64 = sample_duration as f64;
    let start_boundary = duration_secs * 0.15;
    let end_boundary = duration_secs * 0.85;
    let latest_possible_start = end_boundary - sample_duration_f64;

    if latest_possible_start <= start_boundary {
        log::warn!(
            "Warning: Video duration ({:.2}s) results in an invalid sampling window ({:.2}s - {:.2}s) for sample duration {}. Skipping grain analysis.",
            duration_secs,
            start_boundary,
            end_boundary,
            sample_duration
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
    log::debug!(
        "Generated random sample start times: {:?}",
        sample_start_times
    );

    // --- Create Temporary Directory ---
    let temp_dir = temp_files::create_grain_analysis_dir(config)?;
    let temp_dir_path = temp_dir.path();
    log::debug!(
        "Created temporary directory for samples: {}",
        temp_dir_path.display()
    );

    // --- Test Grain Levels ---
    // Show detailed phase info as sub-items in verbose mode
    log::debug!("Testing grain levels...");
    crate::progress_reporting::report_sub_item("Testing grain levels...");

    let mut phase1_results: Vec<HashMap<Option<GrainLevel>, u64>> = Vec::with_capacity(num_samples);
    let mut early_estimates: Vec<GrainLevel> = Vec::with_capacity(num_samples);

    for (i, &start_time) in sample_start_times.iter().enumerate() {
        // Sample processing details as sub-items in verbose mode
        log::debug!("Sample {}/{} at {:.1}s:", i + 1, num_samples, start_time);
        crate::progress_reporting::report_sub_item(&format!(
            "Sample {}/{} at {:.1}s:",
            i + 1,
            num_samples,
            start_time
        ));

        let raw_sample_path = match extract_sample(
            file_path,
            start_time,
            sample_duration,
            temp_dir_path,
        ) {
            Ok(path) => path,
            Err(e) => {
                // Log the error and propagate it
                log::error!("Failed to extract sample {}: {}", i + 1, e);
                return Err(CoreError::FilmGrainEncodingFailed(format!(
                    "Failed to extract sample {}: {}",
                    i + 1,
                    e
                )));
            }
        };

        let mut results_for_this_sample = HashMap::new();

        for (level_opt, hqdn3d_override) in &initial_test_levels {
            let level_desc = match level_opt {
                None => "Baseline",
                Some(GrainLevel::Baseline) => "Baseline",
                Some(GrainLevel::VeryLight) => "VeryLight",
                Some(GrainLevel::Light) => "Light",
                Some(GrainLevel::LightModerate) => "Light-Mod",
                Some(GrainLevel::Moderate) => "Moderate",
                Some(GrainLevel::Elevated) => "Elevated",
            };
            log::debug!(
                "    Encoding sample {} with initial level: {}...",
                i + 1,
                level_desc
            );

            let output_filename = format!(
                "sample_{}_initial_{}.mkv",
                i + 1,
                level_desc.replace(['-'], "_")
            );
            let encoded_sample_path = temp_dir_path.join(&output_filename);

            let mut sample_params = base_encode_params.clone();
            sample_params.input_path = raw_sample_path.clone();
            sample_params.output_path = encoded_sample_path.clone();
            sample_params.hqdn3d_params = hqdn3d_override.map(|s| s.to_string());
            sample_params.duration = sample_duration_f64;

            if let Err(e) = crate::external::ffmpeg::run_ffmpeg_encode(
                &sample_params,
                true,
                true,
                *level_opt,
            ) {
                log::error!(
                    "Failed to encode sample {} with initial level {}: {}",
                    i + 1,
                    level_desc,
                    e
                );
                return Err(CoreError::FilmGrainAnalysisFailed(format!(
                    "Failed to encode sample {} with initial level {}: {}",
                    i + 1,
                    level_desc,
                    e
                )));
            }

            let encoded_size = match get_file_size(&encoded_sample_path) {
                Ok(size) => size,
                Err(e) => {
                    log::error!(
                        "Failed to get size for initial sample {} level {} (path: {}): {}",
                        i + 1,
                        level_desc,
                        encoded_sample_path.display(),
                        e
                    );
                    return Err(CoreError::FilmGrainAnalysisFailed(format!(
                        "Failed to get size for initial sample {} level {} (path: {}): {}",
                        i + 1,
                        level_desc,
                        encoded_sample_path.display(),
                        e
                    )));
                }
            };

            let size_mb = encoded_size as f64 / (1024.0 * 1024.0);
            
            log::debug!("  {:12} {:.1} MB", format!("{}:", level_desc), size_mb);
            crate::progress_reporting::report_sub_item(&format!(
                "  {:12} {:.1} MB",
                format!("{}:", level_desc),
                size_mb
            ));
            results_for_this_sample.insert(*level_opt, encoded_size);
        }
        phase1_results.push(results_for_this_sample.clone());
        
        // Analyze this sample immediately for early exit check
        let sample_estimate = analyze_sample_with_knee_point(&results_for_this_sample, knee_threshold, i + 1);
        early_estimates.push(sample_estimate);
        
        // Early exit optimization: Check if we have consistent results after at least 3 samples
        if early_estimates.len() >= 3 {
            // Check if all estimates are the same
            let first_estimate = early_estimates[0];
            if early_estimates.iter().all(|&e| e == first_estimate) {
                log::info!(
                    "Early exit: All {} samples consistently show {:?} grain level",
                    early_estimates.len(), first_estimate
                );
                crate::progress_reporting::report_sub_item(&format!(
                    "Early exit: Consistent results detected ({:?})",
                    first_estimate
                ));
                break; // Exit early with consistent results
            }
            
            // Check if estimates are within one level of each other
            let min_level = early_estimates.iter().min().unwrap();
            let max_level = early_estimates.iter().max().unwrap();
            let level_distance = (*max_level as u8).saturating_sub(*min_level as u8);
            
            if level_distance <= 1 && early_estimates.len() >= 4 {
                // Estimates are very close (adjacent levels) and we have enough samples
                log::info!(
                    "Early exit: {} samples show consistent range ({:?} to {:?})",
                    early_estimates.len(), min_level, max_level
                );
                crate::progress_reporting::report_sub_item(&format!(
                    "Early exit: Consistent range detected ({:?} to {:?})",
                    min_level, max_level
                ));
                break;
            }
        }
    }

    // --- Determine Final Result ---
    // Use early_estimates which already contains the analyzed results
    let mut final_estimates = early_estimates;
    if final_estimates.is_empty() {
        log::error!("No final estimates generated after all phases. Analysis failed.");
        return Err(CoreError::FilmGrainAnalysisFailed(
            "No final estimates generated after all phases".to_string(),
        ));
    }

    let mut final_level = calculate_median_level(&mut final_estimates);

    // Apply max level constraint if configured
    if final_level > max_level {
        let message = format!(
            "Detected level {:?} exceeds maximum allowed level {:?}. Using maximum level.",
            final_level, max_level
        );
        crate::progress_reporting::report_debug_info(&message);
        final_level = max_level;
    }

    // Final result is important - show in normal mode with meaningful description
    let level_description = match final_level {
        GrainLevel::Baseline => "None (no denoising needed)",
        GrainLevel::VeryLight => "VeryLight",
        GrainLevel::Light => "Light",
        GrainLevel::LightModerate => "LightModerate",
        GrainLevel::Moderate => "Moderate",
        GrainLevel::Elevated => "Elevated",
    };

    // Use the centralized function for success+status formatting
    crate::progress_reporting::report_completion_with_status(
        "Grain analysis complete",
        "Detected grain",
        &format!("{} - applying appropriate denoising", level_description),
    );

    // temp_dir cleanup happens automatically on drop

    Ok(Some(GrainAnalysisResult {
        detected_level: final_level,
    }))
}
