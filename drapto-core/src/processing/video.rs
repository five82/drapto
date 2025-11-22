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
//!    - Execute ffmpeg with the determined parameters
//!    - Handle results and send notifications

use crate::EncodeResult;
use crate::config::CoreConfig;
use crate::error::{CoreError, CoreResult};
use crate::events::{Event, EventDispatcher};
use crate::external::ffmpeg::run_ffmpeg_encode;
use crate::external::ffprobe_executor::get_media_info;
use crate::external::get_file_size as external_get_file_size;
use crate::notifications::NotificationSender;
use crate::processing::analysis::run_crop_detection;
use crate::processing::audio;
use crate::processing::encode_params::{
    EncodingSetupParams, determine_quality_settings, setup_encoding_parameters,
};
use crate::processing::formatting::{
    format_audio_description_basic, format_audio_description_config,
    generate_audio_results_description,
};
use crate::processing::reporting::{
    emit_event, send_notification_safe, send_validation_failure_notifications,
};
use crate::processing::validation::validate_output_video;
use crate::system_info::SystemInfo;
use crate::utils::{SafePath, calculate_size_reduction, resolve_output_path};

use std::path::PathBuf;
use std::time::{Duration, Instant};
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

        // Get detailed audio stream info (used for logging/bitrate calculation)
        let audio_streams = audio::analyze_and_log_audio_detailed(input_path);

        // Format audio description - each track on its own line for multiple tracks
        let audio_description = format_audio_description_basic(&audio_channels);

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
        let (crop_filter_opt, _is_hdr) = run_crop_detection(
            input_path,
            &video_props,
            config,
            event_dispatcher,
            &input_filename,
        );

        // Audio channels already analyzed above

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
        });

        // Format audio description for the config display
        let audio_description =
            format_audio_description_config(&audio_channels, audio_streams.as_deref());

        // Convert video codec to display name
        let encoder_display = match final_encode_params.video_codec.as_str() {
            "libsvtav1" => "SVT-AV1",
            other => other,
        };

        // Convert audio codec to display name - handle mixed spatial/non-spatial tracks
        let audio_codec_display = "Opus".to_string();

        // Emit encoding configuration event
        emit_event(
            event_dispatcher,
            Event::EncodingConfigurationDisplayed {
                encoder: encoder_display.to_string(),
                preset: final_encode_params.preset.to_string(),
                tune: final_encode_params.tune.to_string(),
                quality: format!("CRF {}", final_encode_params.quality),
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

                let (validation_passed, validation_steps) = match validate_output_video(
                    input_path,
                    &output_path,
                    expected_dimensions,
                    Some(duration_secs),
                    Some(is_hdr),
                    expected_audio_track_count,
                    None,
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
