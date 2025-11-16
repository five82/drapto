//! FFmpeg command building and execution for video encoding
//!
//! This module handles building complex ffmpeg command lines with
//! appropriate arguments for video (libsvtav1) and audio (libopus) encoding,
//! progress reporting, and error handling.

use crate::error::{CoreError, CoreResult, command_failed_error};
use crate::events::{Event, EventDispatcher};
use crate::external::AudioStreamInfo;
use crate::processing::audio;

use ffmpeg_sidecar::command::FfmpegCommand;
use log::debug;

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Parameters required for running an `FFmpeg` encode operation.
#[derive(Debug, Clone)]
pub struct EncodeParams {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub quality: u32,
    pub preset: u8,
    pub tune: u8,
    pub ac_bias: f32,
    pub enable_variance_boost: bool,
    pub variance_boost_strength: u8,
    pub variance_octile: u8,
    /// Whether to use hardware decoding (when available)
    pub use_hw_decode: bool,
    /// Optional override for SVT-AV1 logical processor usage.
    pub logical_processors: Option<u32>,
    pub crop_filter: Option<String>,
    pub audio_channels: Vec<u32>,
    pub audio_streams: Option<Vec<AudioStreamInfo>>,
    pub duration: f64,
    /// The adaptive hqdn3d parameters based on noise analysis (used if override is not provided).
    pub hqdn3d_params: Option<String>,
    // Actual values that will be used in FFmpeg command (for display purposes)
    pub video_codec: String,
    pub pixel_format: String,
    pub matrix_coefficients: String,
    pub audio_codec: String,
    pub film_grain_level: u8,
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

    // Audio filter will be applied per-stream later for transcoded streams only
    let hqdn3d_to_use = if has_denoising {
        hqdn3d_override.or(params.hqdn3d_params.as_deref())
    } else {
        None
    };
    let filter_chain = crate::external::VideoFilterChain::new()
        .add_denoise(hqdn3d_to_use.unwrap_or(""))
        .add_crop(params.crop_filter.as_deref().unwrap_or(""))
        .build();

    if let Some(ref filters) = filter_chain {
        cmd.args(["-vf", filters]);
        log::debug!("Applying video filters: {}", filters);
    } else {
        log::debug!("No video filters applied.");
    }

    // Use film grain level from params (single source of truth)
    let film_grain_value = params.film_grain_level;

    // Video encoding configuration - use actual codec from params
    cmd.args(["-c:v", &params.video_codec]);
    cmd.args(["-pix_fmt", &params.pixel_format]);
    cmd.args(["-crf", &params.quality.to_string()]);
    cmd.args(["-preset", &params.preset.to_string()]);

    let mut svtav1_params_builder = crate::external::SvtAv1ParamsBuilder::new()
        .with_ac_bias(params.ac_bias)
        .with_enable_variance_boost(params.enable_variance_boost)
        .with_variance_boost_strength(params.variance_boost_strength)
        .with_variance_octile(params.variance_octile)
        .with_tune(params.tune)
        .with_film_grain(film_grain_value);

    if let Some(lp) = params.logical_processors {
        svtav1_params_builder = svtav1_params_builder.add_param("lp", &lp.to_string());
        log::debug!("SVT-AV1 logical processors limited to {}", lp);
    }

    let svtav1_params = svtav1_params_builder.build();
    cmd.args(["-svtav1-params", &svtav1_params]);

    if film_grain_value > 0 {
        log::debug!("Applying film grain synthesis: level={}", film_grain_value);
    } else {
        log::debug!("No film grain synthesis applied (denoise level is None or 0).");
    }

