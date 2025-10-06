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

    let filename = crate::utils::get_filename_safe(&params.input_path)
        .unwrap_or_else(|_| params.input_path.display().to_string());

    if status.success() {
        log::info!("Encode finished successfully for {}", filename);
        Ok(())
    } else {
        let error_message = format!(
            "FFmpeg process exited with non-zero status ({:?}). Stderr output:\n{}",
            status.code(),
            stderr_buffer.trim()
        );

        let filename = crate::utils::get_filename_safe(&params.input_path)
            .unwrap_or_else(|_| params.input_path.display().to_string());
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
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Create test parameters with common defaults
    fn create_test_params() -> EncodeParams {
        EncodeParams {
            input_path: PathBuf::from("/test/input.mkv"),
            output_path: PathBuf::from("/test/output.mkv"),
            quality: 27,
            preset: 6,
            tune: 3,
            use_hw_decode: false,
            logical_processors: None,
            crop_filter: None,
            audio_channels: vec![6], // 5.1 audio
            audio_streams: None,
            duration: 3600.0,
            hqdn3d_params: Some("2:1.5:3:2.5".to_string()), // Example SDR denoising params
            // Actual values used in FFmpeg command
            video_codec: "libsvtav1".to_string(),
            pixel_format: "yuv420p10le".to_string(),
            matrix_coefficients: "bt709".to_string(),
            audio_codec: "libopus".to_string(),
            film_grain_level: crate::config::MIN_FILM_GRAIN_VALUE,
        }
    }

    /// Create test parameters with crop filter
    fn create_test_params_with_crop(crop: Option<&str>) -> EncodeParams {
        let mut params = create_test_params();
        params.crop_filter = crop.map(|c| c.to_string());
        params
    }

    #[test]
    fn test_video_filter_chain_with_sdr_denoising() {
        // Test that SDR denoising parameters are correctly included in video filters
        let params = create_test_params();

        let hqdn3d_to_use = params.hqdn3d_params.as_deref();
        let filter_chain = crate::external::VideoFilterChain::new()
            .add_denoise(hqdn3d_to_use.unwrap_or(""))
            .add_crop(params.crop_filter.as_deref().unwrap_or(""))
            .build();

        assert!(
            filter_chain.is_some(),
            "Should have video filters when denoising is enabled"
        );
        let filters = filter_chain.unwrap();
        assert!(
            filters.contains("hqdn3d=2:1.5:3:2.5"),
            "Video filters should contain test SDR HQDN3D parameters"
        );
    }

    #[test]
    fn test_video_filter_chain_with_hdr_denoising() {
        // Test that HDR denoising parameters are correctly used
        let hdr_params = Some("1:0.8:2.5:2".to_string()); // Example HDR denoising params

        let filter_chain = crate::external::VideoFilterChain::new()
            .add_denoise(hdr_params.as_deref().unwrap_or(""))
            .build();

        assert!(
            filter_chain.is_some(),
            "Should have video filters for HDR denoising"
        );
        let filters = filter_chain.unwrap();
        assert!(
            filters.contains("hqdn3d=1:0.8:2.5:2"),
            "Video filters should contain test HDR HQDN3D parameters"
        );
    }

    #[test]
    fn test_svt_av1_params_with_film_grain() {
        // Test that film grain synthesis is correctly applied when denoising is enabled
        let svtav1_params = crate::external::SvtAv1ParamsBuilder::new()
            .with_tune(3)
            .with_film_grain(crate::config::MIN_FILM_GRAIN_VALUE)
            .build();

        assert!(
            svtav1_params.contains("tune=3"),
            "SVT-AV1 params should contain tune=3"
        );
        assert!(
            svtav1_params.contains(&format!(
                "film-grain={}",
                crate::config::MIN_FILM_GRAIN_VALUE
            )),
            "SVT-AV1 params should contain film-grain={}",
            crate::config::MIN_FILM_GRAIN_VALUE
        );
        assert!(
            svtav1_params.contains("film-grain-denoise=0"),
            "SVT-AV1 params should disable built-in film grain denoising"
        );
    }

    #[test]
    fn test_svt_av1_params_with_logical_processor_limit() {
        let mut params = create_test_params();
        params.logical_processors = Some(10);

        let cmd = build_ffmpeg_command(&params, None, false, true).unwrap();
        let cmd_string = format!("{:?}", cmd);

        assert!(
            cmd_string.contains("lp=10"),
            "Command should include lp parameter when logical processors are limited: {}",
            cmd_string
        );
    }

    #[test]
    fn test_svt_av1_params_without_film_grain() {
        // Test that film grain is not applied when denoising is disabled
        let svtav1_params = crate::external::SvtAv1ParamsBuilder::new()
            .with_tune(3)
            .with_film_grain(0) // No film grain
            .build();

        assert!(
            svtav1_params.contains("tune=3"),
            "SVT-AV1 params should contain tune=3"
        );
        assert!(
            !svtav1_params.contains("film-grain="),
            "SVT-AV1 params should not contain film-grain when disabled"
        );
    }

    #[test]
    fn test_audio_bitrate_calculation() {
        // Test that audio bitrates match what's displayed in configuration
        use crate::processing::audio::calculate_audio_bitrate;

        // Test common channel configurations
        assert_eq!(calculate_audio_bitrate(1), 64, "Mono should be 64kbps");
        assert_eq!(calculate_audio_bitrate(2), 128, "Stereo should be 128kbps");
        assert_eq!(calculate_audio_bitrate(6), 256, "5.1 should be 256kbps");
        assert_eq!(calculate_audio_bitrate(8), 384, "7.1 should be 384kbps");
    }

    #[test]
    fn test_video_filter_chain_with_crop_and_denoise() {
        // Test that both crop and denoise filters are properly combined
        let filter_chain = crate::external::VideoFilterChain::new()
            .add_denoise("2:1.5:3:2.5")
            .add_crop("crop=1920:800:0:140")
            .build();

        assert!(filter_chain.is_some(), "Should have video filters");
        let filters = filter_chain.unwrap();
        assert!(filters.contains("hqdn3d="), "Should contain HQDN3D filter");
        assert!(
            filters.contains("crop=1920:800:0:140"),
            "Should contain crop filter"
        );
        assert!(filters.contains(","), "Filters should be comma-separated");
    }

    #[test]
    fn test_configuration_constants_consistency() {
        // Test that our configuration constants are reasonable values

        // Film grain constants should be reasonable
        assert!(
            crate::config::MIN_FILM_GRAIN_VALUE <= 50,
            "Min film grain should be <= 50"
        );
        assert!(
            crate::config::MIN_FILM_GRAIN_VALUE > 0,
            "Min film grain should be > 0"
        );
        assert!(
            crate::config::MAX_FILM_GRAIN_VALUE <= 50,
            "Max film grain should be <= 50"
        );
        assert!(
            crate::config::MAX_FILM_GRAIN_VALUE > crate::config::MIN_FILM_GRAIN_VALUE,
            "Max should be > min"
        );

        // Resolution thresholds should be reasonable
        assert!(
            crate::config::HD_WIDTH_THRESHOLD < crate::config::UHD_WIDTH_THRESHOLD,
            "HD threshold should be < UHD threshold"
        );
        assert!(
            crate::config::HD_WIDTH_THRESHOLD >= 1920,
            "HD threshold should be >= 1920"
        );
        assert!(
            crate::config::UHD_WIDTH_THRESHOLD >= 3840,
            "UHD threshold should be >= 3840"
        );
    }

    #[test]
    fn test_build_ffmpeg_command_with_crop_filter() {
        // Test that crop filters are correctly included in the full FFmpeg command
        let params = create_test_params_with_crop(Some("crop=1920:800:0:140"));
        let cmd = build_ffmpeg_command(&params, None, false, true).unwrap();
        let cmd_string = format!("{:?}", cmd);

        assert!(
            cmd_string.contains("-vf"),
            "Command should contain video filter argument"
        );
        assert!(
            cmd_string.contains("crop=1920:800:0:140"),
            "Command should contain crop filter"
        );
        assert!(
            cmd_string.contains("hqdn3d="),
            "Command should contain denoise filter when denoising enabled"
        );
    }

    #[test]
    fn test_build_ffmpeg_command_with_crop_and_denoise() {
        // Test that crop and denoise filters are properly chained
        let params = create_test_params_with_crop(Some("crop=1920:1036:0:22"));
        let cmd = build_ffmpeg_command(&params, None, false, true).unwrap();
        let cmd_string = format!("{:?}", cmd);

        // Should contain both filters properly chained
        assert!(
            cmd_string.contains("crop=1920:1036:0:22"),
            "Command should contain crop filter"
        );
        assert!(
            cmd_string.contains("hqdn3d="),
            "Command should contain denoise filter"
        );
        assert!(
            cmd_string.contains("hqdn3d=2:1.5:3:2.5,crop=1920:1036:0:22"),
            "Filters should be properly chained in denoise,crop order"
        );
    }

    #[test]
    fn test_build_ffmpeg_command_without_crop_filter() {
        // Test that FFmpeg command works correctly when no crop is needed
        let params = create_test_params_with_crop(None);
        let cmd = build_ffmpeg_command(&params, None, false, true).unwrap();
        let cmd_string = format!("{:?}", cmd);

        assert!(
            !cmd_string.contains("crop="),
            "Command should not contain crop filter"
        );
        assert!(
            cmd_string.contains("hqdn3d="),
            "Command should still contain denoise filter"
        );
    }

    #[test]
    fn test_build_ffmpeg_command_crop_only_no_denoise() {
        // Test crop filter without denoising
        let params = create_test_params_with_crop(Some("crop=1920:800:0:140"));
        let cmd = build_ffmpeg_command(&params, None, false, false).unwrap();
        let cmd_string = format!("{:?}", cmd);

        assert!(
            cmd_string.contains("-vf"),
            "Command should contain video filter argument"
        );
        assert!(
            cmd_string.contains("crop=1920:800:0:140"),
            "Command should contain crop filter"
        );
        assert!(
            !cmd_string.contains("hqdn3d"),
            "Command should not contain denoise filter when denoising disabled"
        );
    }

    #[test]
    fn test_encode_params_match_ffmpeg_command() {
        // Test that EncodeParams fields accurately reflect what's used in the FFmpeg command
        let params = EncodeParams {
            input_path: std::path::PathBuf::from("test_input.mkv"),
            output_path: std::path::PathBuf::from("test_output.mkv"),
            quality: 27,
            preset: 6,
            tune: 3,
            use_hw_decode: false,
            logical_processors: None,
            crop_filter: Some("crop=1920:1036:0:22".to_string()),
            audio_channels: vec![6], // 5.1 surround
            audio_streams: None,
            duration: 120.0,
            hqdn3d_params: Some("2:1.5:3:2.5".to_string()),
            // Test the actual values that should match FFmpeg command
            video_codec: "libsvtav1".to_string(),
            pixel_format: "yuv420p10le".to_string(),
            matrix_coefficients: "bt709".to_string(),
            audio_codec: "libopus".to_string(),
            film_grain_level: 4,
        };

        let cmd = build_ffmpeg_command(&params, None, false, true).unwrap();
        let cmd_string = format!("{:?}", cmd);

        // Validate video codec matches
        assert!(
            cmd_string.contains("-c:v"),
            "Command should contain video codec flag"
        );
        assert!(
            cmd_string.contains(&params.video_codec),
            "Command should contain video codec: {}",
            params.video_codec
        );

        // Validate pixel format matches
        assert!(
            cmd_string.contains("-pix_fmt"),
            "Command should contain pixel format flag"
        );
        assert!(
            cmd_string.contains(&params.pixel_format),
            "Command should contain pixel format: {}",
            params.pixel_format
        );

        // Validate audio codec matches
        assert!(
            cmd_string.contains("-c:a"),
            "Command should contain audio codec flag"
        );
        assert!(
            cmd_string.contains(&params.audio_codec),
            "Command should contain audio codec: {}",
            params.audio_codec
        );

        // Validate preset matches
        assert!(
            cmd_string.contains("-preset"),
            "Command should contain preset flag"
        );
        assert!(
            cmd_string.contains(&params.preset.to_string()),
            "Command should contain preset: {}",
            params.preset
        );

        // Validate quality (CRF) matches
        assert!(
            cmd_string.contains("-crf"),
            "Command should contain CRF flag"
        );
        assert!(
            cmd_string.contains(&params.quality.to_string()),
            "Command should contain quality: {}",
            params.quality
        );

        // Validate film grain level matches (in svtav1-params)
        assert!(
            cmd_string.contains("-svtav1-params"),
            "Command should contain SVT-AV1 params"
        );
        assert!(
            cmd_string.contains(&format!("film-grain={}", params.film_grain_level)),
            "Command should contain film grain level: {}",
            params.film_grain_level
        );

        // Validate tune parameter matches (in svtav1-params)
        assert!(
            cmd_string.contains(&format!("tune={}", params.tune)),
            "Command should contain tune parameter: {}",
            params.tune
        );

        // Validate hqdn3d params match
        if let Some(ref hqdn3d) = params.hqdn3d_params {
            assert!(
                cmd_string.contains(&format!("hqdn3d={}", hqdn3d)),
                "Command should contain hqdn3d params: {}",
                hqdn3d
            );
        }

        // Validate audio bitrate is calculated correctly for 5.1 (6 channels)
        let expected_bitrate = crate::processing::audio::calculate_audio_bitrate(6); // 256 kbps for 5.1
        assert!(
            cmd_string.contains(&format!("{}k", expected_bitrate)),
            "Command should contain correct audio bitrate: {}k",
            expected_bitrate
        );
    }

    #[test]
    fn test_encode_params_film_grain_disabled() {
        // Test that film grain level 0 results in no film grain in FFmpeg command
        let params = EncodeParams {
            input_path: std::path::PathBuf::from("test_input.mkv"),
            output_path: std::path::PathBuf::from("test_output.mkv"),
            quality: 27,
            preset: 6,
            tune: 3,
            use_hw_decode: false,
            logical_processors: None,
            crop_filter: None,
            audio_channels: vec![2], // Stereo
            audio_streams: None,
            duration: 120.0,
            hqdn3d_params: None, // No denoising
            video_codec: "libsvtav1".to_string(),
            pixel_format: "yuv420p10le".to_string(),
            matrix_coefficients: "bt709".to_string(),
            audio_codec: "libopus".to_string(),
            film_grain_level: 0, // Disabled
        };

        let cmd = build_ffmpeg_command(&params, None, false, false).unwrap();
        let cmd_string = format!("{:?}", cmd);

        // Film grain should not appear in command when disabled (level 0)
        assert!(
            !cmd_string.contains("film-grain="),
            "Command should not contain film-grain parameter when disabled. Command: {}",
            cmd_string
        );

        // Should not contain hqdn3d when denoising is disabled
        assert!(
            !cmd_string.contains("hqdn3d"),
            "Command should not contain hqdn3d when denoising disabled"
        );
    }

    #[test]
    fn test_spatial_audio_command_generation() {
        use crate::external::AudioStreamInfo;

        // Test single spatial audio track (Dolby Atmos)
        let spatial_stream = AudioStreamInfo {
            channels: 8,
            codec_name: "truehd".to_string(),
            profile: Some("Dolby TrueHD + Dolby Atmos".to_string()),
            index: 0,
            is_spatial: true,
        };

        let params = EncodeParams {
            input_path: std::path::PathBuf::from("test_input.mkv"),
            output_path: std::path::PathBuf::from("test_output.mkv"),
            quality: 27,
            preset: 6,
            tune: 3,
            use_hw_decode: false,
            logical_processors: None,
            crop_filter: None,
            audio_channels: vec![8],
            audio_streams: Some(vec![spatial_stream]),
            duration: 120.0,
            hqdn3d_params: None,
            video_codec: "libsvtav1".to_string(),
            pixel_format: "yuv420p10le".to_string(),
            matrix_coefficients: "bt709".to_string(),
            audio_codec: "libopus".to_string(),
            film_grain_level: 0,
        };

        let cmd = build_ffmpeg_command(&params, None, false, false).unwrap();
        let cmd_string = format!("{:?}", cmd);

        // Should use copy codec for spatial audio
        assert!(
            cmd_string.contains("-c:a:0") && cmd_string.contains("copy"),
            "Command should contain copy codec for spatial audio: {}",
            cmd_string
        );

        // Should map audio stream correctly
        assert!(
            cmd_string.contains("-map") && cmd_string.contains("0:a:0"),
            "Command should map first audio stream: {}",
            cmd_string
        );

        // Should NOT contain opus bitrate settings for spatial audio
        assert!(
            !cmd_string.contains("-b:a:0"),
            "Command should not contain bitrate for copied spatial audio: {}",
            cmd_string
        );
    }

    #[test]
    fn test_mixed_spatial_non_spatial_audio_command() {
        use crate::external::AudioStreamInfo;

        // Mixed scenario: spatial + commentary track
        let spatial_stream = AudioStreamInfo {
            channels: 8,
            codec_name: "truehd".to_string(),
            profile: Some("Dolby TrueHD + Dolby Atmos".to_string()),
            index: 0,
            is_spatial: true,
        };

        let commentary_stream = AudioStreamInfo {
            channels: 2,
            codec_name: "aac".to_string(),
            profile: Some("LC".to_string()),
            index: 1,
            is_spatial: false,
        };

        let params = EncodeParams {
            input_path: std::path::PathBuf::from("test_input.mkv"),
            output_path: std::path::PathBuf::from("test_output.mkv"),
            quality: 27,
            preset: 6,
            tune: 3,
            use_hw_decode: false,
            logical_processors: None,
            crop_filter: None,
            audio_channels: vec![8, 2],
            audio_streams: Some(vec![spatial_stream, commentary_stream]),
            duration: 120.0,
            hqdn3d_params: None,
            video_codec: "libsvtav1".to_string(),
            pixel_format: "yuv420p10le".to_string(),
            matrix_coefficients: "bt709".to_string(),
            audio_codec: "libopus".to_string(),
            film_grain_level: 0,
        };

        let cmd = build_ffmpeg_command(&params, None, false, false).unwrap();
        let cmd_string = format!("{:?}", cmd);

        // First stream should be copied (spatial)
        assert!(
            cmd_string.contains("-c:a:0") && cmd_string.contains("copy"),
            "First stream should use copy codec: {}",
            cmd_string
        );

        // Second stream should use opus
        assert!(
            cmd_string.contains("-c:a:1") && cmd_string.contains("libopus"),
            "Second stream should use opus codec: {}",
            cmd_string
        );

        // Second stream should have bitrate
        assert!(
            cmd_string.contains("-b:a:1") && cmd_string.contains("128k"),
            "Second stream should have opus bitrate: {}",
            cmd_string
        );

        // Should map both streams
        assert!(
            cmd_string.contains("0:a:0") && cmd_string.contains("0:a:1"),
            "Command should map both audio streams: {}",
            cmd_string
        );
    }

    #[test]
    fn test_dtsx_audio_command_generation() {
        use crate::external::AudioStreamInfo;

        // Test DTS:X spatial audio track
        let dtsx_stream = AudioStreamInfo {
            channels: 8,
            codec_name: "dts".to_string(),
            profile: Some("DTS:X".to_string()),
            index: 0,
            is_spatial: true,
        };

        let params = EncodeParams {
            input_path: std::path::PathBuf::from("test_input.mkv"),
            output_path: std::path::PathBuf::from("test_output.mkv"),
            quality: 27,
            preset: 6,
            tune: 3,
            use_hw_decode: false,
            logical_processors: None,
            crop_filter: None,
            audio_channels: vec![8],
            audio_streams: Some(vec![dtsx_stream]),
            duration: 120.0,
            hqdn3d_params: None,
            video_codec: "libsvtav1".to_string(),
            pixel_format: "yuv420p10le".to_string(),
            matrix_coefficients: "bt709".to_string(),
            audio_codec: "libopus".to_string(),
            film_grain_level: 0,
        };

        let cmd = build_ffmpeg_command(&params, None, false, false).unwrap();
        let cmd_string = format!("{:?}", cmd);

        // Should use copy codec for DTS:X
        assert!(
            cmd_string.contains("-c:a:0") && cmd_string.contains("copy"),
            "Command should contain copy codec for DTS:X audio: {}",
            cmd_string
        );
    }

    #[test]
    fn test_eac3_joc_audio_command_generation() {
        use crate::external::AudioStreamInfo;

        // Test E-AC-3 with JOC (Atmos) spatial audio track
        let eac3_joc_stream = AudioStreamInfo {
            channels: 8,
            codec_name: "eac3".to_string(),
            profile: Some("Dolby Digital Plus + JOC".to_string()),
            index: 0,
            is_spatial: true,
        };

        let params = EncodeParams {
            input_path: std::path::PathBuf::from("test_input.mkv"),
            output_path: std::path::PathBuf::from("test_output.mkv"),
            quality: 27,
            preset: 6,
            tune: 3,
            use_hw_decode: false,
            logical_processors: None,
            crop_filter: None,
            audio_channels: vec![8],
            audio_streams: Some(vec![eac3_joc_stream]),
            duration: 120.0,
            hqdn3d_params: None,
            video_codec: "libsvtav1".to_string(),
            pixel_format: "yuv420p10le".to_string(),
            matrix_coefficients: "bt709".to_string(),
            audio_codec: "libopus".to_string(),
            film_grain_level: 0,
        };

        let cmd = build_ffmpeg_command(&params, None, false, false).unwrap();
        let cmd_string = format!("{:?}", cmd);

        // Should use copy codec for E-AC-3 + JOC
        assert!(
            cmd_string.contains("-c:a:0") && cmd_string.contains("copy"),
            "Command should contain copy codec for E-AC-3 JOC audio: {}",
            cmd_string
        );
    }

    #[test]
    fn test_fallback_behavior_without_detailed_streams() {
        // Test fallback when no detailed audio stream info is available
        let params = EncodeParams {
            input_path: std::path::PathBuf::from("test_input.mkv"),
            output_path: std::path::PathBuf::from("test_output.mkv"),
            quality: 27,
            preset: 6,
            tune: 3,
            use_hw_decode: false,
            logical_processors: None,
            crop_filter: None,
            audio_channels: vec![6], // 5.1 surround
            audio_streams: None,     // No detailed stream info
            duration: 120.0,
            hqdn3d_params: None,
            video_codec: "libsvtav1".to_string(),
            pixel_format: "yuv420p10le".to_string(),
            matrix_coefficients: "bt709".to_string(),
            audio_codec: "libopus".to_string(),
            film_grain_level: 0,
        };

        let cmd = build_ffmpeg_command(&params, None, false, false).unwrap();
        let cmd_string = format!("{:?}", cmd);

        // Should fall back to traditional behavior
        assert!(
            cmd_string.contains("-c:a") && cmd_string.contains("libopus"),
            "Command should use opus codec in fallback mode: {}",
            cmd_string
        );

        assert!(
            cmd_string.contains("-b:a:0") && cmd_string.contains("256k"),
            "Command should contain 5.1 bitrate in fallback mode: {}",
            cmd_string
        );

        assert!(
            cmd_string.contains("-map") && cmd_string.contains("0:a"),
            "Command should map all audio streams in fallback mode: {}",
            cmd_string
        );
    }
}
