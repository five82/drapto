//! FFmpeg command building and execution for video encoding
//!
//! This module handles building complex ffmpeg command lines with
//! appropriate arguments for video (libsvtav1) and audio (libopus) encoding,
//! progress reporting, and error handling.

use crate::error::{CoreError, CoreResult, command_failed_error};
use crate::processing::audio;
use crate::processing::grain_types::GrainLevel;
use crate::progress_reporting;

use ffmpeg_sidecar::command::FfmpegCommand;
use log::{debug, error, info, warn};

use std::path::{Path, PathBuf};
use std::time::Instant;

/// Parameters required for running an `FFmpeg` encode operation.
#[derive(Debug, Clone)]
pub struct EncodeParams {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub quality: u32,
    pub preset: u8,
    /// Whether to use hardware decoding (when available)
    pub use_hw_decode: bool,
    pub crop_filter: Option<String>,
    pub audio_channels: Vec<u32>,
    pub duration: f64,
    /// The final hqdn3d parameters determined by analysis (used if override is not provided).
    pub hqdn3d_params: Option<String>,
}

/// Builds and configures an `FFmpeg` command using ffmpeg-sidecar's builder pattern.
///
/// This function creates a complete `FFmpeg` command for video encoding with libsvtav1
/// and audio encoding with libopus. It leverages ffmpeg-sidecar's builder methods
/// for cleaner and more maintainable code.
///
/// # Arguments
///
/// * `params` - Encoding parameters, including quality, preset, and filters
/// * `hqdn3d_override` - Optional override for the noise reduction filter parameters
/// * `disable_audio` - Whether to disable audio encoding
/// * `is_grain_analysis_sample` - Whether this is for grain analysis (simplified arguments)
///
/// # Returns
///
/// * `CoreResult<FfmpegCommand>` - The configured `FFmpeg` command ready for execution
pub fn build_ffmpeg_command(
    params: &EncodeParams,
    hqdn3d_override: Option<&str>,
    disable_audio: bool,
    is_grain_analysis_sample: bool,
    grain_level: Option<crate::processing::grain_types::GrainLevel>,
) -> CoreResult<FfmpegCommand> {
    // Use the new builder for common setup
    let mut cmd = crate::external::FfmpegCommandBuilder::new()
        .with_hardware_accel(params.use_hw_decode)
        .build();
    cmd.input(params.input_path.to_string_lossy().as_ref());

    cmd.input(params.input_path.to_string_lossy().as_ref());

    if !is_grain_analysis_sample && !disable_audio {
        cmd.args(["-af", "aformat=channel_layouts=7.1|5.1|stereo|mono"]);
    }
    let hqdn3d_to_use = hqdn3d_override.or(params.hqdn3d_params.as_deref());
    let filter_chain = crate::external::VideoFilterChain::new()
        .add_denoise(hqdn3d_to_use.unwrap_or(""))
        .add_crop(params.crop_filter.as_deref().unwrap_or(""))
        .build();

    if let Some(ref filters) = filter_chain {
        cmd.args(["-vf", filters]);
        if is_grain_analysis_sample {
            debug!("Applying video filters (grain sample): {filters}");
        } else {
            crate::progress_reporting::info(&format!("Applying video filters: {filters}"));
        }
    } else if !is_grain_analysis_sample {
        crate::progress_reporting::info("No video filters applied.");
    }

    let film_grain_value = if let Some(denoise_params) = hqdn3d_to_use {
        map_hqdn3d_to_film_grain(denoise_params, grain_level)
    } else {
        0
    };

    // Video encoding configuration - always use software encoding (libsvtav1)
    cmd.args(["-c:v", "libsvtav1"]);
    cmd.args(["-pix_fmt", "yuv420p10le"]);
    cmd.args(["-crf", &params.quality.to_string()]);
    cmd.args(["-preset", &params.preset.to_string()]);

    let svtav1_params = crate::external::SvtAv1ParamsBuilder::new()
        .with_film_grain(film_grain_value)
        .build();
    cmd.args(["-svtav1-params", &svtav1_params]);

    if !is_grain_analysis_sample {
        if film_grain_value > 0 {
            crate::progress_reporting::info(&format!(
                "Applying film grain synthesis: level={film_grain_value}"
            ));
        } else {
            crate::progress_reporting::info(
                "No film grain synthesis applied (denoise level is None or 0).",
            );
        }
    } else if film_grain_value > 0 {
        debug!(
            "Applying film grain synthesis (grain sample): level={film_grain_value}"
        );
    }

    if !is_grain_analysis_sample && !disable_audio {
        cmd.args(["-c:a", "libopus"]);
        for (i, &channels) in params.audio_channels.iter().enumerate() {
            let bitrate = audio::calculate_audio_bitrate(channels);
            cmd.args([&format!("-b:a:{i}"), &format!("{bitrate}k")]);
        }
    }

    if is_grain_analysis_sample || disable_audio {
        cmd.args(["-map", "0:v:0"]);
        if disable_audio {
            cmd.arg("-an");
        }
    } else {
        cmd.args(["-map", "0:v:0"]);
        cmd.args(["-map", "0:a"]);
        cmd.args(["-map_metadata", "0"]);
        cmd.args(["-map_chapters", "0"]);
    }

    cmd.args(["-movflags", "+faststart"]);

    cmd.output(params.output_path.to_string_lossy().as_ref());

    Ok(cmd)
}

