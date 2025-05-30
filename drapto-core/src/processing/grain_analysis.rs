//! Film grain analysis for optimal denoising parameter selection.
//!
//! This module handles the detection and analysis of film grain/noise in video
//! files to determine optimal denoising parameters. It extracts multiple samples
//! from the video, tests various denoising levels, and selects the highest 
//! compression level that maintains quality within a perceptual threshold.

use crate::config::CoreConfig;
use crate::error::{CoreError, CoreResult};
use crate::external::ffmpeg::EncodeParams;
use crate::external::{calculate_xpsnr, extract_sample, get_file_size};
use crate::hardware_decode::is_hardware_decoding_available;
use crate::temp_files;
use rand::{Rng, thread_rng};
use std::path::Path;

pub use crate::processing::grain_types::{
    GrainAnalysisResult, GrainLevel, GrainLevelParseError, GrainLevelTestResult,
};



/// Determines the appropriate hqdn3d denoising parameters based on grain level
#[must_use]
pub fn determine_hqdn3d_params(level: GrainLevel) -> Option<String> {
    level.hqdn3d_params().map(|s| s.to_string())
}




/// Tests a single grain level and returns the result
fn test_grain_level(
    raw_sample_path: &Path,
    base_encode_params: &EncodeParams,
    temp_dir_path: &Path,
    sample_index: usize,
    sample_duration_f64: f64,
    level: Option<GrainLevel>,
) -> CoreResult<GrainLevelTestResult> {
    let level_desc = match level {
        None => "Baseline",
        Some(GrainLevel::Baseline) => "Baseline",
        Some(GrainLevel::VeryLight) => "VeryLight",
        Some(GrainLevel::Light) => "Light",
        Some(GrainLevel::LightModerate) => "Light-Mod",
        Some(GrainLevel::Moderate) => "Moderate",
        Some(GrainLevel::Elevated) => "Elevated",
    };

    log::debug!(
        "    Testing sample {} with level: {}...",
        sample_index,
        level_desc
    );

    let output_filename = format!(
        "sample_{}_level_{}.mkv",
        sample_index,
        level_desc.replace(['-'], "_")
    );
    let encoded_sample_path = temp_dir_path.join(&output_filename);

    let mut sample_params = base_encode_params.clone();
    sample_params.input_path = raw_sample_path.to_path_buf();
    sample_params.output_path = encoded_sample_path.clone();
    sample_params.hqdn3d_params = level.and_then(|l| l.hqdn3d_params()).map(|s| s.to_string());
    sample_params.duration = sample_duration_f64;

    crate::external::ffmpeg::run_ffmpeg_encode(&sample_params, true, true, level)
        .map_err(|e| {
            CoreError::FilmGrainAnalysisFailed(format!(
                "Failed to encode sample {} with level {}: {}",
                sample_index, level_desc, e
            ))
        })?;

    let encoded_size = get_file_size(&encoded_sample_path).map_err(|e| {
        CoreError::FilmGrainAnalysisFailed(format!(
            "Failed to get size for sample {} level {}: {}",
            sample_index, level_desc, e
        ))
    })?;

    let size_mb = encoded_size as f64 / (1024.0 * 1024.0);

    // Calculate XPSNR against raw sample
    let xpsnr = match calculate_xpsnr(
        raw_sample_path,
        &encoded_sample_path,
        sample_params.crop_filter.as_deref(),
    ) {
        Ok(xpsnr_value) => {
            log::debug!("    XPSNR for {}: {:.2} dB", level_desc, xpsnr_value);
            Some(xpsnr_value)
        }
        Err(e) => {
            log::warn!("    Failed to calculate XPSNR for {}: {}", level_desc, e);
            None
        }
    };

    if let Some(xpsnr_value) = xpsnr {
        crate::progress_reporting::info(&format!(
            "  {:12} {:.1} MB, XPSNR: {:.1} dB",
            format!("{}:", level_desc),
            size_mb,
            xpsnr_value
        ));
    } else {
        crate::progress_reporting::info(&format!(
            "  {:12} {:.1} MB",
            format!("{}:", level_desc),
            size_mb
        ));
    }

    Ok(GrainLevelTestResult {
        file_size: encoded_size,
        xpsnr,
    })
}

