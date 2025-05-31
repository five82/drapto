//! FFmpeg command building and execution for video encoding
//!
//! This module handles building complex ffmpeg command lines with
//! appropriate arguments for video (libsvtav1) and audio (libopus) encoding,
//! progress reporting, and error handling.

use crate::error::{CoreError, CoreResult, command_failed_error};
use crate::processing::audio;
use crate::progress_reporting;

use ffmpeg_sidecar::command::FfmpegCommand;
use log::{debug, info, warn};

use std::path::PathBuf;
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
///
/// # Returns
///
/// * `CoreResult<FfmpegCommand>` - The configured `FFmpeg` command ready for execution
pub fn build_ffmpeg_command(
    params: &EncodeParams,
    hqdn3d_override: Option<&str>,
    disable_audio: bool,
    has_denoising: bool,
) -> CoreResult<FfmpegCommand> {
    // Use the new builder for common setup
    let mut cmd = crate::external::FfmpegCommandBuilder::new()
        .with_hardware_accel(params.use_hw_decode)
        .build();
    cmd.input(params.input_path.to_string_lossy().as_ref());

    if !disable_audio {
        cmd.args(["-af", "aformat=channel_layouts=7.1|5.1|stereo|mono"]);
    }
    let hqdn3d_to_use = hqdn3d_override.or(params.hqdn3d_params.as_deref());
    let filter_chain = crate::external::VideoFilterChain::new()
        .add_denoise(hqdn3d_to_use.unwrap_or(""))
        .add_crop(params.crop_filter.as_deref().unwrap_or(""))
        .build();

    if let Some(ref filters) = filter_chain {
        cmd.args(["-vf", filters]);
        crate::progress_reporting::info(&format!("Applying video filters: {filters}"));
    } else {
        crate::progress_reporting::info("No video filters applied.");
    }

    let film_grain_value = if has_denoising {
        crate::config::FIXED_FILM_GRAIN_VALUE
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

    if film_grain_value > 0 {
        crate::progress_reporting::info(&format!(
            "Applying film grain synthesis: level={film_grain_value}"
        ));
    } else {
        crate::progress_reporting::info(
            "No film grain synthesis applied (denoise level is None or 0).",
        );
    }

    if !disable_audio {
        cmd.args(["-c:a", "libopus"]);
        for (i, &channels) in params.audio_channels.iter().enumerate() {
            let bitrate = audio::calculate_audio_bitrate(channels);
            cmd.args([&format!("-b:a:{i}"), &format!("{bitrate}k")]);
        }
        cmd.args(["-map", "0:v:0"]);
        cmd.args(["-map", "0:a"]);
        cmd.args(["-map_metadata", "0"]);
        cmd.args(["-map_chapters", "0"]);
    } else {
        cmd.args(["-map", "0:v:0"]);
        cmd.arg("-an");
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
/// * `has_denoising` - Whether denoising is applied
///
/// # Returns
///
/// * `CoreResult<()>` - Success or error with detailed information
pub fn run_ffmpeg_encode(
    params: &EncodeParams,
    disable_audio: bool,
    has_denoising: bool,
) -> CoreResult<()> {
    progress_reporting::encode_start(&params.input_path, &params.output_path);
    info!(
        target: "drapto::progress",
        "Starting encode: {} -> {}",
        params.input_path.display(),
        params.output_path.display()
    );

    debug!("Encode parameters: {params:?}");

    let mut cmd = build_ffmpeg_command(params, None, disable_audio, has_denoising)?;
    let cmd_string = format!("{cmd:?}");
    crate::progress_reporting::ffmpeg_command(&cmd_string);
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
        crate::progress_reporting::status(
            "Progress duration",
            &crate::utils::format_duration_seconds(duration),
            false,
        );
    } else {
        warn!("Video duration not provided or zero; progress percentage will not be accurate.");
    }

    let mut progress_handler =
        crate::progress_reporting::ffmpeg_handler::FfmpegProgressHandler::new(
            duration_secs,
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
        crate::progress_reporting::clear_progress();

        crate::progress_reporting::info(&format!(
            "Encode finished successfully for {filename_cow}"
        ));
        Ok(())
    } else {
        let error_message = format!(
            "FFmpeg process exited with non-zero status ({:?}). Stderr output:\n{}",
            status.code(),
            progress_handler.stderr_buffer().trim()
        );

        crate::progress_reporting::encode_error(&params.input_path, &error_message);

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






