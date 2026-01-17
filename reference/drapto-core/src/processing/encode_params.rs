//! Helpers for building encoding parameters and related decisions.
//!
//! This module keeps the pure configuration logic isolated from the main
//! workflow so it can be tested and evolved independently.

use crate::config::{CoreConfig, HD_WIDTH_THRESHOLD, UHD_WIDTH_THRESHOLD};
use crate::external::ffmpeg::EncodeParams;
use crate::processing::video_properties::VideoProperties;
use std::path::Path;

/// Parameters for setting up encoding configuration
pub struct EncodingSetupParams<'a> {
    pub input_path: &'a Path,
    pub output_path: &'a Path,
    pub quality: u32,
    pub config: &'a CoreConfig,
    pub crop_filter_opt: Option<String>,
    pub audio_channels: Vec<u32>,
    pub audio_streams: Option<Vec<crate::external::AudioStreamInfo>>,
    pub duration_secs: f64,
    pub video_props: &'a VideoProperties,
}

/// Determine how many logical processors SVT-AV1 should use in responsive mode.
/// Returns (processors_for_encoder, processors_reserved).
pub fn plan_responsive_threads(total_logical: usize) -> Option<(u32, usize)> {
    if total_logical <= 1 {
        return None;
    }

    let mut reserve = if total_logical <= 8 { 2 } else { 4 };
    if reserve >= total_logical {
        reserve = total_logical.saturating_sub(1);
    }

    let usable = total_logical.saturating_sub(reserve);
    if usable == 0 {
        None
    } else {
        Some((usable as u32, reserve))
    }
}

/// Determines quality settings based on video resolution and config.
///
/// Returns (quality, category, is_hdr)
pub fn determine_quality_settings(
    video_props: &VideoProperties,
    config: &CoreConfig,
) -> (u32, &'static str, bool) {
    let video_width = video_props.width;

    // Select quality (CRF) based on video resolution
    let quality = if video_width >= UHD_WIDTH_THRESHOLD {
        // UHD (4K) quality setting
        config.quality_uhd
    } else if video_width >= HD_WIDTH_THRESHOLD {
        // HD (1080p) quality setting
        config.quality_hd
    } else {
        // SD (below 1080p) quality setting
        config.quality_sd
    };

    // Determine the category label for logging
    let category = if video_width >= UHD_WIDTH_THRESHOLD {
        "UHD"
    } else if video_width >= HD_WIDTH_THRESHOLD {
        "HD"
    } else {
        "SD"
    };

    // Detect HDR/SDR status using MediaInfo
    let is_hdr = video_props.hdr_info.is_hdr;

    (quality.into(), category, is_hdr)
}

/// Sets up encoding parameters from analysis results and config.
pub fn setup_encoding_parameters(params: EncodingSetupParams) -> EncodeParams {
    let preset_value = params.config.svt_av1_preset;
    let tune_value = params.config.svt_av1_tune;

    let mut initial_encode_params = EncodeParams {
        input_path: params.input_path.to_path_buf(),
        output_path: params.output_path.to_path_buf(),
        quality: params.quality,
        preset: preset_value,
        tune: tune_value,
        ac_bias: params.config.svt_av1_ac_bias,
        enable_variance_boost: params.config.svt_av1_enable_variance_boost,
        variance_boost_strength: params.config.svt_av1_variance_boost_strength,
        variance_octile: params.config.svt_av1_variance_octile,
        video_denoise_filter: params.config.video_denoise_filter.clone(),
        film_grain: params.config.svt_av1_film_grain,
        film_grain_denoise: params.config.svt_av1_film_grain_denoise,
        logical_processors: None,
        crop_filter: params.crop_filter_opt,
        audio_channels: params.audio_channels,
        audio_streams: params.audio_streams,
        duration: params.duration_secs,
        // Actual values that will be used in FFmpeg command
        video_codec: "libsvtav1".to_string(),
        pixel_format: "yuv420p10le".to_string(),
        matrix_coefficients: params
            .video_props
            .hdr_info
            .matrix_coefficients
            .clone()
            .unwrap_or_else(|| "bt709".to_string()),
        audio_codec: "libopus".to_string(),
    };

    let logical_processors = if params.config.responsive_encoding {
        let total_logical = num_cpus::get();
        match plan_responsive_threads(total_logical) {
            Some((lp, reserved)) => {
                log::info!(
                    "Responsive mode enabled: reserving {} of {} logical threads (SVT-AV1 using {})",
                    reserved,
                    total_logical,
                    lp
                );
                Some(lp)
            }
            None => {
                log::warn!(
                    "Responsive mode requested but system has insufficient logical processors ({}); using default threading",
                    total_logical
                );
                None
            }
        }
    } else {
        None
    };

    initial_encode_params.logical_processors = logical_processors;
    initial_encode_params
}

#[cfg(test)]
mod tests {
    use super::plan_responsive_threads;

    #[test]
    fn reserves_two_threads_for_medium_systems() {
        let plan = plan_responsive_threads(8).expect("Plan should exist for 8 logical threads");
        assert_eq!(plan.0, 6);
        assert_eq!(plan.1, 2);
    }

    #[test]
    fn reserves_four_threads_for_large_systems() {
        let plan = plan_responsive_threads(16).expect("Plan should exist for 16 logical threads");
        assert_eq!(plan.0, 12);
        assert_eq!(plan.1, 4);
    }

    #[test]
    fn scales_down_reserve_for_two_thread_systems() {
        let plan = plan_responsive_threads(2).expect("Plan should exist for 2 logical threads");
        assert_eq!(plan.0, 1);
        assert_eq!(plan.1, 1);
    }

    #[test]
    fn returns_none_when_insufficient_threads() {
        assert!(plan_responsive_threads(1).is_none());
        assert!(plan_responsive_threads(0).is_none());
    }
}
