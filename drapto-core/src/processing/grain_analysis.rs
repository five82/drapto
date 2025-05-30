//! Film grain analysis for optimal denoising parameter selection.
//!
//! This module handles the detection and analysis of film grain/noise in video
//! files to determine optimal denoising parameters. It uses a multi-phase approach
//! with sample extraction, encoding tests, and knee point analysis to find the
//! best balance between file size reduction and visual quality preservation.

use rand::{Rng, thread_rng};
use std::collections::HashMap;
use std::path::Path;
use crate::config::CoreConfig;
use crate::error::{CoreError, CoreResult};
use crate::external::ffmpeg::EncodeParams;
use crate::external::{calculate_xpsnr, extract_sample, get_file_size};
use crate::hardware_decode::is_hardware_decoding_available;
use crate::temp_files;


pub use crate::processing::grain_types::{
    GrainAnalysisResult, GrainLevel, GrainLevelParseError, GrainLevelTestResult,
};


/// Minimum number of samples to extract regardless of video duration
const MIN_SAMPLES: usize = 3;

/// Maximum number of samples to extract (to avoid excessive processing)
const MAX_SAMPLES: usize = 7;

/// Target seconds of video per sample (used to calculate number of samples)
const SECS_PER_SAMPLE_TARGET: f64 = 1200.0; // 20 minutes

/// Get the hqdn3d parameters to test for grain analysis
fn get_test_levels() -> Vec<(GrainLevel, &'static str)> {
    vec![
        GrainLevel::VeryLight,
        GrainLevel::Light,
        GrainLevel::LightModerate,
        GrainLevel::Moderate,
        GrainLevel::Elevated,
    ]
    .into_iter()
    .filter_map(|level| level.hqdn3d_params().map(|params| (level, params)))
    .collect()
}


/// Determines the appropriate hqdn3d denoising parameters based on grain level
#[must_use] pub fn determine_hqdn3d_params(level: GrainLevel) -> Option<String> {
    level.hqdn3d_params().map(|s| s.to_string())
}

/// Converts a grain level to a numeric strength value (0.0 to 1.0)
#[must_use] pub fn grain_level_to_strength(level: GrainLevel) -> f64 {
    match level {
        GrainLevel::Baseline => 0.0,
        GrainLevel::VeryLight => 0.2,
        GrainLevel::Light => 0.4,
        GrainLevel::LightModerate => 0.6,
        GrainLevel::Moderate => 0.8,
        GrainLevel::Elevated => 1.0,
    }
}

/// Converts a numeric strength value (0.0 to 1.0) to the nearest grain level
#[must_use] pub fn strength_to_grain_level(strength: f64) -> GrainLevel {
    let clamped = strength.clamp(0.0, 1.0);

    if clamped <= 0.1 {
        GrainLevel::Baseline
    } else if clamped <= 0.3 {
        GrainLevel::VeryLight
    } else if clamped <= 0.5 {
        GrainLevel::Light
    } else if clamped <= 0.7 {
        GrainLevel::LightModerate
    } else if clamped <= 0.9 {
        GrainLevel::Moderate
    } else {
        GrainLevel::Elevated
    }
}

/// Calculates the median grain level from a list of estimates
fn calculate_median_level(estimates: &mut [GrainLevel]) -> GrainLevel {
    if estimates.is_empty() {
        return GrainLevel::VeryLight;
    }

    estimates.sort();
    let mid = estimates.len() / 2;

    if estimates.len() % 2 == 0 && estimates.len() > 1 {
        estimates[mid - 1]
    } else {
        estimates[mid]
    }
}


