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


use crate::config::{CoreConfig, UHD_WIDTH_THRESHOLD, HD_WIDTH_THRESHOLD, HDR_COLOR_SPACES};
use crate::error::{CoreError, CoreResult};
use crate::external::ffmpeg::{EncodeParams, run_ffmpeg_encode};
use crate::external::get_file_size as external_get_file_size;
use crate::notifications::{Notification, NtfyNotificationSender};
use crate::processing::audio;
use crate::processing::crop_detection;
use crate::processing::video_properties::VideoProperties;
use crate::EncodeResult;

use log::warn;

use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Helper function to safely send notifications with consistent error handling.
/// This function blocks until the notification is sent to ensure proper ordering.
fn send_notification_safe(
    sender: Option<&NtfyNotificationSender>,
    title: &str,
    message: String,
    priority: Option<u8>,
    tag: Option<&str>,
    context: &str,
) {
    if let Some(sender) = sender {
        let mut notification = Notification::new(title, message);
        
        if let Some(p) = priority {
            notification = notification.with_priority(p);
        }
        
        if let Some(t) = tag {
            notification = notification.with_tag(t);
        }

        // Send notification synchronously
        if let Err(e) = sender.send(&notification) {
            warn!("Failed to send {} notification: {e}", context);
        }
    }
}

/// Determines quality settings based on video resolution and config.
/// 
/// Returns (quality, category, is_hdr)
fn determine_quality_settings(video_props: &VideoProperties, config: &CoreConfig) -> (u32, &'static str, bool) {
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

    // Detect HDR/SDR status based on color space
    let color_space = video_props.color_space.as_deref().unwrap_or("");
    let is_hdr = HDR_COLOR_SPACES.contains(&color_space);

    (quality.into(), category, is_hdr)
}

/// Sets up encoding parameters from analysis results and config.
fn setup_encoding_parameters(
    input_path: &std::path::Path,
    output_path: &std::path::Path,
    quality: u32,
    config: &CoreConfig,
    crop_filter_opt: Option<String>,
    audio_channels: Vec<u32>,
    duration_secs: f64,
) -> EncodeParams {
    let preset_value = config.encoder_preset;

    let mut initial_encode_params = EncodeParams {
        input_path: input_path.to_path_buf(),
        output_path: output_path.to_path_buf(),
        quality,
        preset: preset_value,
        use_hw_decode: true,
        crop_filter: crop_filter_opt,
        audio_channels,
        duration: duration_secs,
        hqdn3d_params: None,
    };

    // Apply fixed denoising parameters if enabled
    let final_hqdn3d_params = if config.enable_denoise {
        Some(crate::config::FIXED_HQDN3D_PARAMS.to_string())
    } else {
        crate::progress_reporting::info_debug("Denoising disabled via config.");
        None
    };

    // Finalize encoding parameters
    initial_encode_params.hqdn3d_params = final_hqdn3d_params;
    initial_encode_params
}