/// Performs binary search to find the optimal grain level for a sample
fn find_optimal_grain_level_binary_search(
    raw_sample_path: &Path,
    base_encode_params: &EncodeParams,
    temp_dir_path: &Path,
    sample_index: usize,
    sample_duration_f64: f64,
    xpsnr_threshold: f64,
    available_levels: &[GrainLevel],
) -> CoreResult<GrainLevel> {
    // First, test baseline
    let baseline_result = test_grain_level(
        raw_sample_path,
        base_encode_params,
        temp_dir_path,
        sample_index,
        sample_duration_f64,
        None,
    )?;

    let baseline_xpsnr = baseline_result.xpsnr.ok_or_else(|| {
        CoreError::FilmGrainAnalysisFailed(format!(
            "Sample {}: Baseline XPSNR is missing",
            sample_index
        ))
    })?;

    let baseline_size_mb = baseline_result.file_size as f64 / (1024.0 * 1024.0);
    crate::progress_reporting::info(&format!(
        "Sample {}: Baseline reference - {:.1} MB, XPSNR: {:.1} dB",
        sample_index, baseline_size_mb, baseline_xpsnr
    ));

    // If no levels available, return VeryLight as default
    if available_levels.is_empty() {
        return Ok(GrainLevel::VeryLight);
    }

    // Binary search through available levels
    let mut low = 0;
    let mut high = available_levels.len();
    let mut best_level = GrainLevel::VeryLight;
    let mut best_xpsnr_drop = 0.0;
    let mut best_size_reduction = 0.0;

    while low < high {
        let mid = (low + high) / 2;
        let test_level = available_levels[mid];

        let result = test_grain_level(
            raw_sample_path,
            base_encode_params,
            temp_dir_path,
            sample_index,
            sample_duration_f64,
            Some(test_level),
        )?;

        if let Some(xpsnr) = result.xpsnr {
            let xpsnr_drop = baseline_xpsnr - xpsnr;

            if xpsnr_drop <= xpsnr_threshold {
                // This level is acceptable, try a higher level
                let size_reduction =
                    ((baseline_result.file_size - result.file_size) as f64 / baseline_result.file_size as f64) * 100.0;

                best_level = test_level;
                best_xpsnr_drop = xpsnr_drop;
                best_size_reduction = size_reduction;

                log::debug!(
                    "Sample {}: {:?} is within threshold - XPSNR drop: {:.1} dB, trying higher",
                    sample_index,
                    test_level,
                    xpsnr_drop
                );

                // Try to go higher (more aggressive)
                low = mid + 1;
            } else {
                // This level exceeds threshold, try a lower level
                log::debug!(
                    "Sample {}: {:?} exceeds threshold - XPSNR drop: {:.1} dB, trying lower",
                    sample_index,
                    test_level,
                    xpsnr_drop
                );

                // Go lower (less aggressive)
                high = mid;
            }
        } else {
            // No XPSNR available, be conservative and go lower
            high = mid;
        }
    }

    // Report the selected level
    if best_xpsnr_drop < -0.1 {
        // XPSNR improved with denoising - indicates noisy/grainy content
        log::info!(
            "Sample {}: XPSNR improved with denoising (baseline: {:.1} dB, denoised: {:.1} dB) - indicates noisy/grainy content (possibly dark scenes)",
            sample_index, baseline_xpsnr, baseline_xpsnr - best_xpsnr_drop
        );
        crate::progress_reporting::info(&format!(
            "Sample {}: Selected {:?} ({:.1}% size reduction, XPSNR: +{:.1} dB improvement from baseline)",
            sample_index, best_level, best_size_reduction, -best_xpsnr_drop
        ));
    } else if best_xpsnr_drop > 0.1 {
        crate::progress_reporting::info(&format!(
            "Sample {}: Selected {:?} ({:.1}% size reduction, XPSNR: -{:.1} dB from baseline)",
            sample_index, best_level, best_size_reduction, best_xpsnr_drop
        ));
    } else {
        crate::progress_reporting::info(&format!(
            "Sample {}: Selected {:?} ({:.1}% size reduction, XPSNR: same as baseline)",
            sample_index, best_level, best_size_reduction
        ));
    }

    Ok(best_level)
}