/// Determines the optimal denoising level for file size reduction with quality awareness.
///
/// This function analyzes encoding results to find the best balance between
/// file size reduction and quality preservation. The goal is practical file
/// size reduction for home/mobile viewing, not archival quality.
///
/// # Algorithm Overview
///
/// 1. Calculate efficiency for each grain level: (`size_reduction` * `quality_factor` / `sqrt(grain_level_value)`)
/// 2. Quality factor is derived from XPSNR measurements:
///    - XPSNR > 45 dB: factor = 1.0 (virtually no quality loss)
///    - XPSNR 40-45 dB: factor = 0.9-1.0 (minimal quality loss)
///    - XPSNR 35-40 dB: factor = 0.7-0.9 (moderate quality loss)
///    - XPSNR < 35 dB: factor < 0.7 (significant quality loss)
/// 3. Find diminishing returns point where improvement rate drops below threshold
/// 4. If no clear knee point exists, default to `VeryLight` (safe choice)
///
/// # Key Behaviors
///
/// - Always returns at least `VeryLight` (never Baseline/no denoising)
/// - Provides clear messaging about why a level was chosen
/// - Conservative approach: when uncertain, choose lighter denoising
/// - Penalizes levels that cause significant quality degradation
///
/// # Arguments
///
/// * `results` - `HashMap` mapping grain levels to their test results (size and XPSNR)
/// * `knee_threshold` - Threshold factor (0.0-1.0) for knee point selection (currently unused)
/// * `sample_index` - The index of the sample being analyzed (for logging)
///
/// # Returns
///
/// The optimal `GrainLevel` for file size reduction (minimum: `VeryLight`)
fn analyze_sample_with_knee_point(
    results: &HashMap<Option<GrainLevel>, GrainLevelTestResult>,
    knee_threshold: f64,
    sample_index: usize,
) -> GrainLevel {
    // Get the baseline (no denoising) for informational display
    let baseline_result = match results.get(&None) {
        Some(result) if result.file_size > 0 => Some(result),
        _ => {
            results
                .get(&Some(GrainLevel::Baseline))
                .filter(|r| r.file_size > 0)
        }
    };

    let verylight_result = match results.get(&Some(GrainLevel::VeryLight)) {
        Some(result) if result.file_size > 0 => {
            log::debug!(
                "Sample {sample_index}: Using VeryLight as reference for comparisons."
            );
            result
        }
        _ => {
            crate::progress_reporting::error(&format!(
                "  Sample {sample_index}: ERROR: VeryLight reference is missing. Cannot analyze."
            ));
            return GrainLevel::VeryLight;
        }
    };

    let verylight_size = verylight_result.file_size;
    let verylight_xpsnr = verylight_result.xpsnr;

    if let Some(baseline) = baseline_result {
        let baseline_size_mb = baseline.file_size as f64 / (1024.0 * 1024.0);
        if let Some(baseline_xpsnr_val) = baseline.xpsnr {
            crate::progress_reporting::info(&format!(
                "Sample {sample_index}: Baseline reference - {baseline_size_mb:.1} MB, XPSNR: {baseline_xpsnr_val:.1} dB"
            ));
        } else {
            crate::progress_reporting::info(&format!(
                "Sample {sample_index}: Baseline reference - {baseline_size_mb:.1} MB"
            ));
        }
    }

    // Map GrainLevel enum variants to numerical values for calculations
    let to_numeric = |level: Option<GrainLevel>| -> f64 {
        match level {
            None => 0.0,
            Some(GrainLevel::Baseline) => 0.0,
            Some(GrainLevel::VeryLight) => 1.0,
            Some(GrainLevel::Light) => 2.0,
            Some(GrainLevel::LightModerate) => 3.0,
            Some(GrainLevel::Moderate) => 4.0,
            Some(GrainLevel::Elevated) => 5.0,
        }
    };

    // Initialize vector to store efficiency metrics for each grain level
    let mut efficiencies: Vec<(Option<GrainLevel>, f64)> = Vec::new();

    let mut size_reductions: Vec<(Option<GrainLevel>, f64)> = Vec::new();
    let mut quality_metrics: Vec<(Option<GrainLevel>, f64)> = Vec::new();

    // Process each grain level and calculate its efficiency
    for (&level, result) in results.iter().filter(|&(_, r)| r.file_size > 0) {
        let size = result.file_size;
        let grain_numeric = to_numeric(level);

        // Skip Baseline and VeryLight levels
        if grain_numeric <= 1.0 {
            continue;
        }

        // Calculate size reduction compared to VeryLight
        let reduction = verylight_size.saturating_sub(size) as f64;

        if reduction <= 0.0 {
            continue;
        }

        // Calculate percentage reduction for reporting
        let reduction_pct = (reduction / verylight_size as f64) * 100.0;
        size_reductions.push((level, reduction_pct));

        let quality_factor = if let Some(xpsnr) = result.xpsnr {
            let xpsnr_delta = if let Some(verylight_xpsnr_val) = verylight_xpsnr {
                verylight_xpsnr_val - xpsnr
            } else {
                log::debug!("No VeryLight XPSNR available for delta calculation");
                1.0
            };

            // Quality factor based on XPSNR delta from VeryLight:
            // Goal: Maximize compression while staying below perceptible quality loss
            // Based on research consensus for XPSNR JND thresholds:
            // - < 0.25 dB: Virtually imperceptible (no penalty)
            // - 0.25-0.45 dB: Borderline JND, maybe noticeable on side-by-side
            // - 0.5-1.0 dB: Reliably noticeable, still acceptable for streaming
            // - 1.0-3.0 dB: Clearly visible to most viewers
            // - > 3.0 dB: Significant quality degradation

            let quality_factor = if xpsnr_delta < 0.45 {
                1.0 // Virtually imperceptible to borderline JND - no penalty
            } else if xpsnr_delta < 1.0 {
                1.0 - (xpsnr_delta - 0.45) * 0.091 // Linear from 1.0 to 0.95
            } else if xpsnr_delta < 3.0 {
                0.95 - (xpsnr_delta - 1.0) * 0.125 // Linear from 0.95 to 0.7
            } else {
                // More than 3 dB loss - apply heavy penalty
                (0.7 - (xpsnr_delta - 3.0) * 0.15).max(0.2)
            };

            quality_metrics.push((level, xpsnr));
            quality_factor
        } else {
            0.85
        };

        // Calculate efficiency using square-root scaling with quality factor
        let adjusted_denom = grain_numeric.sqrt();
        let efficiency = (reduction * quality_factor) / adjusted_denom;

        if efficiency > 0.0 && efficiency.is_finite() {
            efficiencies.push((level, efficiency));
        }
    }

    if efficiencies.is_empty() {
        crate::progress_reporting::info(&format!(
            "  Sample {sample_index}: No efficiency improvements found. Using VeryLight (safe default)."
        ));
        return GrainLevel::VeryLight;
    }

    // Sort efficiencies by grain level (lowest to highest)
    let mut sorted_efficiencies = efficiencies.clone();
    sorted_efficiencies.sort_by(|a, b| {
        let a_val = to_numeric(a.0);
        let b_val = to_numeric(b.0);
        a_val
            .partial_cmp(&b_val)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Find the point of diminishing returns
    let mut selected_level: Option<(Option<GrainLevel>, f64)> = None;
    if sorted_efficiencies.len() == 1 {
        selected_level = sorted_efficiencies.first().copied();
    } else {
        for i in 0..sorted_efficiencies.len() {
            if i == 0 {
                continue;
            }

            let (curr_level, curr_eff) = sorted_efficiencies[i];
            let (prev_level, prev_eff) = sorted_efficiencies[i - 1];

            let improvement = curr_eff - prev_eff;
            let improvement_rate = if prev_eff > 0.0 {
                improvement / prev_eff
            } else if improvement > 0.0 {
                1.0
            } else {
                0.0
            };

            log::debug!(
                "Sample {}: {:?} to {:?}: improvement={:.2}, rate={:.1}%",
                sample_index,
                prev_level.unwrap_or(GrainLevel::Baseline),
                curr_level.unwrap_or(GrainLevel::Baseline),
                improvement,
                improvement_rate * 100.0
            );

            if improvement_rate < knee_threshold {
                selected_level = Some(sorted_efficiencies[i - 1]);
                log::debug!(
                    "Sample {}: Diminishing returns detected at {:?}, selecting {:?}",
                    sample_index,
                    curr_level.unwrap_or(GrainLevel::Baseline),
                    prev_level.unwrap_or(GrainLevel::Baseline)
                );
                break;
            }
        }

        if selected_level.is_none() && !sorted_efficiencies.is_empty() {
            let continuously_improving = sorted_efficiencies.windows(2).all(|w| w[1].1 > w[0].1);

            if continuously_improving && sorted_efficiencies.len() >= 3 {
                log::debug!(
                    "Sample {sample_index}: Efficiency continuously improving. Selecting Light as balanced choice."
                );

                selected_level = sorted_efficiencies
                    .iter()
                    .find(|(level, _)| matches!(level, Some(GrainLevel::Light)))
                    .copied();

                if selected_level.is_none() {
                    let max_efficiency = sorted_efficiencies
                        .iter()
                        .map(|(_, eff)| *eff)
                        .fold(0.0, f64::max);

                    let threshold = 0.7 * max_efficiency;
                    selected_level = sorted_efficiencies
                        .iter()
                        .find(|(_, eff)| *eff >= threshold)
                        .copied();
                }
            } else {
                selected_level = sorted_efficiencies.first().copied();
            }
        }
    }

    // Process the selected level
    if let Some((Some(level), efficiency)) = selected_level {
        // Calculate the actual file size reduction percentage for this level
        let level_result = results.get(&Some(level)).unwrap();
        // Size reduction from VeryLight (for decision display)
        let size_reduction_from_verylight =
            ((verylight_size - level_result.file_size) as f64 / verylight_size as f64) * 100.0;

        // Get quality metric for reporting (show delta from VeryLight)
        let quality_info = if let Some(xpsnr) = level_result.xpsnr {
            if let Some(verylight_xpsnr_val) = verylight_xpsnr {
                let delta = verylight_xpsnr_val - xpsnr;
                if delta > 0.1 {
                    format!(" (XPSNR: {xpsnr:.1} dB, -{delta:.1} dB from VeryLight)")
                } else {
                    format!(" (XPSNR: {xpsnr:.1} dB, same as VeryLight)")
                }
            } else {
                format!(" (XPSNR: {xpsnr:.1} dB)")
            }
        } else {
            String::new()
        };

        // Check if this was selected due to continuous improvement
        let is_continuous_improvement =
            sorted_efficiencies.windows(2).all(|w| w[1].1 > w[0].1) && level == GrainLevel::Light;

        // Report the analysis results as sub-items in verbose mode
        if is_continuous_improvement {
            log::debug!(
                "Sample {sample_index}: Continuous improvement detected. Selected {level:?} as balanced choice. Efficiency: {efficiency:.2}, Size reduction: {size_reduction_from_verylight:.1}% beyond VeryLight"
            );
            crate::progress_reporting::info(&format!(
                "Sample {sample_index}: Selected {level:?} ({size_reduction_from_verylight:.1}% additional reduction{quality_info}) - continuous improvement"
            ));
        } else {
            log::debug!(
                "Sample {sample_index}: Knee point found at {level:?}. Efficiency: {efficiency:.2}, Size reduction: {size_reduction_from_verylight:.1}% beyond VeryLight"
            );
            crate::progress_reporting::info(&format!(
                "Sample {sample_index}: Selected {level:?} ({size_reduction_from_verylight:.1}% additional reduction{quality_info})"
            ));
        }

        // Return the selected optimal grain level
        level
    } else {
        // No suitable level found - return VeryLight
        log::debug!(
            "Sample {sample_index}: No improvements beyond VeryLight found. Using VeryLight as optimal choice."
        );
        crate::progress_reporting::info(&format!(
            "Sample {sample_index}: Using VeryLight (no better alternatives found)."
        ));

        // Use VeryLight as the result
        GrainLevel::VeryLight
    }
}


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
/// * `spawner` - Implementation of `FfmpegSpawner` for executing ffmpeg
/// * `metadata_provider` - Implementation of `FileMetadataProvider` for file operations
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
/// use drapto_core::processing::grain_analysis::analyze_grain;
/// use drapto_core::processing::grain_types::GrainLevel;
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
    crate::progress_reporting::processing("Analyzing grain levels");

    // Inform user about hardware decoding status for the main encode
    if base_encode_params.use_hw_decode {
        let hw_decode_available = is_hardware_decoding_available();
        if hw_decode_available {
            log::debug!(
                "VideoToolbox hardware decoding will be used for main encode (disabled during analysis)"
            );
            crate::progress_reporting::info(
                "VideoToolbox hardware decoding will be used for main encode (disabled during analysis)",
            );
        } else {
            crate::progress_reporting::debug(
                "Software decoding will be used (hardware decoding not available on this platform)",
            );
        }
    }

    let sample_duration = config.film_grain_sample_duration;
    let knee_threshold = config.film_grain_knee_threshold;
    let max_level = config.film_grain_max_level;

    let base_samples = (duration_secs / SECS_PER_SAMPLE_TARGET).ceil() as usize;
    let mut num_samples = base_samples.clamp(MIN_SAMPLES, MAX_SAMPLES);
    if num_samples % 2 == 0 {
        num_samples = (num_samples + 1).min(MAX_SAMPLES);
    }
    log::debug!("Calculated number of samples: {num_samples}");

    crate::progress_reporting::info(&format!(
        "Extracting {num_samples} samples for analysis..."
    ));

    let min_required_duration = f64::from(sample_duration * num_samples as u32);
    if duration_secs < min_required_duration {
        log::warn!(
            "Warning: Video duration ({duration_secs:.2}s) is too short for the minimum required duration ({min_required_duration:.2}s) for {num_samples} samples. Skipping grain analysis."
        );
        return Ok(None);
    }

    // Always include Baseline (no denoising) as the reference for comparison
    log::debug!("Ensuring 'Baseline' (no denoising) is included for grain analysis");
    let initial_test_levels: Vec<(Option<GrainLevel>, Option<&str>)> =
        std::iter::once((None, None))
            .chain(
                get_test_levels()
                    .iter()
                    .map(|(level, params)| (Some(*level), Some(*params))),
            )
            .collect();

    log::debug!(
        "Initial test levels: {:?}",
        initial_test_levels
            .iter()
            .map(|(level, _)| level.map_or("Baseline".to_string(), |l| format!("{l:?}")))
            .collect::<Vec<_>>()
    );

    let sample_duration_f64 = f64::from(sample_duration);
    let start_boundary = duration_secs * 0.15;
    let end_boundary = duration_secs * 0.85;
    let latest_possible_start = end_boundary - sample_duration_f64;

    if latest_possible_start <= start_boundary {
        log::warn!(
            "Warning: Video duration ({duration_secs:.2}s) results in an invalid sampling window ({start_boundary:.2}s - {end_boundary:.2}s) for sample duration {sample_duration}. Skipping grain analysis."
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
        "Generated random sample start times: {sample_start_times:?}"
    );

    let temp_dir = temp_files::create_grain_analysis_dir(config)?;
    let temp_dir_path = temp_dir.path();
    log::debug!(
        "Created temporary directory for samples: {}",
        temp_dir_path.display()
    );

    log::debug!("Testing grain levels...");
    crate::progress_reporting::info("Testing grain levels...");

    let mut phase1_results: Vec<HashMap<Option<GrainLevel>, GrainLevelTestResult>> =
        Vec::with_capacity(num_samples);
    let mut early_estimates: Vec<GrainLevel> = Vec::with_capacity(num_samples);

    for (i, &start_time) in sample_start_times.iter().enumerate() {
        log::debug!("Sample {}/{} at {:.1}s:", i + 1, num_samples, start_time);
        crate::progress_reporting::info(&format!(
            "Sample {}/{} at {:.1}s:",
            i + 1,
            num_samples,
            start_time
        ));

        let raw_sample_path =
            match extract_sample(file_path, start_time, sample_duration, temp_dir_path) {
                Ok(path) => path,
                Err(e) => {
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
            sample_params.hqdn3d_params = hqdn3d_override.map(std::string::ToString::to_string);
            sample_params.duration = sample_duration_f64;

            if let Err(e) =
                crate::external::ffmpeg::run_ffmpeg_encode(&sample_params, true, true, *level_opt)
            {
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

            // Calculate XPSNR against raw sample for all levels including baseline
            let xpsnr = match calculate_xpsnr(
                &raw_sample_path,
                &encoded_sample_path,
                sample_params.crop_filter.as_deref(),
            ) {
                Ok(xpsnr_value) => {
                    log::debug!("    XPSNR for {level_desc}: {xpsnr_value:.2} dB");
                    Some(xpsnr_value)
                }
                Err(e) => {
                    log::warn!("    Failed to calculate XPSNR for {level_desc}: {e}");
                    None
                }
            };

            if let Some(xpsnr_value) = xpsnr {
                log::debug!(
                    "  {:12} {:.1} MB, XPSNR: {:.1} dB",
                    format!("{}:", level_desc),
                    size_mb,
                    xpsnr_value
                );
                crate::progress_reporting::info(&format!(
                    "  {:12} {:.1} MB, XPSNR: {:.1} dB",
                    format!("{}:", level_desc),
                    size_mb,
                    xpsnr_value
                ));
            } else {
                log::debug!("  {:12} {:.1} MB", format!("{}:", level_desc), size_mb);
                crate::progress_reporting::info(&format!(
                    "  {:12} {:.1} MB",
                    format!("{}:", level_desc),
                    size_mb
                ));
            }

            results_for_this_sample.insert(
                *level_opt,
                GrainLevelTestResult {
                    file_size: encoded_size,
                    xpsnr,
                },
            );
        }
        phase1_results.push(results_for_this_sample.clone());

        let sample_estimate =
            analyze_sample_with_knee_point(&results_for_this_sample, knee_threshold, i + 1);
        early_estimates.push(sample_estimate);

        // Check if we have consistent results after at least 3 samples
        if early_estimates.len() >= 3 {
            let first_estimate = early_estimates[0];
            if early_estimates.iter().all(|&e| e == first_estimate) {
                log::info!(
                    "Early exit: All {} samples consistently show {:?} grain level",
                    early_estimates.len(),
                    first_estimate
                );
                crate::progress_reporting::info(&format!(
                    "Early exit: Consistent results detected ({first_estimate:?})"
                ));
                break;
            }

            let min_level = early_estimates.iter().min().unwrap();
            let max_level = early_estimates.iter().max().unwrap();
            let level_distance = (*max_level as u8).saturating_sub(*min_level as u8);

            if level_distance <= 1 && early_estimates.len() >= 4 {
                log::info!(
                    "Early exit: {} samples show consistent range ({:?} to {:?})",
                    early_estimates.len(),
                    min_level,
                    max_level
                );
                crate::progress_reporting::info(&format!(
                    "Early exit: Consistent range detected ({min_level:?} to {max_level:?})"
                ));
                break;
            }
        }
    }

    let mut final_estimates = early_estimates;
    if final_estimates.is_empty() {
        log::error!("No final estimates generated after all phases. Analysis failed.");
        return Err(CoreError::FilmGrainAnalysisFailed(
            "No final estimates generated after all phases".to_string(),
        ));
    }

    let mut final_level = calculate_median_level(&mut final_estimates);

    if final_level > max_level {
        let message = format!(
            "Detected level {final_level:?} exceeds maximum allowed level {max_level:?}. Using maximum level."
        );
        crate::progress_reporting::debug(&message);
        final_level = max_level;
    }

    let level_description = match final_level {
        GrainLevel::Baseline => "None (no denoising needed)",
        GrainLevel::VeryLight => "VeryLight",
        GrainLevel::Light => "Light",
        GrainLevel::LightModerate => "LightModerate",
        GrainLevel::Moderate => "Moderate",
        GrainLevel::Elevated => "Elevated",
    };

    crate::progress_reporting::success("Grain analysis complete");
    crate::progress_reporting::status(
        "Detected grain",
        &format!("{level_description} - applying appropriate denoising"),
        true,
    );

    Ok(Some(GrainAnalysisResult {
        detected_level: final_level,
    }))
}
