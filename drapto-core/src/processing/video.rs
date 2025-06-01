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
use crate::EncodeResult;
use crate::utils::format_duration;

use log::{error, info, warn};

use std::path::PathBuf;
use std::time::Instant;

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
                let notification = Notification::new(
                    "Drapto Encode Skipped",
                    format!(
                        "Skipped encode for {}: Output file already exists at {}",
                        filename,
                        output_path.display()
                    )
                );

                if let Err(e) = sender.send(&notification) {
                    warn!("Failed to send notification for {filename}: {e}");
                }
            }

            continue;
        }

        crate::terminal::print_status("File", &filename, false);

        // Send encoding start notification
        if let Some(sender) = notification_sender {
            let notification = Notification::new(
                "Encoding Started",
                format!("Started encoding {filename}")
            ).with_priority(3).with_tag("start");

            if let Err(e) = sender.send(&notification) {
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
                    let notification = Notification::new(
                        "Encoding Error",
                        format!("Error encoding {filename}: Failed to get video properties")
                    ).with_priority(5).with_tag("error");

                    if let Err(e) = sender.send(&notification) {
                        warn!("Failed to send error notification for {filename}: {e}");
                    }
                }

                info!("");
                continue;
            }
        };

        let video_width = video_props.width;
        let duration_secs = video_props.duration_secs;

        // Determine quality settings based on resolution
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
        crate::terminal::print_status("Video quality", &format!("{} ({}) - CRF {}", video_width, category, quality), false);
        crate::terminal::print_status("Duration", &format!("{:.2}s", duration_secs), false);

        // Detect and report HDR/SDR status based on color space
        let color_space = video_props.color_space.as_deref().unwrap_or("");
        let is_hdr = HDR_COLOR_SPACES.contains(&color_space);
        let dynamic_range = if is_hdr { "HDR" } else { "SDR" };
        crate::terminal::print_status("Dynamic range", dynamic_range, false);

        // Perform crop detection
        crate::terminal::print_processing("Detecting black bars");

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
        crate::terminal::print_processing("Audio analysis");

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

        // Apply fixed denoising parameters if enabled
        let final_hqdn3d_params = if config.enable_denoise {
            Some(crate::config::FIXED_HQDN3D_PARAMS.to_string())
        } else {
            crate::terminal::print_sub_item("Denoising disabled via config.");
            None
        };


        // Finalize encoding parameters
        initial_encode_params.hqdn3d_params = final_hqdn3d_params.clone();
        let final_encode_params = initial_encode_params;

        log::debug!("ENCODING CONFIGURATION");
        log::debug!("Video:");
        log::debug!("Preset: {preset_value} (SVT-AV1)");
        log::debug!("Quality: {quality} (CRF)");

        if let Some(hqdn3d) = &final_hqdn3d_params {
            log::debug!("Grain Level: VeryLight ({hqdn3d})");
        } else {
            log::debug!("Grain Level: None (no denoising)");
        }

        // Hardware info
        log::debug!("Hardware:");
        let hw_info = crate::hardware_decode::get_hardware_decoding_info();
        let hw_display = match hw_info {
            Some(info) => format!("{info} (decode only)"),
            None => "None available".to_string(),
        };
        log::debug!("Acceleration: {hw_display}");

        crate::terminal::print_section("ENCODING CONFIGURATION");
        
        // Video settings - Level 3 subsection within main section
        crate::terminal::print_subsection_level3("Video:");
        crate::terminal::print_status("Preset", &format!("{} (SVT-AV1)", preset_value), false);
        crate::terminal::print_status("Quality", &format!("{} (CRF)", quality), false);

        if let Some(hqdn3d) = &final_hqdn3d_params {
            crate::terminal::print_status("Grain Level", &format!("VeryLight ({})", hqdn3d), false);
        } else {
            crate::terminal::print_status("Grain Level", "None (no denoising)", false);
        }

        // Hardware info - Level 3 subsection within main section
        crate::terminal::print_subsection_level3_with_spacing("Hardware:");
        let hw_info = crate::hardware_decode::get_hardware_decoding_info();
        let hw_display = match hw_info {
            Some(info) => format!("{info} (decode only)"),
            None => "No hardware decoder available".to_string(),
        };
        crate::terminal::print_status("Acceleration", &hw_display, false);

        let encode_result = run_ffmpeg_encode(
            &final_encode_params,
            false, // disable_audio: Keep audio in the output
            final_hqdn3d_params.is_some(),  // has_denoising: Whether denoising is applied
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

                crate::terminal::print_completion_with_status(
                    &format!("Encoding complete: {}", filename),
                    "Time",
                    &format_duration(file_elapsed_time.as_secs_f64())
                );

                // Send success notification
                if let Some(sender) = notification_sender {
                    let reduction = if input_size > 0 {
                        100 - ((output_size * 100) / input_size)
                    } else {
                        0
                    };

                    let duration_secs = file_elapsed_time.as_secs();
                    let duration_str = if duration_secs >= 3600 {
                        format!("{}h {}m {}s", duration_secs / 3600, (duration_secs % 3600) / 60, duration_secs % 60)
                    } else if duration_secs >= 60 {
                        format!("{}m {}s", duration_secs / 60, duration_secs % 60)
                    } else {
                        format!("{duration_secs}s")
                    };

                    let notification = Notification::new(
                        "Encoding Complete",
                        format!("Completed encoding {filename} in {duration_str}. Reduced by {reduction}%")
                    ).with_priority(4).with_tag("complete");

                    if let Err(e) = sender.send(&notification) {
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
                    let notification = Notification::new(
                        "Drapto Encode Skipped",
                        format!("Skipped encode for {filename}: No streams found.")
                    );

                    if let Err(err) = sender.send(&notification) {
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
                    let notification = Notification::new(
                        "Encoding Error",
                        format!("Error encoding {filename}: ffmpeg failed: {e}")
                    ).with_priority(5).with_tag("error");

                    if let Err(err) = sender.send(&notification) {
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
