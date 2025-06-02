//! FFmpeg command building and execution for video encoding
//!
//! This module handles building complex ffmpeg command lines with
//! appropriate arguments for video (libsvtav1) and audio (libopus) encoding,
//! progress reporting, and error handling.

use crate::error::{CoreError, CoreResult, command_failed_error};
use crate::processing::audio;

use ffmpeg_sidecar::command::FfmpegCommand;
use log::debug;

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
    /// The fixed hqdn3d parameters for VeryLight denoising (used if override is not provided).
    pub hqdn3d_params: Option<String>,
}

/// Builds FFmpeg command for libsvtav1 video and libopus audio encoding.
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
        crate::progress_reporting::info_debug(&format!("Applying video filters: {}", filters));
    } else {
        crate::progress_reporting::info_debug("No video filters applied.");
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
        crate::progress_reporting::info_debug(&format!("Applying film grain synthesis: level={}", film_grain_value));
    } else {
        crate::progress_reporting::info_debug("No film grain synthesis applied (denoise level is None or 0).");
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

/// Executes FFmpeg encode with progress monitoring and error handling.
pub fn run_ffmpeg_encode(
    params: &EncodeParams,
    disable_audio: bool,
    has_denoising: bool,
) -> CoreResult<()> {
    debug!("Output: {}", params.output_path.display());
    log::info!(
        target: "drapto::progress",
        "Starting encode: {} -> {}",
        params.input_path.display(),
        params.output_path.display()
    );

    debug!("Encode parameters: {params:?}");

    let mut cmd = build_ffmpeg_command(params, None, disable_audio, has_denoising)?;
    let cmd_string = format!("{cmd:?}");
    debug!("FFmpeg command: {}", cmd_string);
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
        crate::progress_reporting::status("Duration", &crate::utils::format_duration(duration), false);
    } else {
        crate::progress_reporting::warning("Video duration not provided or zero; progress percentage will not be accurate.");
    }

    let mut stderr_buffer = String::new();
    let mut last_progress_percent = 0.0;

    for event in child.iter().map_err(|e| {
        command_failed_error(
            "ffmpeg",
            std::process::ExitStatus::default(),
            format!("Failed to get event iterator: {e}"),
        )
    })? {
        match event {
            ffmpeg_sidecar::event::FfmpegEvent::Log(_level, message) => {
                stderr_buffer.push_str(&message);
                stderr_buffer.push('\n');
            }
            ffmpeg_sidecar::event::FfmpegEvent::Error(error) => {
                stderr_buffer.push_str(&format!("ERROR: {}\n", error));
            }
            ffmpeg_sidecar::event::FfmpegEvent::Progress(progress) => {
                if let Some(total_duration) = duration_secs {
                    let elapsed_secs = if let Some(duration) = crate::utils::parse_ffmpeg_time(&progress.time) {
                        duration
                    } else {
                        progress.time.parse::<f64>().unwrap_or(0.0)
                    };
                    
                    let percent = if total_duration > 0.0 {
                        (elapsed_secs / total_duration * 100.0).min(100.0)
                    } else {
                        0.0
                    };
                    
                    // Only report progress when it changes by at least 3% (like the original implementation)
                    if percent >= last_progress_percent + 3.0 || 
                       (percent >= 100.0 && last_progress_percent < 100.0) {
                        
                        
                        crate::progress_reporting::progress(
                            percent as f32,
                            elapsed_secs,
                            total_duration
                        );
                        
                        last_progress_percent = percent;
                    }
                }
            }
            _ => {}
        }
    }
    
    // Show 100% completion and finish (leave visible)
    if let Some(total_duration) = duration_secs {
        crate::progress_reporting::progress(100.0, total_duration, total_duration);
    }
    crate::progress_reporting::finish_progress();

    // FFmpeg finished - check status
    let status = std::process::ExitStatus::default();
    let filename = crate::utils::get_filename_safe(&params.input_path)
        .unwrap_or_else(|_| params.input_path.display().to_string());

    if status.success() {
        crate::progress_reporting::info(""); // Blank line after progress bar
        crate::progress_reporting::success(&format!("Encode finished successfully for {}", filename));
        Ok(())
    } else {
        let error_message = format!(
            "FFmpeg process exited with non-zero status ({:?}). Stderr output:\n{}",
            status.code(),
            stderr_buffer.trim()
        );

        let filename = crate::utils::get_filename_safe(&params.input_path)
        .unwrap_or_else(|_| params.input_path.display().to_string());
        crate::progress_reporting::error(&format!("FFmpeg error for {}: {}", filename, error_message));

        // Check for specific error types
        if stderr_buffer.contains("No streams found") {
            Err(CoreError::NoStreamsFound(filename.to_string()))
        } else {
            Err(command_failed_error(
                "ffmpeg (sidecar)",
                status,
                error_message,
            ))
        }
    }
}