/// Executes an `FFmpeg` encode operation.
///
/// This function handles the complete `FFmpeg` encoding process lifecycle, including:
/// - Constructing and executing the `FFmpeg` command
/// - Monitoring and reporting progress during encoding
/// - Processing and filtering `FFmpeg` output and error messages
/// - Determining encoding success or failure
///
/// # Arguments
///
/// * `params` - Encoding parameters for this operation
/// * `disable_audio` - Whether to disable audio in the output
/// * `is_grain_analysis_sample` - Whether this is a grain analysis sample encode
/// * `_grain_level_being_tested` - Optional grain level for analysis runs
///
/// # Returns
///
/// * `CoreResult<()>` - Success or error with detailed information
pub fn run_ffmpeg_encode(
    params: &EncodeParams,
    disable_audio: bool,
    is_grain_analysis_sample: bool,
    _grain_level_being_tested: Option<GrainLevel>,
) -> CoreResult<()> {
    let filename_cow = params
        .input_path
        .file_name().map_or_else(|| params.input_path.to_string_lossy(), |name| name.to_string_lossy());
    if is_grain_analysis_sample {
        debug!("Starting grain sample FFmpeg encode for: {filename_cow}");
    } else {
        progress_reporting::encode_start(&params.input_path, &params.output_path);
        info!(
            target: "drapto::progress",
            "Starting encode: {} -> {}",
            params.input_path.display(),
            params.output_path.display()
        );
    }

    debug!("Encode parameters: {params:?}");

    let mut cmd = build_ffmpeg_command(params, None, disable_audio, is_grain_analysis_sample, _grain_level_being_tested)?;
    if is_grain_analysis_sample {
        debug!("FFmpeg command (grain sample): {cmd:?}");
    } else {
        let cmd_string = format!("{cmd:?}");
        crate::progress_reporting::ffmpeg_command(&cmd_string, false);
    }
    if is_grain_analysis_sample {
        debug!("Starting grain sample encode...");
    }
    let _start_time = Instant::now();
    let mut child = cmd.spawn().map_err(|e| {
        command_failed_error(
            "ffmpeg",
            std::process::ExitStatus::default(),
            format!("Failed to start: {e}"),
        )
    })?;

    let duration_secs: Option<f64> = if params.duration > 0.0 {
        Some(params.duration)
    } else {
        None
    };

    if let Some(duration) = duration_secs {
        if is_grain_analysis_sample {
            debug!(
                "Using provided duration for progress (grain sample): {duration:.2}s"
            );
        } else {
            crate::progress_reporting::status(
                "Progress duration",
                &crate::utils::format_duration_seconds(duration),
                false,
            );
        }
    } else {
        warn!("Video duration not provided or zero; progress percentage will not be accurate.");
    }

    let mut progress_handler =
        crate::progress_reporting::ffmpeg_handler::FfmpegProgressHandler::new(
            duration_secs,
            is_grain_analysis_sample,
        );

    for event in child.iter().map_err(|e| {
        command_failed_error(
            "ffmpeg",
            std::process::ExitStatus::default(),
            format!("Failed to get event iterator: {e}"),
        )
    })? {
        progress_handler.handle_event(event)?;
    }

    // FFmpeg finished - check status
    let status = std::process::ExitStatus::default();
    let filename_cow = params
        .input_path
        .file_name().map_or_else(|| params.input_path.to_string_lossy(), |name| name.to_string_lossy());

    if status.success() {
        if !is_grain_analysis_sample {
            crate::progress_reporting::clear_progress();
        }

        let prefix = if is_grain_analysis_sample {
            "Grain sample encode"
        } else {
            "Encode"
        };
        if is_grain_analysis_sample {
            log::debug!("{prefix} finished successfully for {filename_cow}");
        } else {
            crate::progress_reporting::info(&format!(
                "{prefix} finished successfully for {filename_cow}"
            ));
        }
        Ok(())
    } else {
        let error_message = format!(
            "FFmpeg process exited with non-zero status ({:?}). Stderr output:\n{}",
            status.code(),
            progress_handler.stderr_buffer().trim()
        );

        let prefix = if is_grain_analysis_sample {
            "Grain sample encode"
        } else {
            "FFmpeg encode"
        };
        if is_grain_analysis_sample {
            debug!("{prefix} failed for {filename_cow}: {error_message}");
        } else {
            crate::progress_reporting::encode_error(&params.input_path, &error_message);
        }

        // Check for specific error types
        if progress_handler
            .stderr_buffer()
            .contains("No streams found")
        {
            Err(CoreError::NoStreamsFound(filename_cow.to_string()))
        } else {
            Err(command_failed_error(
                "ffmpeg (sidecar)",
                status,
                error_message,
            ))
        }
    }
}

