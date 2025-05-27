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
//!    - Perform grain analysis if denoising is enabled
//!    - Execute ffmpeg with the determined parameters
//!    - Handle results and send notifications


use crate::config::CoreConfig;
use crate::error::{CoreError, CoreResult};
use crate::external::ffmpeg::{EncodeParams, run_ffmpeg_encode};
use crate::external::get_file_size as external_get_file_size;
use crate::notifications::{NotificationType, NtfyNotificationSender};
use crate::processing::audio;
use crate::processing::{crop_detection, grain_analysis};
use crate::EncodeResult;
use crate::utils::format_duration;

use log::{error, info, warn};

use std::path::PathBuf;
use std::time::Instant;

/// Processes a list of video files according to the provided configuration.
///
/// This is the main entry point for the drapto-core library. It orchestrates
/// the entire encoding workflow, from analyzing video properties to executing
/// ffmpeg and reporting results.
///
/// # Arguments
///
/// * `notification_sender` - Implementation of `NotificationSender` for sending notifications
/// * `config` - Core configuration containing encoding parameters and paths
/// * `files_to_process` - List of paths to the video files to process
/// * `target_filename_override` - Optional override for the output filename
///
/// # Returns
///
/// * `Ok(Vec<EncodeResult>)` - A vector of results for successfully processed files
/// * `Err(CoreError)` - If a critical error occurs during processing
///
/// # Examples
///
/// ```rust,no_run
/// use drapto_core::{CoreConfig, process_videos, EncodeResult};
/// use drapto_core::notifications::NtfyNotificationSender;
/// use drapto_core::processing::grain_types::GrainLevel;
/// use std::path::PathBuf;
///
/// // Create dependencies
/// let notification_sender = NtfyNotificationSender::new("https://ntfy.sh/my-topic").unwrap();
///
/// // Create configuration using the builder pattern
/// let config = drapto_core::config::CoreConfigBuilder::new()
///     .input_dir(PathBuf::from("/path/to/input"))
///     .output_dir(PathBuf::from("/path/to/output"))
///     .log_dir(PathBuf::from("/path/to/logs"))
///     .enable_denoise(true)
///     .encoder_preset(6)
///     .quality_sd(24)
///     .quality_hd(26)
///     .quality_uhd(28)
///     .crop_mode("auto")
///     .ntfy_topic("https://ntfy.sh/my-topic")
///     .film_grain_sample_duration(5)
///     .film_grain_knee_threshold(0.8)
///     .film_grain_max_level(GrainLevel::Moderate)
///     .film_grain_refinement_points_count(5)
///     .build();
///
/// // Find files to process
/// let files = vec![PathBuf::from("/path/to/video.mkv")];
///
/// // Process videos
/// match process_videos(
///     Some(&notification_sender),
///     &config,
///     &files,
///     None,
/// ) {
///     Ok(results) => {
///         println!("Successfully processed {} files", results.len());
///         for result in results {
///             println!("File: {}, Duration: {}", result.filename, result.duration.as_secs());
///         }
///     },
///     Err(e) => {
///         eprintln!("Error processing videos: {}", e);
///     }
/// }
/// ```
pub fn process_videos(
    notification_sender: Option<&NtfyNotificationSender>,
    config: &CoreConfig,
    files_to_process: &[PathBuf],
    target_filename_override: Option<PathBuf>,
) -> CoreResult<Vec<EncodeResult>> {
    let mut results: Vec<EncodeResult> = Vec::new();

    for input_path in files_to_process {
        let file_start_time = Instant::now();

        let filename = input_path
            .file_name()
            .ok_or_else(|| {
                CoreError::PathError(format!(
                    "Failed to get filename for {}",
                    input_path.display()
                ))
            })?
            .to_string_lossy()
            .to_string();

        let _filename_noext = input_path
            .file_stem()
            .ok_or_else(|| {
                CoreError::PathError(format!(
                    "Failed to get filename stem for {}",
                    input_path.display()
                ))
            })?
            .to_string_lossy()
            .to_string();

        // Determine output path based on configuration and target filename override
        let output_path = match &target_filename_override {
            Some(target_filename) if files_to_process.len() == 1 => {
                config.output_dir.join(target_filename)
            }
            _ => config.output_dir.join(&filename),
        };

        // Skip processing if the output file already exists
        if output_path.exists() {
            // Log the error with details - always shown regardless of verbosity
            let error_msg = format!(
                "Output file already exists: {}. Skipping encode.",
                output_path.display()
            );
            error!("{error_msg}");

            // Send a notification if notification_sender is provided
            if let Some(sender) = notification_sender {
                let notification = NotificationType::Custom {
                    title: "Drapto Encode Skipped".to_string(),
                    message: format!(
                        "Skipped encode for {}: Output file already exists at {}",
                        filename,
                        output_path.display()
                    ),
                    priority: 3,
                };

                if let Err(e) = sender.send_notification(&notification) {
                    warn!("Failed to send notification for {filename}: {e}");
                }
            }

            continue;
        }

        crate::progress_reporting::status("File", &filename, false);

        // Send encoding start notification
        if let Some(sender) = notification_sender {
            let notification = NotificationType::EncodeStart {
                input_path: input_path.clone(),
                output_path: output_path.clone(),
            };

            if let Err(e) = sender.send_notification(&notification) {
                warn!("Failed to send start notification for {filename}: {e}");
            }
        }

        // Analyze video properties
        let video_props = match crate::external::get_video_properties(input_path) {
            Ok(props) => props,
            Err(e) => {
                // Log the error and skip this file
                error!(
                    "Failed to get video properties for {filename}: {e}. Skipping file."
                );

                // Send an error notification if notification_sender is provided
                if let Some(sender) = notification_sender {
                    let notification = NotificationType::EncodeError {
                        input_path: input_path.clone(),
                        message: "Failed to get video properties".to_string(),
                    };

                    if let Err(e) = sender.send_notification(&notification) {
                        warn!("Failed to send error notification for {filename}: {e}");
                    }
                }

                crate::progress_reporting::info("");
                continue;
            }
        };

        let video_width = video_props.width;
        let duration_secs = video_props.duration_secs;

        // Determine quality settings based on resolution
        const UHD_WIDTH_THRESHOLD: u32 = 3840;
        const HD_WIDTH_THRESHOLD: u32 = 1920;

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

        // Report the detected resolution and selected quality as a status line
        crate::progress_reporting::status(
            "Video quality",
            &format!("{video_width} ({category}) - CRF {quality}"),
            false,
        );
        crate::progress_reporting::status("Duration", &format!("{duration_secs:.2}s"), false);

        // Detect and report HDR/SDR status based on color space
        let color_space = video_props.color_space.as_deref().unwrap_or("");
        let is_hdr = color_space == "bt2020nc" || color_space == "bt2020c";
        let dynamic_range = if is_hdr { "HDR" } else { "SDR" };
        crate::progress_reporting::status("Dynamic range", dynamic_range, false);

        // Perform crop detection
        crate::progress_reporting::processing("Detecting black bars");

        let disable_crop = config.crop_mode == "off";
        let (crop_filter_opt, _is_hdr) =
            match crop_detection::detect_crop(input_path, &video_props, disable_crop) {
                Ok(result) => result,
                Err(e) => {
                    // Log warning and proceed without cropping
                    warn!(
                        "Crop detection failed for {filename}: {e}. Proceeding without cropping."
                    );
                    // detect_crop returns Ok(None) for failures
                    (None, false)
                }
            };

        // Analyze audio streams
        crate::progress_reporting::processing("Audio analysis");

        // Log information about audio streams (channels, bitrates)
        let _ = audio::log_audio_info(input_path);

        // Get audio channel information for encoding
        let audio_channels = match crate::external::get_audio_channels(input_path) {
            Ok(channels) => channels,
            Err(e) => {
                // Log warning and continue with empty channel list
                warn!(
                    "Error getting audio channels for ffmpeg command build: {e}. Using empty list."
                );
                vec![] // Empty vector as fallback
            }
        };

        // Prepare encoding parameters
        let preset_value = config.encoder_preset;

        let mut initial_encode_params = EncodeParams {
            input_path: input_path.clone(),
            output_path: output_path.clone(),
            quality: quality.into(),
            preset: preset_value,
            use_hw_decode: true,
            crop_filter: crop_filter_opt.clone(),
            audio_channels: audio_channels.clone(),
            duration: duration_secs,
            hqdn3d_params: None,
        };

        // Perform grain analysis to determine denoising parameters
        let final_hqdn3d_params_result = if config.enable_denoise {
            grain_analysis::analyze_grain(input_path, config, &initial_encode_params, duration_secs)
        } else {
            // Skip grain analysis if denoising is disabled
            info!("Denoising disabled via config.");
            Ok(None)
        };

        // Process grain analysis results
        let (final_grain_level, final_hqdn3d_params): (
            Option<grain_analysis::GrainLevel>,
            Option<String>,
        ) = match final_hqdn3d_params_result {
            // Case 1: Grain analysis completed successfully with a result
            Ok(Some(result)) => {
                // Grain level is already reported by the grain analysis module
                // Return both the level and parameters (or None for Baseline videos)
                let params = grain_analysis::determine_hqdn3d_params(result.detected_level);
                (Some(result.detected_level), params)
            }
            // Case 2: Grain analysis was skipped or produced no result
            Ok(None) => {
                info!(
                    "Grain analysis skipped or did not produce a result. Proceeding without denoising."
                );
                (None, None)
            }
            // Case 3: Grain analysis failed with an error
            Err(e) => {
                // Log the error and skip this file
                error!("Grain analysis failed critically: {e}. Skipping file.");

                // Send an error notification if notification_sender is provided
                if let Some(sender) = notification_sender {
                    let notification = NotificationType::EncodeError {
                        input_path: input_path.clone(),
                        message: "Grain analysis failed".to_string(),
                    };

                    if let Err(e) = sender.send_notification(&notification) {
                        warn!("Failed to send error notification for {filename}: {e}");
                    }
                }

                crate::progress_reporting::info("");
                continue;
            }
        };

        // Finalize encoding parameters
        initial_encode_params.hqdn3d_params = final_hqdn3d_params.clone();
        let final_encode_params = initial_encode_params;

        log::debug!("ENCODING CONFIGURATION");
        log::debug!("Video:");
        log::debug!("Preset: {preset_value} (SVT-AV1)");
        log::debug!("Quality: {quality} (CRF)");

        match (final_grain_level, &final_hqdn3d_params) {
            (Some(level), Some(hqdn3d)) => {
                let grain_level_str = match level {
                    grain_analysis::GrainLevel::Baseline => "Baseline",
                    grain_analysis::GrainLevel::VeryLight => "VeryLight",
                    grain_analysis::GrainLevel::Light => "Light",
                    grain_analysis::GrainLevel::LightModerate => "LightModerate",
                    grain_analysis::GrainLevel::Moderate => "Moderate",
                    grain_analysis::GrainLevel::Elevated => "Elevated",
                };
                log::debug!("Grain Level: {grain_level_str} ({hqdn3d})");
            }
            _ => {
                log::debug!("Grain Level: None (no denoising)");
            }
        }

        // Hardware info
        log::debug!("Hardware:");
        let hw_info = crate::hardware_decode::get_hardware_decoding_info();
        let hw_display = match hw_info {
            Some(info) => format!("{info} (decode only)"),
            None => "None available".to_string(),
        };
        log::debug!("Acceleration: {hw_display}");

        crate::progress_reporting::section("ENCODING CONFIGURATION");
        // Video settings
        crate::progress_reporting::info("Video:");
        crate::progress_reporting::status("Preset", &format!("{preset_value} (SVT-AV1)"), false);
        crate::progress_reporting::status("Quality", &format!("{quality} (CRF)"), false);

        match (final_grain_level, &final_hqdn3d_params) {
            (Some(level), Some(hqdn3d)) => {
                let grain_level_str = match level {
                    grain_analysis::GrainLevel::Baseline => "Baseline",
                    grain_analysis::GrainLevel::VeryLight => "VeryLight",
                    grain_analysis::GrainLevel::Light => "Light",
                    grain_analysis::GrainLevel::LightModerate => "LightModerate",
                    grain_analysis::GrainLevel::Moderate => "Moderate",
                    grain_analysis::GrainLevel::Elevated => "Elevated",
                };
                crate::progress_reporting::status(
                    "Grain Level",
                    &format!("{grain_level_str} ({hqdn3d})"),
                    false,
                );
            }
            _ => {
                crate::progress_reporting::status("Grain Level", "None (no denoising)", false);
            }
        }

        // Hardware info
        crate::progress_reporting::info("Hardware:");
        let hw_info = crate::hardware_decode::get_hardware_decoding_info();
        let hw_display = match hw_info {
            Some(info) => format!("{info} (decode only)"),
            None => "No hardware decoder available".to_string(),
        };
        crate::progress_reporting::status("Acceleration", &hw_display, false);

        let encode_result = run_ffmpeg_encode(
            &final_encode_params,
            false, // disable_audio: Keep audio in the output
            false, // is_grain_analysis_sample: This is the main encode, not a sample
            final_grain_level,  // grain_level: Pass the detected grain level for film grain synthesis
        );

        // Handle encoding results

        match encode_result {
            Ok(()) => {
                let file_elapsed_time = file_start_time.elapsed();

                let input_size = external_get_file_size(input_path)?;
                let output_size = external_get_file_size(&output_path)?;
                results.push(EncodeResult {
                    filename: filename.clone(),
                    duration: file_elapsed_time,
                    input_size,
                    output_size,
                });

                crate::progress_reporting::success(&format!(
                    "Encoding complete: {} in {}",
                    filename,
                    format_duration(file_elapsed_time)
                ));

                // Send success notification
                if let Some(sender) = notification_sender {
                    let notification = NotificationType::EncodeComplete {
                        input_path: input_path.clone(),
                        output_path: output_path.clone(),
                        input_size,
                        output_size,
                        duration: file_elapsed_time,
                    };

                    if let Err(e) = sender.send_notification(&notification) {
                        warn!(
                            "Failed to send success notification for {filename}: {e}"
                        );
                    }
                }
            }

            Err(e) => if let CoreError::NoStreamsFound(path) = &e {
                warn!(
                    "Skipping encode for {filename}: FFmpeg reported no processable streams found in '{path}'."
                );

                // Send a notification if notification_sender is provided
                if let Some(sender) = notification_sender {
                    let notification = NotificationType::Custom {
                        title: "Drapto Encode Skipped".to_string(),
                        message: format!(
                            "Skipped encode for {filename}: No streams found."
                        ),
                        priority: 3,
                    };

                    if let Err(err) = sender.send_notification(&notification) {
                        warn!("Failed to send skip notification for {filename}: {err}");
                    }
                }
            } else {
                // Log the error for all other error types
                error!(
                    "ffmpeg encode failed for {filename}: {e}. Check logs for details."
                );

                // Send an error notification if notification_sender is provided
                if let Some(sender) = notification_sender {
                    let notification = NotificationType::EncodeError {
                        input_path: input_path.clone(),
                        message: format!("ffmpeg failed: {e}"),
                    };

                    if let Err(err) = sender.send_notification(&notification) {
                        warn!(
                            "Failed to send error notification for {filename}: {err}"
                        );
                    }
                }
            }
            }
        }

    Ok(results)
}
