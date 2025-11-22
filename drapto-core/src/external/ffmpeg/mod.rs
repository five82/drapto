//! FFmpeg command building and execution for video encoding
//!
//! This module handles building complex ffmpeg command lines with
//! appropriate arguments for video (libsvtav1) and audio (libopus) encoding,
//! progress reporting, and error handling.

use crate::error::{CoreError, CoreResult, command_failed_error};
use crate::external::AudioStreamInfo;
use crate::processing::audio;
use crate::reporting::{ProgressSnapshot, Reporter};

use ffmpeg_sidecar::command::FfmpegCommand;
use log::debug;

use std::path::PathBuf;
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
    /// Optional override for SVT-AV1 logical processor usage.
    pub logical_processors: Option<u32>,
    pub crop_filter: Option<String>,
    pub audio_channels: Vec<u32>,
    pub audio_streams: Option<Vec<AudioStreamInfo>>,
    pub duration: f64,
    // Actual values that will be used in FFmpeg command (for display purposes)
    pub video_codec: String,
    pub pixel_format: String,
    pub matrix_coefficients: String,
    pub audio_codec: String,
}

/// Builds FFmpeg command for libsvtav1 video and libopus audio encoding.
pub fn build_ffmpeg_command(
    params: &EncodeParams,
    disable_audio: bool,
) -> CoreResult<FfmpegCommand> {
    // Use the new builder for common setup
    let mut cmd = crate::external::FfmpegCommandBuilder::new().build();
    cmd.input(params.input_path.to_string_lossy().as_ref());

    // Audio filter will be applied per-stream later for transcoded streams only
    let filter_chain = crate::external::VideoFilterChain::new()
        .add_crop(params.crop_filter.as_deref().unwrap_or(""))
        .build();

    if let Some(ref filters) = filter_chain {
        cmd.args(["-vf", filters]);
        log::debug!("Applying video filters: {}", filters);
    } else {
        log::debug!("No video filters applied.");
    }

    // Video encoding configuration - use actual codec from params
    cmd.args(["-c:v", &params.video_codec]);
    cmd.args(["-pix_fmt", &params.pixel_format]);
    cmd.args(["-crf", &params.quality.to_string()]);
    cmd.args(["-preset", &params.preset.to_string()]);

    let mut svtav1_params_builder = crate::external::SvtAv1ParamsBuilder::new()
        .with_ac_bias(params.ac_bias)
        .with_enable_variance_boost(params.enable_variance_boost);

    if params.enable_variance_boost {
        svtav1_params_builder = svtav1_params_builder
            .with_variance_boost_strength(params.variance_boost_strength)
            .with_variance_octile(params.variance_octile);
    }

    svtav1_params_builder = svtav1_params_builder.with_tune(params.tune);

    if let Some(lp) = params.logical_processors {
        svtav1_params_builder = svtav1_params_builder.add_param("lp", &lp.to_string());
        log::debug!("SVT-AV1 logical processors limited to {}", lp);
    }

    let svtav1_params = svtav1_params_builder.build();
    cmd.args(["-svtav1-params", &svtav1_params]);

    if !disable_audio {
        // Map video stream
        cmd.args(["-map", "0:v:0"]);

        // Handle audio streams with per-stream mapping for precise control
        if let Some(ref audio_streams) = params.audio_streams {
            // Always use per-stream mapping for consistency and precise control
            for (output_index, stream) in audio_streams.iter().enumerate() {
                cmd.args(["-map", &format!("0:a:{}", stream.index)]);

                // Always transcode audio to Opus
                cmd.args([&format!("-c:a:{}", output_index), &params.audio_codec]);
                let bitrate = audio::calculate_audio_bitrate(stream.channels);
                cmd.args([&format!("-b:a:{}", output_index), &format!("{bitrate}k")]);
                // Apply audio format filter to all audio streams
                cmd.args([
                    &format!("-filter:a:{}", output_index),
                    "aformat=channel_layouts=7.1|5.1|stereo|mono",
                ]);
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
    total_frames: u64,
    reporter: Option<&dyn Reporter>,
) -> CoreResult<()> {
    run_ffmpeg_encode_internal(params, disable_audio, total_frames, reporter)
}

fn run_ffmpeg_encode_internal(
    params: &EncodeParams,
    disable_audio: bool,
    total_frames: u64,
    reporter: Option<&dyn Reporter>,
) -> CoreResult<()> {
    debug!("Output: {}", params.output_path.display());

    let filename = crate::utils::get_filename_safe(&params.input_path)
        .unwrap_or_else(|_| params.input_path.display().to_string());

    log::info!(
        target: "drapto::progress",
        "Starting encode: {} -> {}",
        params.input_path.display(),
        params.output_path.display()
    );

    debug!("Encode parameters: {params:?}");

    let mut cmd = build_ffmpeg_command(params, disable_audio)?;
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

                    if let Some(rep) = reporter {
                        rep.encoding_progress(&ProgressSnapshot {
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

#[cfg(test)]
mod tests;
