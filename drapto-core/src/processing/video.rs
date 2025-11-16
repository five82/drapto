//! Main video encoding orchestration.
//!
//! This module coordinates the entire encoding workflow, from analyzing video
//! properties to executing ffmpeg and reporting results.
//!
//! # Workflow
//!
//! 1. Initialize processing and check hardware decoding
//! 2. For each video file:
//!    - Determine output path and check for existing files
//!    - Detect video properties (resolution, duration, etc.)
//!    - Select quality settings based on resolution
//!    - Perform crop detection if enabled
//!    - Analyze audio streams and determine bitrates
//!    - Apply denoising if enabled
//!    - Execute ffmpeg with the determined parameters
//!    - Handle results and send notifications

use crate::EncodeResult;
use crate::config::{CoreConfig, HD_WIDTH_THRESHOLD, UHD_WIDTH_THRESHOLD};
use crate::error::{CoreError, CoreResult};
use crate::events::{Event, EventDispatcher};
use crate::external::ffmpeg::{EncodeParams, run_ffmpeg_encode};
use crate::external::ffprobe_executor::get_media_info;
use crate::external::{AudioStreamInfo, get_file_size as external_get_file_size};
use crate::notifications::NotificationSender;
use crate::processing::audio;
use crate::processing::crop_detection;
use crate::processing::noise_analysis;
use crate::processing::validation::validate_output_video;
use crate::processing::video_properties::VideoProperties;
use crate::system_info::SystemInfo;
use crate::utils::{SafePath, calculate_size_reduction, resolve_output_path};

use log::warn;

/// Parameters for setting up encoding configuration
struct EncodingSetupParams<'a> {
    input_path: &'a std::path::Path,
    output_path: &'a std::path::Path,
    quality: u32,
    config: &'a CoreConfig,
    crop_filter_opt: Option<String>,
    audio_channels: Vec<u32>,
    audio_streams: Option<Vec<AudioStreamInfo>>,
    duration_secs: f64,
    video_props: &'a VideoProperties,
    noise_analysis: Option<&'a noise_analysis::NoiseAnalysis>,
}