/// Main entry point for video processing. Orchestrates analysis, encoding, and notifications.
pub fn process_videos(
    notification_sender: Option<&NtfyNotificationSender>,
    config: &CoreConfig,
    files_to_process: &[PathBuf],
    target_filename_override: Option<PathBuf>,
) -> CoreResult<Vec<EncodeResult>> {
    let mut results: Vec<EncodeResult> = Vec::new();

    for input_path in files_to_process {
        let file_start_time = Instant::now();

        let filename = crate::utils::get_filename_safe(input_path)?;

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
            crate::progress_reporting::warning(&error_msg);

            // Send a notification if notification_sender is provided
            send_notification_safe(
                notification_sender,
                "Drapto Encode Skipped",
                format!(
                    "Skipped encode for {}: Output file already exists at {}",
                    filename,
                    output_path.display()
                ),
                None,
                None,
                "skip"
            );

            continue;
        }

        // Send encoding start notification
        send_notification_safe(
            notification_sender,
            "Encoding Started",
            format!("Started encoding {filename}"),
            Some(3),
            Some("start"),
            "start"
        );

        // Analyze video properties
        let video_props = match crate::external::get_video_properties(input_path) {
            Ok(props) => props,
            Err(e) => {
                // Log the error and skip this file
                crate::progress_reporting::error(&format!("Could not analyze {filename}: {e}"));

                // Send an error notification if notification_sender is provided
                send_notification_safe(
                    notification_sender,
                    "Encoding Error",
                    format!("Error encoding {filename}: Failed to get video properties"),
                    Some(5),
                    Some("error"),
                    "error"
                );

                continue;
            }
        };

        let video_width = video_props.width;
        let video_height = video_props.height;
        let duration_secs = video_props.duration_secs;

        // Determine quality settings based on resolution
        let (quality, category, is_hdr) = determine_quality_settings(&video_props, config);

        // Report consolidated video analysis
        crate::progress_reporting::report_video_analysis(&filename, video_width, video_height, duration_secs, category, is_hdr);

        // Perform crop detection
        crate::progress_reporting::report_processing_step("Detecting black bars");

        let disable_crop = config.crop_mode == "off";
        let (crop_filter_opt, _is_hdr) =
            match crop_detection::detect_crop(input_path, &video_props, disable_crop) {
                Ok(result) => result,
                Err(e) => {
                    // Log warning and proceed without cropping
                    crate::progress_reporting::warning(&format!(
                        "Crop detection failed for {filename}: {e}. Proceeding without cropping."
                    ));
                    // detect_crop returns Ok(None) for failures
                    (None, false)
                }
            };

        // Analyze audio and get channel information for encoding (details shown in encoding config)
        let audio_channels = audio::get_audio_channels_quiet(input_path);

        // Setup encoding parameters
        let final_encode_params = setup_encoding_parameters(
            input_path,
            &output_path,
            quality,
            config,
            crop_filter_opt,
            audio_channels,
            duration_secs,
        );

        // Display consolidated encoding configuration
        crate::progress_reporting::report_encoding_configuration(
            final_encode_params.quality,
            final_encode_params.preset,
            &final_encode_params.audio_channels,
            final_encode_params.hqdn3d_params.is_some()
        );

        let encode_result = run_ffmpeg_encode(
            &final_encode_params,
            false, // disable_audio: Keep audio in the output
            final_encode_params.hqdn3d_params.is_some(),  // has_denoising: Whether denoising is applied
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

                crate::progress_reporting::report_final_results(
                    file_elapsed_time,
                    input_size,
                    output_size
                );

                // Send success notification
                let reduction = crate::utils::calculate_size_reduction(input_size, output_size);

                let duration_secs = file_elapsed_time.as_secs();
                let duration_str = if duration_secs >= 3600 {
                    format!("{}h {}m {}s", duration_secs / 3600, (duration_secs % 3600) / 60, duration_secs % 60)
                } else if duration_secs >= 60 {
                    format!("{}m {}s", duration_secs / 60, duration_secs % 60)
                } else {
                    format!("{duration_secs}s")
                };

                send_notification_safe(
                    notification_sender,
                    "Encoding Complete",
                    format!("Completed encoding {filename} in {duration_str}. Reduced by {reduction}%"),
                    Some(4),
                    Some("complete"),
                    "success"
                );
            }

            Err(e) => if let CoreError::NoStreamsFound(path) = &e {
                crate::progress_reporting::warning(&format!(
                    "Skipping encode for {filename}: FFmpeg reported no processable streams found in '{path}'."
                ));

                // Send a notification if notification_sender is provided
                send_notification_safe(
                    notification_sender,
                    "Drapto Encode Skipped",
                    format!("Skipped encode for {filename}: No streams found."),
                    None,
                    None,
                    "skip"
                );
            } else {
                // Log the error for all other error types
                crate::progress_reporting::error(&format!("FFmpeg failed to encode {filename}: {e}"));

                // Send an error notification if notification_sender is provided
                send_notification_safe(
                    notification_sender,
                    "Encoding Error",
                    format!("Error encoding {filename}: ffmpeg failed: {e}"),
                    Some(5),
                    Some("error"),
                    "error"
                );
            }
            }

        // Apply cooldown between encodes when processing multiple files
        // This helps ensure notifications arrive in order
        if files_to_process.len() > 1 && input_path != files_to_process.last().unwrap() && config.encode_cooldown_secs > 0 {
            std::thread::sleep(std::time::Duration::from_secs(config.encode_cooldown_secs));
        }
        }

    // Generate appropriate summary based on number of files processed
    let total_duration: Duration = results.iter().map(|r| r.duration).sum();
    
    match results.len() {
        0 => {
            crate::progress_reporting::warning("No files were successfully encoded");
        }
        1 => {
            // Single file already has final results, just confirm overall success
            crate::progress_reporting::success(&format!("Successfully encoded {}", results[0].filename));
        }
        _ => {
            // Multiple files get consolidated summary
            crate::progress_reporting::report_batch_summary(&results, total_duration);
        }
    }

    Ok(results)
}