/// Maps hqdn3d denoising parameters to SVT-AV1 film grain synthesis values.
///
/// This function looks up the corresponding film grain value for standard
/// grain levels. For custom parameters, it uses a square-root scale based
/// on the luma spatial strength.
///
/// # Arguments
///
/// * `hqdn3d_params` - The hqdn3d filter parameters as a string
/// * `grain_level` - Optional grain level for direct lookup
///
/// # Returns
///
/// * The corresponding SVT-AV1 film grain synthesis value (0-50)
fn map_hqdn3d_to_film_grain(hqdn3d_params: &str, grain_level: Option<crate::processing::grain_types::GrainLevel>) -> u8 {
    if hqdn3d_params.is_empty() {
        return 0;
    }

    // If we have a grain level, use its film grain value directly
    if let Some(level) = grain_level {
        if let Some(value) = level.film_grain_value() {
            return value;
        }
    }

    // For custom parameters, extract luma spatial strength
    let luma_spatial = parse_hqdn3d_first_param(hqdn3d_params);

    if luma_spatial <= 0.1 {
        return 0;
    }

    // Use square-root scale to reduce bias against higher grain values
    let adjusted_value = (luma_spatial * 8.0).sqrt() * 8.0;

    let film_grain_value = adjusted_value.round() as u8;
    film_grain_value.min(50) // Max value for SVT-AV1 film grain
}

/// Extracts the luma spatial strength parameter from an hqdn3d filter string.
///
/// # Arguments
///
/// * `params` - The complete hqdn3d filter string to parse
///
/// # Returns
///
/// * The extracted luma spatial strength as a float, or 0.0 if parsing fails
fn parse_hqdn3d_first_param(params: &str) -> f32 {
    // Handle params with or without "hqdn3d=" prefix
    let params_to_parse = if let Some(suffix) = params.strip_prefix("hqdn3d=") {
        suffix
    } else {
        params
    };
    
    if let Some(index) = params_to_parse.find(':') {
        let first_param = &params_to_parse[0..index];
        return first_param.parse::<f32>().unwrap_or(0.0);
    }
    
    // If no colon found, try parsing the entire string as a single value
    params_to_parse.parse::<f32>().unwrap_or(0.0)
}


/// Extracts a raw video sample using ffmpeg's -c copy.
///
/// Creates a temporary file within the specified output directory.
pub fn extract_sample(
    input_path: &Path,
    start_time_secs: f64,
    duration_secs: u32,
    output_dir: &Path,
) -> CoreResult<PathBuf> {
    debug!(
        "Extracting sample: input={}, start={}, duration={}, out_dir={}",
        input_path.display(),
        start_time_secs,
        duration_secs,
        output_dir.display()
    );

    let output_path = crate::temp_files::create_temp_file_path(output_dir, "raw_sample", "mkv");

    let mut cmd = crate::external::FfmpegCommandBuilder::new()
        .with_hardware_accel(true)
        .build();

    cmd.input(input_path.to_string_lossy().as_ref())
        .args(["-ss", &start_time_secs.to_string()])
        .args(["-t", &duration_secs.to_string()])
        .args(["-c", "copy"])
        .args(["-an"])
        .args(["-sn"])
        .args(["-map", "0:v"])
        .args(["-map_metadata", "0"])
        .output(output_path.to_string_lossy().as_ref());

    debug!("Running sample extraction command: {cmd:?}");

    let mut child = cmd.spawn().map_err(|e| {
        command_failed_error(
            "ffmpeg",
            std::process::ExitStatus::default(),
            format!("Failed to start sample extraction: {e}"),
        )
    })?;

    // Collect stderr for error reporting, filtering non-critical messages
    let mut stderr_buffer = String::new();
    for event in child.iter().map_err(|e| {
        command_failed_error(
            "ffmpeg",
            std::process::ExitStatus::default(),
            format!("Failed to get event iterator: {e}"),
        )
    })? {
        match event {
            ffmpeg_sidecar::event::FfmpegEvent::Log(_level, message) => {
                if !crate::external::is_non_critical_ffmpeg_message(&message) {
                    stderr_buffer.push_str(&format!("{message}\n"));
                }
            }
            ffmpeg_sidecar::event::FfmpegEvent::Error(error) => {
                if !crate::external::is_non_critical_ffmpeg_message(&error) {
                    stderr_buffer.push_str(&format!("{error}\n"));
                }
            }
            _ => {}
        }
    }

    let status = std::process::ExitStatus::default();

    if !status.success() {
        let error_msg = if stderr_buffer.is_empty() {
            "Sample extraction process failed".to_string()
        } else {
            format!("Sample extraction failed. Stderr:\n{}", stderr_buffer.trim())
        };
        error!("Sample extraction failed: {error_msg}");
        return Err(command_failed_error(
            "ffmpeg (sample extraction)",
            status,
            error_msg,
        ));
    }

    debug!(
        "Sample extracted successfully to: {}",
        output_path.display()
    );
    Ok(output_path)
}


