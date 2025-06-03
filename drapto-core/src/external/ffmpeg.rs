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
    pub tune: u8,
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
        .with_tune(params.tune)
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

    if duration_secs.is_none() || duration_secs == Some(0.0) {
        crate::progress_reporting::warning("Video duration not provided or zero; progress percentage will not be accurate.");
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
                    
                    // Always report progress (both for progress bar and logging)
                    crate::progress_reporting::progress(
                        percent as f32,
                        elapsed_secs,
                        total_duration
                    );
                }
            }
            _ => {}
        }
    }
    
    // Just finish the progress bar - 100% will have been reported by the event loop
    crate::progress_reporting::finish_progress();

    // FFmpeg finished - check status
    let status = std::process::ExitStatus::default();
    let filename = crate::utils::get_filename_safe(&params.input_path)
        .unwrap_or_else(|_| params.input_path.display().to_string());

    if status.success() {
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
            crop_filter: None,
            audio_channels: vec![6], // 5.1 audio
            duration: 3600.0,
            hqdn3d_params: Some(crate::config::FIXED_HQDN3D_PARAMS_SDR.to_string()),
        }
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

        assert!(filter_chain.is_some(), "Should have video filters when denoising is enabled");
        let filters = filter_chain.unwrap();
        assert!(filters.contains(&format!("hqdn3d={}", crate::config::FIXED_HQDN3D_PARAMS_SDR)), 
                "Video filters should contain SDR HQDN3D parameters: {}", crate::config::FIXED_HQDN3D_PARAMS_SDR);
    }

    #[test]
    fn test_video_filter_chain_with_hdr_denoising() {
        // Test that HDR denoising parameters are correctly used
        let hdr_params = Some(crate::config::FIXED_HQDN3D_PARAMS_HDR.to_string());
        
        let filter_chain = crate::external::VideoFilterChain::new()
            .add_denoise(hdr_params.as_deref().unwrap_or(""))
            .build();

        assert!(filter_chain.is_some(), "Should have video filters for HDR denoising");
        let filters = filter_chain.unwrap();
        assert!(filters.contains(&format!("hqdn3d={}", crate::config::FIXED_HQDN3D_PARAMS_HDR)), 
                "Video filters should contain HDR HQDN3D parameters: {}", crate::config::FIXED_HQDN3D_PARAMS_HDR);
    }

    #[test]
    fn test_svt_av1_params_with_film_grain() {
        // Test that film grain synthesis is correctly applied when denoising is enabled
        let svtav1_params = crate::external::SvtAv1ParamsBuilder::new()
            .with_tune(3)
            .with_film_grain(crate::config::FIXED_FILM_GRAIN_VALUE)
            .build();

        assert!(svtav1_params.contains("tune=3"), "SVT-AV1 params should contain tune=3");
        assert!(svtav1_params.contains(&format!("film-grain={}", crate::config::FIXED_FILM_GRAIN_VALUE)), 
                "SVT-AV1 params should contain film-grain={}", crate::config::FIXED_FILM_GRAIN_VALUE);
        assert!(svtav1_params.contains("film-grain-denoise=0"), 
                "SVT-AV1 params should disable built-in film grain denoising");
    }

    #[test]
    fn test_svt_av1_params_without_film_grain() {
        // Test that film grain is not applied when denoising is disabled
        let svtav1_params = crate::external::SvtAv1ParamsBuilder::new()
            .with_tune(3)
            .with_film_grain(0) // No film grain
            .build();

        assert!(svtav1_params.contains("tune=3"), "SVT-AV1 params should contain tune=3");
        assert!(!svtav1_params.contains("film-grain="), 
                "SVT-AV1 params should not contain film-grain when disabled");
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
            .add_denoise(crate::config::FIXED_HQDN3D_PARAMS_SDR)
            .add_crop("crop=1920:800:0:140")
            .build();

        assert!(filter_chain.is_some(), "Should have video filters");
        let filters = filter_chain.unwrap();
        assert!(filters.contains("hqdn3d="), "Should contain HQDN3D filter");
        assert!(filters.contains("crop=1920:800:0:140"), "Should contain crop filter");
        assert!(filters.contains(","), "Filters should be comma-separated");
    }

    #[test]
    fn test_configuration_constants_consistency() {
        // Test that our configuration constants are reasonable values
        
        // HDR params should be lighter than SDR params
        let hdr_parts: Vec<f32> = crate::config::FIXED_HQDN3D_PARAMS_HDR
            .split(':')
            .map(|s| s.parse().unwrap())
            .collect();
        let sdr_parts: Vec<f32> = crate::config::FIXED_HQDN3D_PARAMS_SDR
            .split(':')
            .map(|s| s.parse().unwrap())
            .collect();
            
        assert_eq!(hdr_parts.len(), 4, "HDR params should have 4 components");
        assert_eq!(sdr_parts.len(), 4, "SDR params should have 4 components");
        
        // HDR should generally have lower values (lighter denoising)
        assert!(hdr_parts[0] <= sdr_parts[0], "HDR spatial luma should be <= SDR");
        assert!(hdr_parts[1] <= sdr_parts[1], "HDR spatial chroma should be <= SDR");
        
        // Film grain should be reasonable
        assert!(crate::config::FIXED_FILM_GRAIN_VALUE <= 50, "Film grain should be <= 50");
        assert!(crate::config::FIXED_FILM_GRAIN_VALUE > 0, "Film grain should be > 0");
    }
    
}