/// Analyzes the grain/noise in a video file to determine optimal denoising parameters.
///
/// This function implements a streamlined approach to grain analysis:
/// 1. Extract multiple short samples from different parts of the video
/// 2. Encode each sample with various denoising levels
/// 3. Select the highest compression level within XPSNR quality threshold
/// 4. Use the most conservative result across all samples
///
/// The analysis selects the highest denoising level that keeps quality degradation
/// below a perceptual threshold (configured XPSNR drop from baseline).
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
///     film_grain_max_level: GrainLevel::Moderate,
///     xpsnr_threshold: 1.2,
///     grain_min_samples: 3,
///     grain_max_samples: 7,
///     grain_secs_per_sample: 1200.0,
///     grain_sample_start_boundary: 0.15,
///     grain_sample_end_boundary: 0.85,
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
    let max_level = config.film_grain_max_level;

    let base_samples = (duration_secs / config.grain_secs_per_sample).ceil() as usize;
    let num_samples = base_samples.clamp(config.grain_min_samples, config.grain_max_samples);
    log::debug!("Calculated number of samples: {num_samples}");

    crate::progress_reporting::info(&format!("Extracting {num_samples} samples for analysis..."));

    let min_required_duration = f64::from(sample_duration * num_samples as u32);
    if duration_secs < min_required_duration {
        log::warn!(
            "Warning: Video duration ({duration_secs:.2}s) is too short for the minimum required duration ({min_required_duration:.2}s) for {num_samples} samples. Skipping grain analysis."
        );
        return Ok(None);
    }


    let sample_duration_f64 = f64::from(sample_duration);
    let start_boundary = duration_secs * config.grain_sample_start_boundary;
    let end_boundary = duration_secs * config.grain_sample_end_boundary;
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
    log::debug!("Generated random sample start times: {sample_start_times:?}");

    let temp_dir = temp_files::create_grain_analysis_dir(config)?;
    let temp_dir_path = temp_dir.path();
    log::debug!(
        "Created temporary directory for samples: {}",
        temp_dir_path.display()
    );

    log::debug!("Testing grain levels...");
    crate::progress_reporting::info("Testing grain levels...");

    let mut final_estimates: Vec<GrainLevel> = Vec::with_capacity(num_samples);
    let mut max_allowed_level = max_level; // Will be progressively reduced

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

        // Determine available levels based on progressive elimination
        let available_levels: Vec<GrainLevel> = vec![
            GrainLevel::VeryLight,
            GrainLevel::Light,
            GrainLevel::LightModerate,
            GrainLevel::Moderate,
            GrainLevel::Elevated,
        ]
        .into_iter()
        .filter(|&level| level <= max_allowed_level)
        .collect();

        // Use binary search to find optimal level
        let sample_estimate = find_optimal_grain_level_binary_search(
            &raw_sample_path,
            base_encode_params,
            temp_dir_path,
            i + 1,
            sample_duration_f64,
            config.xpsnr_threshold,
            &available_levels,
        )?;
        
        // Update max allowed level for progressive elimination
        if sample_estimate < max_allowed_level {
            max_allowed_level = sample_estimate;
            log::debug!(
                "Progressive elimination: Maximum level for remaining samples reduced to {:?}",
                max_allowed_level
            );
        }
        
        final_estimates.push(sample_estimate);
        
        // If VeryLight is selected, no need to continue - it's the minimum level
        if sample_estimate == GrainLevel::VeryLight {
            log::info!(
                "Early exit: VeryLight selected at sample {}. No lower levels available.",
                i + 1
            );
            crate::progress_reporting::info(&format!(
                "Early exit: Minimum grain level (VeryLight) selected"
            ));
            break;
        }
    }

    if final_estimates.is_empty() {
        log::error!("No final estimates generated after all phases. Analysis failed.");
        return Err(CoreError::FilmGrainAnalysisFailed(
            "No final estimates generated after all phases".to_string(),
        ));
    }

    // Use the most conservative (minimum) level from all samples
    let mut final_level = *final_estimates.iter().min().unwrap_or(&GrainLevel::VeryLight);

    // Show why this level was selected
    let sample_results: Vec<String> = final_estimates
        .iter()
        .map(|level| format!("{:?}", level))
        .collect();
    

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
    
    // Show the selection reasoning to the user
    if final_estimates.len() > 1 {
        crate::progress_reporting::info(&format!(
            "{} selected - most conservative of: {}",
            level_description,
            sample_results.join(", ")
        ));
    }
    
    crate::progress_reporting::status(
        "Detected grain",
        &format!("{level_description} - applying appropriate denoising"),
        true,
    );

    Ok(Some(GrainAnalysisResult {
        detected_level: final_level,
    }))
}