/// Calculates XPSNR (eXtended Peak Signal-to-Noise Ratio) between reference and encoded videos.
///
/// XPSNR is a more advanced quality metric that considers perceptual quality better than PSNR.
///
/// # Arguments
///
/// * `reference_path` - Path to the reference (original) video
/// * `encoded_path` - Path to the encoded video to compare
/// * `crop_filter` - Optional crop filter to apply to the reference video to match encoded dimensions
///
/// # Returns
///
/// * `CoreResult<f64>` - Average XPSNR value in dB
pub fn calculate_xpsnr(
    reference_path: &Path,
    encoded_path: &Path,
    crop_filter: Option<&str>,
) -> CoreResult<f64> {
    debug!(
        "Calculating XPSNR: reference={}, encoded={}",
        reference_path.display(),
        encoded_path.display()
    );

    let mut cmd = crate::external::FfmpegCommandBuilder::new()
        .with_hide_banner(true)
        .with_hardware_accel(false)
        .build();

    // Force software decoding for both inputs
    cmd.args(["-hwaccel", "none"])
        .input(reference_path.to_string_lossy().as_ref())
        .args(["-hwaccel", "none"])
        .input(encoded_path.to_string_lossy().as_ref());

    let filter_complex = if let Some(crop) = crop_filter {
        if crop.is_empty() {
            "[0:v][1:v]xpsnr=stats_file=-".to_string()
        } else {
            format!("[0:v]{crop}[ref];[ref][1:v]xpsnr=stats_file=-")
        }
    } else {
        "[0:v][1:v]xpsnr=stats_file=-".to_string()
    };

    cmd.args(["-filter_complex", &filter_complex])
        .args(["-f", "null", "-"])
        .args(["-loglevel", "info"]);

    debug!("Running XPSNR calculation command: {cmd:?}");

    let mut child = cmd.spawn().map_err(|e| {
        command_failed_error(
            "ffmpeg",
            std::process::ExitStatus::default(),
            format!("Failed to start XPSNR calculation: {e}"),
        )
    })?;

    let mut stderr = String::new();
    let process_success = true;

    for event in child.iter().map_err(|e| {
        command_failed_error(
            "ffmpeg",
            std::process::ExitStatus::default(),
            format!("Failed to get event iterator: {e}"),
        )
    })? {
        use ffmpeg_sidecar::event::FfmpegEvent;
        match event {
            FfmpegEvent::Log(_, line) | FfmpegEvent::Error(line) => {
                stderr.push_str(&line);
                stderr.push('\n');
            }
            FfmpegEvent::Done => {
                break;
            }
            _ => {}
        }
    }

    if !process_success {
        error!("XPSNR calculation failed: {stderr}");
        return Err(command_failed_error(
            "ffmpeg (xpsnr)",
            std::process::ExitStatus::default(),
            format!("XPSNR calculation failed: {stderr}"),
        ));
    }

    // Parse XPSNR value from output - use Y (luma) component
    let mut luma_xpsnr = None;
    for line in stderr.lines() {
        if line.contains("XPSNR") && line.contains("y:") {
            if let Some(y_pos) = line.find("y:") {
                let after_y = &line[y_pos + 2..];
                let value_str = after_y.split_whitespace().next().unwrap_or("").trim();
                if let Ok(value) = value_str.parse::<f64>() {
                    luma_xpsnr = Some(value);
                    break;
                }
            }
        }
    }

    if let Some(value) = luma_xpsnr {
        debug!("XPSNR (luma) calculation successful: {value:.2} dB");
        Ok(value)
    } else {
        error!("Failed to parse XPSNR luma value from output:\n{stderr}");
        Err(CoreError::FilmGrainAnalysisFailed(
            "Failed to parse XPSNR luma value from ffmpeg output".to_string(),
        ))
    }
}