    if !disable_audio {
        // Map video stream
        cmd.args(["-map", "0:v:0"]);

        // Handle audio streams with per-stream mapping for precise control
        if let Some(ref audio_streams) = params.audio_streams {
            // Always use per-stream mapping for consistency and precise control
            for (output_index, stream) in audio_streams.iter().enumerate() {
                cmd.args(["-map", &format!("0:a:{}", stream.index)]);

                if stream.is_spatial {
                    // Copy spatial audio tracks to preserve Atmos/DTS:X
                    cmd.args([&format!("-c:a:{}", output_index), "copy"]);
                    log::info!(
                        "Copying spatial audio stream {} ({} {})",
                        output_index,
                        stream.codec_name,
                        stream.profile.as_deref().unwrap_or("")
                    );
                } else {
                    // Transcode non-spatial audio to Opus
                    cmd.args([&format!("-c:a:{}", output_index), &params.audio_codec]);
                    let bitrate = audio::calculate_audio_bitrate(stream.channels);
                    cmd.args([&format!("-b:a:{}", output_index), &format!("{bitrate}k")]);
                    // Apply audio format filter only to transcoded streams
                    cmd.args([
                        &format!("-filter:a:{}", output_index),
                        "aformat=channel_layouts=7.1|5.1|stereo|mono",
                    ]);
                }
            }
        } else {
            // Fallback to old behavior if no detailed stream info
            cmd.args(["-c:a", &params.audio_codec]);
            for (i, &channels) in params.audio_channels.iter().enumerate() {
                let bitrate = audio::calculate_audio_bitrate(channels);
                cmd.args([&format!("-b:a:{i}"), &format!("{bitrate}k")]);
                // Apply audio format filter to all streams in fallback mode
                cmd.args([
                    &format!("-filter:a:{i}"),
                    "aformat=channel_layouts=7.1|5.1|stereo|mono",
                ]);
            }
            cmd.args(["-map", "0:a"]);
        }

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
    total_frames: u64,
    event_dispatcher: Option<&EventDispatcher>,
) -> CoreResult<()> {
    run_ffmpeg_encode_internal(
        params,
        disable_audio,
        has_denoising,
        total_frames,
        event_dispatcher,
        false,
    )
}

fn run_ffmpeg_encode_internal(
    params: &EncodeParams,
    disable_audio: bool,
    has_denoising: bool,
    total_frames: u64,
    event_dispatcher: Option<&EventDispatcher>,
    is_retry: bool,
) -> CoreResult<()> {
    debug!("Output: {}", params.output_path.display());

    let filename = crate::utils::get_filename_safe(&params.input_path)
        .unwrap_or_else(|_| params.input_path.display().to_string());

    if is_retry {
        log::info!(
            target: "drapto::progress",
            "Retrying encode without hardware decoding: {} -> {}",
            params.input_path.display(),
            params.output_path.display()
        );
    } else {
        log::info!(
            target: "drapto::progress",
            "Starting encode: {} -> {}",
            params.input_path.display(),
            params.output_path.display()
        );
    }

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

    if duration_secs.is_none() || duration_secs == Some(0.0) {
        log::warn!(
            "Video duration not provided or zero; progress percentage will not be accurate."
        );
    }

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
                stderr_buffer.push_str(&message);
                stderr_buffer.push('\n');
            }
            ffmpeg_sidecar::event::FfmpegEvent::Error(error) => {
                stderr_buffer.push_str(&format!("ERROR: {}\n", error));
            }
            ffmpeg_sidecar::event::FfmpegEvent::Progress(progress) => {
                if let Some(total_duration) = duration_secs {
                    let elapsed_secs =
                        if let Some(duration) = crate::utils::parse_ffmpeg_time(&progress.time) {
                            duration
                        } else {
                            progress.time.parse::<f64>().unwrap_or(0.0)
                        };

                    let percent = if total_duration > 0.0 {
                        (elapsed_secs / total_duration * 100.0).min(100.0)
                    } else {
                        0.0
                    };

                    // Parse additional progress information from FFmpeg
                    let speed = progress.speed;
                    let fps = progress.fps;
                    let frame = progress.frame as u64;
                    let bitrate = format!("{:.1}kbps", progress.bitrate_kbps);

                    // Calculate ETA
                    let eta = if speed > 0.0 {
                        let remaining_duration = total_duration - elapsed_secs;
                        let eta_seconds = (remaining_duration / speed as f64) as u64;
                        Duration::from_secs(eta_seconds)
                    } else {
                        Duration::from_secs(0)
                    };

                    // Emit progress event
                    if let Some(dispatcher) = event_dispatcher {
                        dispatcher.emit(Event::EncodingProgress {
                            current_frame: frame,
                            total_frames,
                            percent: percent as f32,
                            speed,
                            fps,
                            eta,
                            bitrate,
                        });
                    }

                    // Always report progress (both for progress bar and logging)
                    log::debug!(
                        "Encoding progress: {:.1}% (elapsed: {:.1}s)",
                        percent,
                        elapsed_secs
                    );
                }
            }
            _ => {}
        }
    }

    // Just finish the progress bar - 100% will have been reported by the event loop
    log::debug!("Encoding progress finished");

    // FFmpeg finished - get the actual exit status
    let status = child.wait().map_err(|e| {
        command_failed_error(
            "ffmpeg",
            std::process::ExitStatus::default(),
            format!("Failed to wait for FFmpeg process: {e}"),
        )
    })?;

    if status.success() {
        log::info!("Encode finished successfully for {}", filename);
        Ok(())
    } else {
        if params.use_hw_decode && !is_retry && should_retry_without_hw_decode(&stderr_buffer) {
            let warning_message = format!(
                "Hardware decoding failed for {}. Retrying without hardware acceleration.",
                filename
            );

            log::warn!("{}", warning_message);

            if let Some(dispatcher) = event_dispatcher {
                dispatcher.emit(Event::Warning {
                    message: warning_message.clone(),
                });
            }

            log::debug!("VAAPI stderr output:\n{}", stderr_buffer.trim());

            cleanup_partial_output(&params.output_path);

            let mut software_params = params.clone();
            software_params.use_hw_decode = false;

            return run_ffmpeg_encode_internal(
                &software_params,
                disable_audio,
                has_denoising,
                total_frames,
                event_dispatcher,
                true,
            );
        }

        let error_message = format!(
            "FFmpeg process exited with non-zero status ({:?}). Stderr output:\n{}",
            status.code(),
            stderr_buffer.trim()
        );

        log::error!("FFmpeg error for {}: {}", filename, error_message);

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

fn should_retry_without_hw_decode(stderr: &str) -> bool {
    let stderr_lower = stderr.to_lowercase();
    const PATTERNS: &[&str] = &[
        "no va display found",
        "hardware device setup failed",
        "device creation failed",
        "libva error",
        "no device available for decoder",
        "vainitialize failed",
    ];

    PATTERNS
        .iter()
        .any(|pattern| stderr_lower.contains(pattern))
}

fn cleanup_partial_output(path: &Path) {
    match fs::remove_file(path) {
        Ok(_) => {
            log::warn!(
                "Removed partial output created during failed hardware decode: {}",
                path.display()
            );
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => {
            log::warn!(
                "Failed to remove partial output at {}: {}",
                path.display(),
                err
            );
        }
    }
}

#[cfg(test)]
mod tests;