/// Determine how many logical processors SVT-AV1 should use in responsive mode.
/// Returns (processors_for_encoder, processors_reserved).
fn plan_responsive_threads(total_logical: usize) -> Option<(u32, usize)> {
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

use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Helper function to safely send notifications with consistent error handling.
fn send_notification_safe(sender: Option<&dyn NotificationSender>, message: &str, context: &str) {
    if let Some(sender) = sender {
        if let Err(e) = sender.send(message) {
            warn!("Failed to send {} notification: {e}", context);
        }
    }
}

/// Helper function to send individual validation failure notifications
fn send_validation_failure_notifications(
    sender: Option<&dyn NotificationSender>,
    filename: &str,
    validation_steps: &[(String, bool, String)],
) {
    let failures: Vec<&(String, bool, String)> = validation_steps
        .iter()
        .filter(|(_, passed, _)| !passed)
        .collect();

    if failures.is_empty() {
        return;
    }

    // Send individual notifications for each failure
    for (step_name, _, message) in failures.iter() {
        let notification_msg = format!(
            "{}: {} validation failed - {}",
            filename, step_name, message
        );
        send_notification_safe(sender, &notification_msg, "validation_failure");
    }

    // Send summary notification if multiple failures
    if failures.len() > 1 {
        let summary_msg = format!(
            "{}: {} validation checks failed (encoding completed)",
            filename,
            failures.len()
        );
        send_notification_safe(sender, &summary_msg, "validation_summary");
    }
}

/// Helper function to emit events if event dispatcher is available
fn emit_event(event_dispatcher: Option<&EventDispatcher>, event: Event) {
    if let Some(dispatcher) = event_dispatcher {
        dispatcher.emit(event);
    }
}

/// Generate audio results description for encoding complete summary
fn generate_audio_results_description(
    audio_channels: &[u32],
    audio_streams: Option<&[crate::external::AudioStreamInfo]>,
) -> String {
    if audio_channels.is_empty() {
        return "No audio".to_string();
    }

    // If we don't have detailed stream info, fall back to basic Opus description
    let Some(streams) = audio_streams else {
        return generate_basic_audio_description(audio_channels);
    };

    if streams.len() == 1 {
        generate_single_stream_description(&streams[0])
    } else {
        generate_multi_stream_description(streams)
    }
}

#[cfg(test)]
mod responsive_thread_tests {
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

/// Generate basic audio description assuming Opus (fallback)
fn generate_basic_audio_description(audio_channels: &[u32]) -> String {
    if audio_channels.len() == 1 {
        let channel_desc = match audio_channels[0] {
            1 => "Mono".to_string(),
            2 => "Stereo".to_string(),
            6 => "5.1 surround".to_string(),
            8 => "7.1 surround".to_string(),
            n => format!("{} channels", n),
        };
        format!(
            "{}, Opus, {} kb/s",
            channel_desc,
            crate::processing::audio::calculate_audio_bitrate(audio_channels[0])
        )
    } else {
        let track_descriptions: Vec<String> = audio_channels
            .iter()
            .enumerate()
            .map(|(i, &channels)| {
                let bitrate = crate::processing::audio::calculate_audio_bitrate(channels);
                let desc = match channels {
                    1 => "Mono".to_string(),
                    2 => "Stereo".to_string(),
                    6 => "5.1 surround".to_string(),
                    8 => "7.1 surround".to_string(),
                    n => format!("{} channels", n),
                };
                format!("Track {}: {}, Opus, {} kb/s", i + 1, desc, bitrate)
            })
            .collect();
        track_descriptions.join("\n                     ")
    }
}

/// Generate description for a single audio stream
fn generate_single_stream_description(stream: &crate::external::AudioStreamInfo) -> String {
    let channel_desc = match stream.channels {
        1 => "Mono".to_string(),
        2 => "Stereo".to_string(),
        6 => "5.1 surround".to_string(),
        8 => "7.1 surround".to_string(),
        n => format!("{} channels", n),
    };

    if stream.is_spatial {
        format!(
            "{} ({}), Preserved",
            channel_desc,
            format_codec_with_profile(&stream.codec_name, stream.profile.as_deref())
        )
    } else {
        let bitrate = crate::processing::audio::calculate_audio_bitrate(stream.channels);
        format!("{}, Opus, {} kb/s", channel_desc, bitrate)
    }
}

/// Generate description for multiple audio streams
fn generate_multi_stream_description(streams: &[crate::external::AudioStreamInfo]) -> String {
    let track_descriptions: Vec<String> = streams
        .iter()
        .enumerate()
        .map(|(i, stream)| {
            let channel_desc = match stream.channels {
                1 => "Mono".to_string(),
                2 => "Stereo".to_string(),
                6 => "5.1 surround".to_string(),
                8 => "7.1 surround".to_string(),
                n => format!("{} channels", n),
            };

            if stream.is_spatial {
                format!(
                    "Track {}: {} ({}), Preserved",
                    i + 1,
                    channel_desc,
                    format_codec_with_profile(&stream.codec_name, stream.profile.as_deref())
                )
            } else {
                let bitrate = crate::processing::audio::calculate_audio_bitrate(stream.channels);
                format!("Track {}: {}, Opus, {} kb/s", i + 1, channel_desc, bitrate)
            }
        })
        .collect();
    track_descriptions.join("\n                     ")
}

/// Format codec name with profile for display
fn format_codec_with_profile(codec_name: &str, profile: Option<&str>) -> String {
    match profile {
        Some(profile) if !profile.is_empty() => {
            format!("{} {}", codec_name.to_uppercase(), profile)
        }
        _ => codec_name.to_uppercase(),
    }
}

/// Calculate final output dimensions after crop is applied
fn get_output_dimensions(
    original_width: u32,
    original_height: u32,
    crop_filter: Option<&str>,
) -> (u32, u32) {
    if let Some(crop) = crop_filter {
        // Parse crop filter format: crop=width:height:x:y
        if let Some(params) = crop.strip_prefix("crop=") {
            let parts: Vec<&str> = params.split(':').collect();
            if parts.len() >= 2 {
                if let (Ok(width), Ok(height)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>())
                {
                    return (width, height);
                }
            }
        }
    }
    // Return original dimensions if no crop or parsing fails
    (original_width, original_height)
}

/// Determines quality settings based on video resolution and config.
///
/// Returns (quality, category, is_hdr)
fn determine_quality_settings(
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
fn setup_encoding_parameters(params: EncodingSetupParams) -> EncodeParams {
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
        use_hw_decode: true,
        logical_processors: None,
        crop_filter: params.crop_filter_opt,
        audio_channels: params.audio_channels,
        audio_streams: params.audio_streams,
        duration: params.duration_secs,
        hqdn3d_params: None,
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
        film_grain_level: 0, // Will be set below
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

    // Apply denoising parameters if enabled
    let (final_hqdn3d_params, film_grain_level) = if params.config.enable_denoise {
        // Noise analysis is required when denoising is enabled - no fallback
        let noise_analysis = params
            .noise_analysis
            .expect("Noise analysis should be available when denoising is enabled");

        log::info!(
            "Using adaptive denoising: hqdn3d={}, film_grain={}",
            noise_analysis.recommended_hqdn3d,
            noise_analysis.recommended_film_grain
        );
        (
            Some(noise_analysis.recommended_hqdn3d.clone()),
            noise_analysis.recommended_film_grain,
        )
    } else {
        log::debug!("Denoising disabled via config.");
        (None, 0)
    };

    // Finalize encoding parameters
    initial_encode_params.hqdn3d_params = final_hqdn3d_params;
    initial_encode_params.film_grain_level = film_grain_level;
    initial_encode_params.logical_processors = logical_processors;
    initial_encode_params
}

/// Main entry point for video processing. Orchestrates analysis, encoding, and notifications.
pub fn process_videos(
    notification_sender: Option<&dyn NotificationSender>,
    config: &CoreConfig,
    files_to_process: &[PathBuf],
    target_filename_override: Option<PathBuf>,
    event_dispatcher: Option<&EventDispatcher>,
) -> CoreResult<Vec<EncodeResult>> {
    let mut results: Vec<EncodeResult> = Vec::new();

    // Emit hardware information at the very beginning
    let system_info = SystemInfo::collect();
    emit_event(
        event_dispatcher,
        Event::HardwareInfo {
            hostname: system_info.hostname,
            os: system_info.os,
            cpu: system_info.cpu,
            memory: system_info.memory,
            decoder: system_info.decoder,
        },
    );

    // Show batch initialization for multiple files
    if files_to_process.len() > 1 {
        emit_event(
            event_dispatcher,
            Event::BatchInitializationStarted {
                total_files: files_to_process.len(),
                file_list: files_to_process
                    .iter()
                    .filter_map(|p| p.file_name())
                    .filter_map(|n| n.to_str())
                    .map(|s| s.to_string())
                    .collect(),
                output_dir: config.output_dir.display().to_string(),
            },
        );
    }

    for (file_index, input_path) in files_to_process.iter().enumerate() {
        let file_start_time = Instant::now();

        // Show file progress context for multiple files
        if files_to_process.len() > 1 {
            emit_event(
                event_dispatcher,
                Event::FileProgressContext {
                    current_file: file_index + 1,
                    total_files: files_to_process.len(),
                },
            );
        }

        let input_filename = crate::utils::get_filename_safe(input_path)?;

        // Safely determine output path with validation
        let target_override = if files_to_process.len() == 1 {
            target_filename_override.as_deref()
        } else {
            None // Don't use override for batch processing
        };

        let output_path = resolve_output_path(input_path, &config.output_dir, target_override)?;

        // Skip processing if the output file already exists
        if output_path.exists() {
            let _output_filename =
                SafePath::get_filename_utf8(&output_path).unwrap_or_else(|_| "unknown".to_string());

            let message = format!(
                "Output file already exists: {}. Skipping encode.",
                output_path.display()
            );

            emit_event(
                event_dispatcher,
                Event::Warning {
                    message: message.clone(),
                },
            );

            send_notification_safe(
                notification_sender,
                &format!(
                    "Skipped encode for {}: Output file already exists at {}",
                    input_filename,
                    output_path.display()
                ),
                "skip",
            );

            continue;
        }

        // Send encoding start notification
        send_notification_safe(
            notification_sender,
            &format!("Started encoding {input_filename}"),
            "start",
        );

        // Analyze video properties
        let video_props = match crate::external::get_video_properties(input_path) {
            Ok(props) => props,
            Err(e) => {
                let error_msg = format!("Could not analyze {input_filename}: {e}");
                emit_event(
                    event_dispatcher,
                    Event::Error {
                        title: "Analysis Error".to_string(),
                        message: error_msg.clone(),
                        context: Some(format!("File: {}", input_path.display())),
                        suggestion: Some("Check if the file is a valid video format".to_string()),
                    },
                );

                send_notification_safe(
                    notification_sender,
                    &format!("Error encoding {input_filename}: Failed to get video properties"),
                    "error",
                );

                continue;
            }
        };

        let video_width = video_props.width;
        let video_height = video_props.height;
        let duration_secs = video_props.duration_secs;

        // Determine quality settings based on resolution
        let (quality, category, is_hdr) = determine_quality_settings(&video_props, config);

        // Get audio channels early for consolidated reporting
        let audio_channels = audio::get_audio_channels_quiet(input_path);

        // Get detailed audio stream info for spatial audio detection
        let audio_streams = audio::analyze_and_log_audio_detailed(input_path);

        // Format audio description - each track on its own line for multiple tracks
        let audio_description = if audio_channels.is_empty() {
            "No audio".to_string()
        } else if audio_channels.len() == 1 {
            match audio_channels[0] {
                1 => "Mono".to_string(),
                2 => "Stereo".to_string(),
                6 => "5.1 surround".to_string(),
                8 => "7.1 surround".to_string(),
                n => format!("{} channels", n),
            }
        } else {
            // Multiple audio tracks - format each on a new line
            let track_descriptions: Vec<String> = audio_channels
                .iter()
                .enumerate()
                .map(|(i, &channels)| {
                    let desc = match channels {
                        1 => "Mono".to_string(),
                        2 => "Stereo".to_string(),
                        6 => "5.1 surround".to_string(),
                        8 => "7.1 surround".to_string(),
                        n => format!("{} channels", n),
                    };
                    format!("Track {}: {}", i + 1, desc)
                })
                .collect();
            track_descriptions.join("\n                     ") // Indent continuation lines
        };

        // Emit initialization event
        emit_event(
            event_dispatcher,
            Event::InitializationStarted {
                input_file: input_filename.clone(),
                output_file: output_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
                duration: crate::utils::format_duration(duration_secs as f64),
                resolution: format!("{}x{}", video_width, video_height),
                category: category.to_string(),
                dynamic_range: if is_hdr {
                    "HDR".to_string()
                } else {
                    "SDR".to_string()
                },
                audio_description,
            },
        );

        // Perform video analysis (crop detection)
        emit_event(event_dispatcher, Event::VideoAnalysisStarted);

        let disable_crop = config.crop_mode == "none";
        let (crop_filter_opt, _is_hdr) = if disable_crop {
            // Crop detection is disabled - emit a special event to show this
            emit_event(
                event_dispatcher,
                Event::BlackBarDetectionComplete {
                    crop_required: false,
                    crop_params: Some("disabled".to_string()),
                },
            );
            (None, false)
        } else {
            // Crop detection is enabled - proceed normally
            emit_event(event_dispatcher, Event::BlackBarDetectionStarted);

            match crop_detection::detect_crop(
                input_path,
                &video_props,
                disable_crop,
                event_dispatcher,
            ) {
                Ok(result) => {
                    emit_event(
                        event_dispatcher,
                        Event::BlackBarDetectionComplete {
                            crop_required: result.0.is_some(),
                            crop_params: result.0.clone(),
                        },
                    );
                    result
                }
                Err(e) => {
                    let warning_msg = format!(
                        "Crop detection failed for {input_filename}: {e}. Proceeding without cropping."
                    );
                    emit_event(
                        event_dispatcher,
                        Event::Warning {
                            message: warning_msg,
                        },
                    );
                    emit_event(
                        event_dispatcher,
                        Event::BlackBarDetectionComplete {
                            crop_required: false,
                            crop_params: None,
                        },
                    );
                    (None, false)
                }
            }
        };

        // Audio channels already analyzed above

        // Perform noise analysis if denoising is enabled
        let noise_analysis_result = if config.enable_denoise {
            emit_event(event_dispatcher, Event::NoiseAnalysisStarted);
            match noise_analysis::analyze_noise(input_path, &video_props, event_dispatcher) {
                Ok(analysis) => {
                    emit_event(
                        event_dispatcher,
                        Event::NoiseAnalysisComplete {
                            average_noise: analysis.average_noise,
                            has_significant_noise: analysis.has_significant_noise,
                            recommended_params: analysis.recommended_hqdn3d.clone(),
                        },
                    );
                    Some(analysis)
                }
                Err(e) => {
                    let error_msg = format!("Noise analysis failed for {input_filename}: {e}");
                    emit_event(
                        event_dispatcher,
                        Event::Error {
                            title: "Noise Analysis Failed".to_string(),
                            message: error_msg.clone(),
                            context: Some(format!("File: {}", input_path.display())),
                            suggestion: Some(
                                "Check if the video file is valid and accessible".to_string(),
                            ),
                        },
                    );

                    send_notification_safe(
                        notification_sender,
                        &format!("Error encoding {input_filename}: Noise analysis failed"),
                        "error",
                    );

                    continue;
                }
            }
        } else {
            None
        };

        // Setup encoding parameters
        let final_encode_params = setup_encoding_parameters(EncodingSetupParams {
            input_path,
            output_path: &output_path,
            quality,
            config,
            crop_filter_opt,
            audio_channels: audio_channels.clone(),
            audio_streams: audio_streams.clone(),
            duration_secs,
            video_props: &video_props,
            noise_analysis: noise_analysis_result.as_ref(),
        });

        // Format audio description for the config display
        let audio_description = if let Some(ref streams) = audio_streams {
            if streams.is_empty() {
                "No audio".to_string()
            } else if streams.len() == 1 {
                let stream = &streams[0];
                let channel_desc = match stream.channels {
                    1 => "Mono".to_string(),
                    2 => "Stereo".to_string(),
                    6 => "5.1".to_string(),
                    8 => "7.1".to_string(),
                    n => format!("{} channels", n),
                };
                if stream.is_spatial {
                    format!(
                        "{} ({} {}) - Preserved",
                        channel_desc,
                        stream.codec_name,
                        stream.profile.as_deref().unwrap_or("")
                    )
                } else {
                    let bitrate =
                        crate::processing::audio::calculate_audio_bitrate(stream.channels);
                    format!("{} @ {}kbps Opus", channel_desc, bitrate)
                }
            } else {
                // Multiple audio tracks - show each on its own line
                let track_descriptions: Vec<String> = streams
                    .iter()
                    .map(|stream| {
                        let desc = match stream.channels {
                            1 => "Mono".to_string(),
                            2 => "Stereo".to_string(),
                            6 => "5.1".to_string(),
                            8 => "7.1".to_string(),
                            n => format!("{} channels", n),
                        };
                        if stream.is_spatial {
                            format!(
                                "Track {}: {} ({} {}) - Preserved",
                                stream.index + 1,
                                desc,
                                stream.codec_name,
                                stream.profile.as_deref().unwrap_or("")
                            )
                        } else {
                            let bitrate =
                                crate::processing::audio::calculate_audio_bitrate(stream.channels);
                            format!(
                                "Track {}: {} @ {}kbps Opus",
                                stream.index + 1,
                                desc,
                                bitrate
                            )
                        }
                    })
                    .collect();
                track_descriptions.join("\n                     ") // Indent continuation lines
            }
        } else {
            // Fallback to simple channel info if detailed analysis failed
            if audio_channels.is_empty() {
                "No audio".to_string()
            } else if audio_channels.len() == 1 {
                let bitrate = crate::processing::audio::calculate_audio_bitrate(audio_channels[0]);
                let channel_desc = match audio_channels[0] {
                    1 => "Mono".to_string(),
                    2 => "Stereo".to_string(),
                    6 => "5.1".to_string(),
                    8 => "7.1".to_string(),
                    n => format!("{} channels", n),
                };
                format!("{} @ {}kbps", channel_desc, bitrate)
            } else {
                let track_descriptions: Vec<String> = audio_channels
                    .iter()
                    .enumerate()
                    .map(|(i, &channels)| {
                        let bitrate = crate::processing::audio::calculate_audio_bitrate(channels);
                        let desc = match channels {
                            1 => "Mono".to_string(),
                            2 => "Stereo".to_string(),
                            6 => "5.1".to_string(),
                            8 => "7.1".to_string(),
                            n => format!("{} channels", n),
                        };
                        format!("Track {}: {} @ {}kbps", i + 1, desc, bitrate)
                    })
                    .collect();
                track_descriptions.join("\n                     ")
            }
        };

        // Format film grain display
        let film_grain_display = if final_encode_params.film_grain_level > 0 {
            format!("Level {}", final_encode_params.film_grain_level)
        } else {
            "None".to_string()
        };

        // Convert video codec to display name
        let encoder_display = match final_encode_params.video_codec.as_str() {
            "libsvtav1" => "SVT-AV1",
            other => other,
        };

        // Convert audio codec to display name - handle mixed spatial/non-spatial tracks
        let audio_codec_display = if let Some(ref streams) = audio_streams {
            let spatial_count = streams.iter().filter(|s| s.is_spatial).count();
            let non_spatial_count = streams.len() - spatial_count;

            match (spatial_count, non_spatial_count) {
                (0, _) => "Opus".to_string(), // All non-spatial
                (_, 0) => "Copy (Spatial Audio Preserved)".to_string(), // All spatial
                (_, _) => "Mixed (Spatial + Opus)".to_string(), // Mixed
            }
        } else {
            // Fallback when no detailed stream info
            match final_encode_params.audio_codec.as_str() {
                "libopus" => "Opus",
                other => other,
            }
            .to_string()
        };

        // Emit encoding configuration event
        emit_event(
            event_dispatcher,
            Event::EncodingConfigurationDisplayed {
                encoder: encoder_display.to_string(),
                preset: final_encode_params.preset.to_string(),
                tune: final_encode_params.tune.to_string(),
                quality: format!("CRF {}", final_encode_params.quality),
                denoising: final_encode_params
                    .hqdn3d_params
                    .as_ref()
                    .map(|p| format!("hqdn3d={}", p))
                    .unwrap_or_else(|| "None".to_string()),
                film_grain: film_grain_display,
                hardware_accel: None, // Hardware info is already shown in VIDEO DETAILS section
                pixel_format: final_encode_params.pixel_format.clone(),
                matrix_coefficients: final_encode_params.matrix_coefficients.clone(),
                audio_codec: audio_codec_display.to_string(),
                audio_description,
            },
        );

        // Get total frame count for progress reporting
        let total_frames = match get_media_info(input_path) {
            Ok(info) => info.total_frames.unwrap_or(0),
            Err(e) => {
                log::warn!("Failed to get media info for frame count: {}", e);
                0
            }
        };

        // Emit encoding started event
        emit_event(event_dispatcher, Event::EncodingStarted { total_frames });

        let encode_result = run_ffmpeg_encode(
            &final_encode_params,
            false, // disable_audio: Keep audio in the output
            final_encode_params.hqdn3d_params.is_some(), // has_denoising: Whether denoising is applied
            total_frames,
            event_dispatcher,
        );

        // Handle encoding results

        match encode_result {
            Ok(()) => {
                let file_elapsed_time = file_start_time.elapsed();

                let input_size = external_get_file_size(input_path)?;
                let output_size = external_get_file_size(&output_path)?;
                let encoding_speed = duration_secs as f32 / file_elapsed_time.as_secs_f32();

                // Calculate expected dimensions after crop for validation
                let expected_dimensions =
                    if let Some(ref crop_filter) = final_encode_params.crop_filter {
                        Some(get_output_dimensions(
                            video_width,
                            video_height,
                            Some(crop_filter),
                        ))
                    } else {
                        Some((video_width, video_height))
                    };

                // Emit validation start event
                emit_event(
                    event_dispatcher,
                    Event::StageProgress {
                        stage: "validation".to_string(),
                        percent: 0.0,
                        message: "Starting output validation".to_string(),
                        eta: None,
                    },
                );

                // Perform post-encode validation
                let expected_audio_track_count = if audio_channels.is_empty() {
                    Some(0)
                } else {
                    Some(audio_channels.len())
                };

                // Extract spatial audio information for validation
                let spatial_audio_flags: Option<Vec<bool>> = audio_streams
                    .as_ref()
                    .map(|streams| streams.iter().map(|stream| stream.is_spatial).collect());

                let (validation_passed, validation_steps) = match validate_output_video(
                    input_path,
                    &output_path,
                    expected_dimensions,
                    Some(duration_secs),
                    Some(is_hdr),
                    expected_audio_track_count,
                    spatial_audio_flags.as_deref(),
                ) {
                    Ok(validation_result) => {
                        let steps = validation_result.get_validation_steps();

                        let validation_passed = validation_result.is_valid();

                        if !validation_passed {
                            let failures = validation_result.get_failures();
                            log::warn!(
                                "Post-encode validation failed for {}: {} (continuing processing)",
                                input_filename,
                                failures.join(", ")
                            );

                            // Send detailed individual notifications for each failure
                            send_validation_failure_notifications(
                                notification_sender,
                                &input_filename,
                                &steps,
                            );
                        } else {
                            log::debug!(
                                "Post-encode validation passed for {}: All validation checks confirmed",
                                input_filename
                            );
                        }

                        (validation_passed, steps)
                    }
                    Err(validation_error) => {
                        // Continue processing even if validation fails due to technical issues
                        let error_steps = vec![
                            (
                                "Video codec".to_string(),
                                false,
                                "Validation error".to_string(),
                            ),
                            (
                                "Bit depth".to_string(),
                                false,
                                "Validation error".to_string(),
                            ),
                            (
                                "Crop detection".to_string(),
                                false,
                                "Validation error".to_string(),
                            ),
                            (
                                "Video duration".to_string(),
                                false,
                                "Validation error".to_string(),
                            ),
                            (
                                "HDR/SDR status".to_string(),
                                false,
                                "Validation error".to_string(),
                            ),
                            (
                                "Audio tracks".to_string(),
                                false,
                                "Validation error".to_string(),
                            ),
                            (
                                "Audio/video sync".to_string(),
                                false,
                                "Validation error".to_string(),
                            ),
                        ];

                        log::warn!(
                            "Post-encode validation error for {}: {} (continuing processing)",
                            input_filename,
                            validation_error
                        );

                        (false, error_steps)
                    }
                };

                results.push(EncodeResult {
                    filename: input_filename.clone(),
                    duration: file_elapsed_time,
                    input_size,
                    output_size,
                    video_duration_secs: duration_secs,
                    encoding_speed,
                    validation_passed,
                    validation_steps: validation_steps.clone(),
                });

                // Emit validation stage completion event
                emit_event(
                    event_dispatcher,
                    Event::StageProgress {
                        stage: "validation".to_string(),
                        percent: 100.0,
                        message: if validation_passed {
                            "Validation completed successfully".to_string()
                        } else {
                            "Validation completed with issues".to_string()
                        },
                        eta: None,
                    },
                );

                // Emit validation complete event
                emit_event(
                    event_dispatcher,
                    Event::ValidationComplete {
                        validation_passed,
                        validation_steps: validation_steps.clone(),
                    },
                );

                // Calculate final output dimensions (considering crop)
                let (final_width, final_height) = get_output_dimensions(
                    video_width,
                    video_height,
                    final_encode_params.crop_filter.as_deref(),
                );

                // Emit encoding complete event
                emit_event(
                    event_dispatcher,
                    Event::EncodingComplete {
                        input_file: input_filename.clone(),
                        output_file: output_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        original_size: input_size,
                        encoded_size: output_size,
                        video_stream: format!("AV1 (libsvtav1), {}x{}", final_width, final_height),
                        audio_stream: generate_audio_results_description(
                            &audio_channels,
                            audio_streams.as_deref(),
                        ),
                        total_time: file_elapsed_time,
                        average_speed: duration_secs as f32 / file_elapsed_time.as_secs_f32(),
                        output_path: output_path.display().to_string(),
                    },
                );

                // Send success notification
                let reduction = calculate_size_reduction(input_size, output_size) as u32;

                let duration_secs = file_elapsed_time.as_secs();
                let duration_str = if duration_secs >= 3600 {
                    format!(
                        "{}h {}m {}s",
                        duration_secs / 3600,
                        (duration_secs % 3600) / 60,
                        duration_secs % 60
                    )
                } else if duration_secs >= 60 {
                    format!("{}m {}s", duration_secs / 60, duration_secs % 60)
                } else {
                    format!("{duration_secs}s")
                };

                send_notification_safe(
                    notification_sender,
                    &format!(
                        "Completed encoding {input_filename} in {duration_str}. Reduced by {reduction}%"
                    ),
                    "success",
                );
            }

            Err(e) => {
                if let CoreError::NoStreamsFound(path) = &e {
                    let warning_msg = format!(
                        "Skipping encode for {input_filename}: FFmpeg reported no processable streams found in '{path}'."
                    );
                    emit_event(
                        event_dispatcher,
                        Event::Warning {
                            message: warning_msg,
                        },
                    );

                    send_notification_safe(
                        notification_sender,
                        &format!("Skipped encode for {input_filename}: No streams found."),
                        "skip",
                    );
                } else {
                    let error_msg = format!("FFmpeg failed to encode {input_filename}: {e}");
                    emit_event(
                        event_dispatcher,
                        Event::Error {
                            title: "Encoding Error".to_string(),
                            message: error_msg.clone(),
                            context: Some(format!("File: {}", input_path.display())),
                            suggestion: Some("Check FFmpeg logs for more details".to_string()),
                        },
                    );

                    send_notification_safe(
                        notification_sender,
                        &format!("Error encoding {input_filename}: ffmpeg failed: {e}"),
                        "error",
                    );
                }
            }
        }

        // Apply cooldown between encodes when processing multiple files
        // This helps ensure notifications arrive in order
        if files_to_process.len() > 1
            && input_path != files_to_process.last().unwrap()
            && config.encode_cooldown_secs > 0
        {
            std::thread::sleep(std::time::Duration::from_secs(config.encode_cooldown_secs));
        }
    }

    // Generate appropriate summary based on number of files processed
    match results.len() {
        0 => {
            emit_event(
                event_dispatcher,
                Event::Warning {
                    message: "No files were successfully encoded".to_string(),
                },
            );
        }
        1 => {
            // Single file - keep existing behavior
            emit_event(
                event_dispatcher,
                Event::OperationComplete {
                    message: format!("Successfully encoded {}", results[0].filename),
                },
            );
        }
        _ => {
            // Multiple files - emit detailed batch summary
            let total_duration: Duration = results.iter().map(|r| r.duration).sum();
            let total_original_size: u64 = results.iter().map(|r| r.input_size).sum();
            let total_encoded_size: u64 = results.iter().map(|r| r.output_size).sum();

            // Calculate average speed across all files
            let total_video_duration_secs: f64 =
                results.iter().map(|r| r.video_duration_secs).sum();
            let average_speed = if total_duration.as_secs_f64() > 0.0 {
                (total_video_duration_secs / total_duration.as_secs_f64()) as f32
            } else {
                0.0f32
            };

            // Build per-file results for the summary
            let file_results: Vec<(String, f64)> = results
                .iter()
                .map(|r| {
                    let reduction = calculate_size_reduction(r.input_size, r.output_size) as f64;
                    (r.filename.clone(), reduction)
                })
                .collect();

            // Count validation results
            let validation_passed_count = results.iter().filter(|r| r.validation_passed).count();
            let validation_failed_count = results.len() - validation_passed_count;

            emit_event(
                event_dispatcher,
                Event::BatchComplete {
                    successful_count: results.len(),
                    total_files: files_to_process.len(),
                    total_original_size,
                    total_encoded_size,
                    total_duration,
                    average_speed,
                    file_results,
                    validation_passed_count,
                    validation_failed_count,
                },
            );
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external::AudioStreamInfo;

    #[test]
    fn test_single_spatial_audio_description() {
        let streams = vec![AudioStreamInfo {
            channels: 8,
            codec_name: "truehd".to_string(),
            profile: Some("Dolby TrueHD + Dolby Atmos".to_string()),
            index: 0,
            is_spatial: true,
        }];
        let audio_channels = vec![8];

        let result = generate_audio_results_description(&audio_channels, Some(&streams));

        assert_eq!(
            result,
            "7.1 surround (TRUEHD Dolby TrueHD + Dolby Atmos), Preserved"
        );
    }

    #[test]
    fn test_single_non_spatial_audio_description() {
        let streams = vec![AudioStreamInfo {
            channels: 2,
            codec_name: "aac".to_string(),
            profile: Some("LC".to_string()),
            index: 0,
            is_spatial: false,
        }];
        let audio_channels = vec![2];

        let result = generate_audio_results_description(&audio_channels, Some(&streams));

        assert_eq!(result, "Stereo, Opus, 128 kb/s");
    }

    #[test]
    fn test_multiple_audio_tracks_mixed() {
        let streams = vec![
            AudioStreamInfo {
                channels: 8,
                codec_name: "truehd".to_string(),
                profile: Some("Dolby TrueHD + Dolby Atmos".to_string()),
                index: 0,
                is_spatial: true,
            },
            AudioStreamInfo {
                channels: 2,
                codec_name: "aac".to_string(),
                profile: Some("LC".to_string()),
                index: 1,
                is_spatial: false,
            },
            AudioStreamInfo {
                channels: 2,
                codec_name: "ac3".to_string(),
                profile: Some("Dolby Digital".to_string()),
                index: 2,
                is_spatial: false,
            },
        ];
        let audio_channels = vec![8, 2, 2];

        let result = generate_audio_results_description(&audio_channels, Some(&streams));

        let expected = "Track 1: 7.1 surround (TRUEHD Dolby TrueHD + Dolby Atmos), Preserved\n                     Track 2: Stereo, Opus, 128 kb/s\n                     Track 3: Stereo, Opus, 128 kb/s";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_multiple_spatial_audio_tracks() {
        let streams = vec![
            AudioStreamInfo {
                channels: 8,
                codec_name: "truehd".to_string(),
                profile: Some("Dolby TrueHD + Dolby Atmos".to_string()),
                index: 0,
                is_spatial: true,
            },
            AudioStreamInfo {
                channels: 8,
                codec_name: "dts".to_string(),
                profile: Some("DTS:X".to_string()),
                index: 1,
                is_spatial: true,
            },
        ];
        let audio_channels = vec![8, 8];

        let result = generate_audio_results_description(&audio_channels, Some(&streams));

        let expected = "Track 1: 7.1 surround (TRUEHD Dolby TrueHD + Dolby Atmos), Preserved\n                     Track 2: 7.1 surround (DTS DTS:X), Preserved";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_multiple_non_spatial_audio_tracks() {
        let streams = vec![
            AudioStreamInfo {
                channels: 6,
                codec_name: "ac3".to_string(),
                profile: Some("Dolby Digital".to_string()),
                index: 0,
                is_spatial: false,
            },
            AudioStreamInfo {
                channels: 2,
                codec_name: "aac".to_string(),
                profile: Some("LC".to_string()),
                index: 1,
                is_spatial: false,
            },
        ];
        let audio_channels = vec![6, 2];

        let result = generate_audio_results_description(&audio_channels, Some(&streams));

        let expected = "Track 1: 5.1 surround, Opus, 256 kb/s\n                     Track 2: Stereo, Opus, 128 kb/s";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_fallback_without_stream_info() {
        let audio_channels = vec![8, 2];

        let result = generate_audio_results_description(&audio_channels, None);

        let expected = "Track 1: 7.1 surround, Opus, 384 kb/s\n                     Track 2: Stereo, Opus, 128 kb/s";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_no_audio_tracks() {
        let audio_channels = vec![];

        let result = generate_audio_results_description(&audio_channels, Some(&[]));

        assert_eq!(result, "No audio");
    }

    #[test]
    fn test_codec_profile_formatting() {
        // Test codec with profile
        let result = format_codec_with_profile("truehd", Some("Dolby TrueHD + Dolby Atmos"));
        assert_eq!(result, "TRUEHD Dolby TrueHD + Dolby Atmos");

        // Test codec without profile
        let result = format_codec_with_profile("dts", None);
        assert_eq!(result, "DTS");

        // Test codec with empty profile
        let result = format_codec_with_profile("truehd", Some(""));
        assert_eq!(result, "TRUEHD");
    }

    #[test]
    fn test_uncommon_channel_configurations() {
        let streams = vec![
            AudioStreamInfo {
                channels: 1,
                codec_name: "aac".to_string(),
                profile: Some("LC".to_string()),
                index: 0,
                is_spatial: false,
            },
            AudioStreamInfo {
                channels: 4,
                codec_name: "ac3".to_string(),
                profile: Some("Dolby Digital".to_string()),
                index: 1,
                is_spatial: false,
            },
            AudioStreamInfo {
                channels: 10,
                codec_name: "dtshd".to_string(),
                profile: Some("DTS-HD Master Audio".to_string()),
                index: 2,
                is_spatial: true,
            },
        ];
        let audio_channels = vec![1, 4, 10];

        let result = generate_audio_results_description(&audio_channels, Some(&streams));

        let expected = "Track 1: Mono, Opus, 64 kb/s\n                     Track 2: 4 channels, Opus, 192 kb/s\n                     Track 3: 10 channels (DTSHD DTS-HD Master Audio), Preserved";
        assert_eq!(result, expected);
    }
}
