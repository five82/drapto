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
use crate::events::{Event, EventDispatcher};
use crate::external::ffmpeg::{EncodeParams, run_ffmpeg_encode};
use crate::external::ffprobe_executor::get_media_info;
use crate::external::get_file_size as external_get_file_size;
use crate::notifications::NotificationSender;
use crate::processing::audio;
use crate::processing::crop_detection;
use crate::processing::video_properties::VideoProperties;
use crate::processing::validation::validate_output_video;
use crate::system_info::SystemInfo;
use crate::EncodeResult;

use log::warn;

/// Parameters for setting up encoding configuration
struct EncodingSetupParams<'a> {
    input_path: &'a std::path::Path,
    output_path: &'a std::path::Path,
    quality: u32,
    config: &'a CoreConfig,
    crop_filter_opt: Option<String>,
    audio_channels: Vec<u32>,
    duration_secs: f64,
    is_hdr: bool,
    video_props: &'a VideoProperties,
}

use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Helper function to safely send notifications with consistent error handling.
fn send_notification_safe(
    sender: Option<&dyn NotificationSender>,
    message: &str,
    context: &str,
) {
    if let Some(sender) = sender {
        if let Err(e) = sender.send(message) {
            warn!("Failed to send {} notification: {e}", context);
        }
    }
}

/// Helper function to emit events if event dispatcher is available
fn emit_event(event_dispatcher: Option<&EventDispatcher>, event: Event) {
    if let Some(dispatcher) = event_dispatcher {
        dispatcher.emit(event);
    }
}

/// Calculate final output dimensions after crop is applied
fn get_output_dimensions(original_width: u32, original_height: u32, crop_filter: Option<&str>) -> (u32, u32) {
    if let Some(crop) = crop_filter {
        // Parse crop filter format: crop=width:height:x:y
        if let Some(params) = crop.strip_prefix("crop=") {
            let parts: Vec<&str> = params.split(':').collect();
            if parts.len() >= 2 {
                if let (Ok(width), Ok(height)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
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
fn setup_encoding_parameters(params: EncodingSetupParams) -> EncodeParams {
    let preset_value = params.config.svt_av1_preset;
    let tune_value = params.config.svt_av1_tune;

    let mut initial_encode_params = EncodeParams {
        input_path: params.input_path.to_path_buf(),
        output_path: params.output_path.to_path_buf(),
        quality: params.quality,
        preset: preset_value,
        tune: tune_value,
        use_hw_decode: true,
        crop_filter: params.crop_filter_opt,
        audio_channels: params.audio_channels,
        duration: params.duration_secs,
        hqdn3d_params: None,
        // Actual values that will be used in FFmpeg command
        video_codec: "libsvtav1".to_string(),
        pixel_format: "yuv420p10le".to_string(),
        color_space: params.video_props.color_space.clone().unwrap_or_else(|| "bt709".to_string()),
        audio_codec: "libopus".to_string(),
        film_grain_level: 0, // Will be set below
    };

    // Apply fixed denoising parameters if enabled, using HDR or SDR specific values
    let final_hqdn3d_params = if params.config.enable_denoise {
        let denoise_params = if params.is_hdr {
            crate::config::FIXED_HQDN3D_PARAMS_HDR
        } else {
            crate::config::FIXED_HQDN3D_PARAMS_SDR
        };
        Some(denoise_params.to_string())
    } else {
        log::debug!("Denoising disabled via config.");
        None
    };

    // Set film grain level based on denoising
    let film_grain_level = if params.config.enable_denoise {
        crate::config::FIXED_FILM_GRAIN_VALUE
    } else {
        0
    };

    // Finalize encoding parameters
    initial_encode_params.hqdn3d_params = final_hqdn3d_params;
    initial_encode_params.film_grain_level = film_grain_level;
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
    emit_event(event_dispatcher, Event::HardwareInfo {
        hostname: system_info.hostname,
        os: system_info.os,
        cpu: system_info.cpu,
        memory: system_info.memory,
        decoder: system_info.decoder,
    });

    // Show batch initialization for multiple files
    if files_to_process.len() > 1 {
        emit_event(event_dispatcher, Event::BatchInitializationStarted {
            total_files: files_to_process.len(),
            file_list: files_to_process.iter()
                .filter_map(|p| p.file_name())
                .filter_map(|n| n.to_str())
                .map(|s| s.to_string())
                .collect(),
            output_dir: config.output_dir.display().to_string(),
        });
    }

    for (file_index, input_path) in files_to_process.iter().enumerate() {
        let file_start_time = Instant::now();

        // Show file progress context for multiple files
        if files_to_process.len() > 1 {
            emit_event(event_dispatcher, Event::FileProgressContext {
                current_file: file_index + 1,
                total_files: files_to_process.len(),
            });
        }

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
            let message = format!(
                "Output file already exists: {}. Skipping encode.",
                output_path.display()
            );
            
            emit_event(event_dispatcher, Event::Warning { message: message.clone() });
            
            send_notification_safe(
                notification_sender,
                &format!("Skipped encode for {}: Output file already exists at {}", filename, output_path.display()),
                "skip"
            );

            continue;
        }

        // Send encoding start notification
        send_notification_safe(
            notification_sender,
            &format!("Started encoding {filename}"),
            "start"
        );

        // Analyze video properties
        let video_props = match crate::external::get_video_properties(input_path) {
            Ok(props) => props,
            Err(e) => {
                let error_msg = format!("Could not analyze {filename}: {e}");
                emit_event(event_dispatcher, Event::Error {
                    title: "Analysis Error".to_string(),
                    message: error_msg.clone(),
                    context: Some(format!("File: {}", input_path.display())),
                    suggestion: Some("Check if the file is a valid video format".to_string()),
                });

                send_notification_safe(
                    notification_sender,
                    &format!("Error encoding {filename}: Failed to get video properties"),
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

        // Get audio channels early for consolidated reporting
        let audio_channels = audio::get_audio_channels_quiet(input_path);

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
            let track_descriptions: Vec<String> = audio_channels.iter()
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
            track_descriptions.join("\n                     ")  // Indent continuation lines
        };

        // Emit initialization event
        emit_event(event_dispatcher, Event::InitializationStarted {
            input_file: filename.clone(),
            output_file: output_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            duration: crate::utils::format_duration(duration_secs as f64),
            resolution: format!("{}x{}", video_width, video_height),
            category: category.to_string(),
            dynamic_range: if is_hdr { "HDR".to_string() } else { "SDR".to_string() },
            audio_description,
            hardware: if crate::hardware_decode::is_hardware_decoding_available() {
                Some("VideoToolbox (decode only)".to_string())
            } else {
                None
            },
        });

        // Perform video analysis (crop detection)
        emit_event(event_dispatcher, Event::VideoAnalysisStarted);
        emit_event(event_dispatcher, Event::BlackBarDetectionStarted);

        let disable_crop = config.crop_mode == "off";
        let (crop_filter_opt, _is_hdr) =
            match crop_detection::detect_crop(input_path, &video_props, disable_crop) {
                Ok(result) => {
                    emit_event(event_dispatcher, Event::BlackBarDetectionComplete {
                        crop_required: result.0.is_some(),
                        crop_params: result.0.clone(),
                    });
                    result
                },
                Err(e) => {
                    let warning_msg = format!(
                        "Crop detection failed for {filename}: {e}. Proceeding without cropping."
                    );
                    emit_event(event_dispatcher, Event::Warning { message: warning_msg });
                    emit_event(event_dispatcher, Event::BlackBarDetectionComplete {
                        crop_required: false,
                        crop_params: None,
                    });
                    (None, false)
                }
            };

        // Audio channels already analyzed above

        // Setup encoding parameters
        let final_encode_params = setup_encoding_parameters(EncodingSetupParams {
            input_path,
            output_path: &output_path,
            quality,
            config,
            crop_filter_opt,
            audio_channels: audio_channels.clone(),
            duration_secs,
            is_hdr,
            video_props: &video_props,
        });

        // Format audio description for the config display
        let audio_description = if audio_channels.is_empty() {
            "No audio".to_string()
        } else if audio_channels.len() == 1 {
            let bitrate = crate::processing::audio::calculate_audio_bitrate(audio_channels[0]);
            let channel_desc = match audio_channels[0] {
                1 => "Mono".to_string(),
                2 => "Stereo".to_string(), 
                6 => "5.1".to_string(),
                8 => "7.1".to_string(),
                n => format!("{} channels", n), // fallback
            };
            format!("{} @ {}kbps", channel_desc, bitrate)
        } else {
            // Multiple audio tracks - show each on its own line
            let track_descriptions: Vec<String> = audio_channels.iter()
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
            track_descriptions.join("\n                     ")  // Indent continuation lines
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

        // Convert audio codec to display name  
        let audio_codec_display = match final_encode_params.audio_codec.as_str() {
            "libopus" => "Opus",
            other => other,
        };

        // Emit encoding configuration event
        emit_event(event_dispatcher, Event::EncodingConfigurationDisplayed {
            encoder: encoder_display.to_string(),
            preset: final_encode_params.preset.to_string(),
            tune: final_encode_params.tune.to_string(), 
            quality: format!("CRF {}", final_encode_params.quality),
            denoising: final_encode_params.hqdn3d_params
                .as_ref()
                .map(|p| format!("hqdn3d={}", p))
                .unwrap_or_else(|| "None".to_string()),
            film_grain: film_grain_display,
            hardware_accel: if crate::hardware_decode::is_hardware_decoding_available() {
                Some("VideoToolbox (decode only)".to_string())
            } else {
                None
            },
            pixel_format: final_encode_params.pixel_format.clone(),
            color_space: final_encode_params.color_space.clone(),
            audio_codec: audio_codec_display.to_string(),
            audio_description,
        });

        // Get total frame count for progress reporting
        let total_frames = match get_media_info(&input_path) {
            Ok(info) => info.total_frames.unwrap_or(0),
            Err(e) => {
                log::warn!("Failed to get media info for frame count: {}", e);
                0
            }
        };

        // Emit encoding started event
        emit_event(event_dispatcher, Event::EncodingStarted {
            total_frames,
        });

        let encode_result = run_ffmpeg_encode(
            &final_encode_params,
            false, // disable_audio: Keep audio in the output
            final_encode_params.hqdn3d_params.is_some(),  // has_denoising: Whether denoising is applied
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
                let expected_dimensions = if let Some(ref crop_filter) = final_encode_params.crop_filter {
                    Some(get_output_dimensions(video_width, video_height, Some(crop_filter)))
                } else {
                    Some((video_width, video_height))
                };

                // Perform post-encode validation
                let (validation_passed, validation_steps) = match validate_output_video(&output_path, expected_dimensions) {
                    Ok(validation_result) => {
                        let steps = validation_result.get_validation_steps();
                        
                        if !validation_result.is_valid() {
                            let failures = validation_result.get_failures();
                            let error_msg = format!(
                                "Post-encode validation failed for {}: {}",
                                filename,
                                failures.join(", ")
                            );
                            
                            emit_event(event_dispatcher, Event::Error {
                                title: "Validation Error".to_string(),
                                message: error_msg.clone(),
                                context: Some(format!("Output file: {}", output_path.display())),
                                suggestion: Some("Check encoding configuration and try again".to_string()),
                            });

                            send_notification_safe(
                                notification_sender,
                                &format!("Validation failed for {}: {}", filename, failures.join(", ")),
                                "validation_error"
                            );

                            continue;
                        } else {
                            log::debug!(
                                "Post-encode validation passed for {}: AV1 codec with 10-bit depth confirmed",
                                filename
                            );
                            
                            (true, steps)
                        }
                    }
                    Err(validation_error) => {
                        let error_msg = format!(
                            "Post-encode validation error for {}: {}",
                            filename,
                            validation_error
                        );
                        
                        emit_event(event_dispatcher, Event::Warning { 
                            message: error_msg.clone() 
                        });

                        log::warn!("{}", error_msg);
                        // Continue processing even if validation fails due to technical issues
                        let error_steps = vec![
                            ("Video codec".to_string(), false, "Validation error".to_string()),
                            ("Bit depth".to_string(), false, "Validation error".to_string()),
                            ("Crop detection".to_string(), false, "Validation error".to_string()),
                        ];
                        (false, error_steps)
                    }
                };
                
                results.push(EncodeResult {
                    filename: filename.clone(),
                    duration: file_elapsed_time,
                    input_size,
                    output_size,
                    video_duration_secs: duration_secs,
                    encoding_speed,
                    validation_passed,
                    validation_steps: validation_steps.clone(),
                });

                // Emit validation complete event
                emit_event(event_dispatcher, Event::ValidationComplete {
                    validation_passed,
                    validation_steps: validation_steps.clone(),
                });

                // Calculate final output dimensions (considering crop)
                let (final_width, final_height) = get_output_dimensions(
                    video_width, 
                    video_height, 
                    final_encode_params.crop_filter.as_deref()
                );

                // Emit encoding complete event
                emit_event(event_dispatcher, Event::EncodingComplete {
                    input_file: filename.clone(),
                    output_file: output_path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    original_size: input_size,
                    encoded_size: output_size,
                    video_stream: format!("AV1 (libsvtav1), {}x{}", 
                        final_width, final_height),
                    audio_stream: if audio_channels.is_empty() {
                        "No audio".to_string()
                    } else if audio_channels.len() == 1 {
                        let channel_desc = match audio_channels[0] {
                            1 => "Mono".to_string(),
                            2 => "Stereo".to_string(),
                            6 => "5.1 surround".to_string(),
                            8 => "7.1 surround".to_string(),
                            n => format!("{} channels", n),
                        };
                        format!("Opus, {}, {} kb/s", 
                            channel_desc, 
                            crate::processing::audio::calculate_audio_bitrate(audio_channels[0]))
                    } else {
                        // Multiple audio tracks - show each on its own line
                        let track_descriptions: Vec<String> = audio_channels.iter()
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
                                format!("Track {}: Opus, {}, {} kb/s", i + 1, desc, bitrate)
                            })
                            .collect();
                        track_descriptions.join("\n                     ")  // Indent continuation lines
                    },
                    total_time: file_elapsed_time,
                    average_speed: duration_secs as f32 / file_elapsed_time.as_secs_f32(),
                    output_path: output_path.display().to_string(),
                });

                // Send success notification
                let reduction = ((input_size - output_size) as f64 / input_size as f64 * 100.0) as u32;

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
                    &format!("Completed encoding {filename} in {duration_str}. Reduced by {reduction}%"),
                    "success"
                );
            }

            Err(e) => if let CoreError::NoStreamsFound(path) = &e {
                let warning_msg = format!(
                    "Skipping encode for {filename}: FFmpeg reported no processable streams found in '{path}'."
                );
                emit_event(event_dispatcher, Event::Warning { message: warning_msg });

                send_notification_safe(
                    notification_sender,
                    &format!("Skipped encode for {filename}: No streams found."),
                    "skip"
                );
            } else {
                let error_msg = format!("FFmpeg failed to encode {filename}: {e}");
                emit_event(event_dispatcher, Event::Error {
                    title: "Encoding Error".to_string(),
                    message: error_msg.clone(),
                    context: Some(format!("File: {}", input_path.display())),
                    suggestion: Some("Check FFmpeg logs for more details".to_string()),
                });

                send_notification_safe(
                    notification_sender,
                    &format!("Error encoding {filename}: ffmpeg failed: {e}"),
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
    match results.len() {
        0 => {
            emit_event(event_dispatcher, Event::Warning { 
                message: "No files were successfully encoded".to_string() 
            });
        }
        1 => {
            // Single file - keep existing behavior
            emit_event(event_dispatcher, Event::OperationComplete {
                message: format!("Successfully encoded {}", results[0].filename),
            });
        }
        _ => {
            // Multiple files - emit detailed batch summary
            let total_duration: Duration = results.iter().map(|r| r.duration).sum();
            let total_original_size: u64 = results.iter().map(|r| r.input_size).sum();
            let total_encoded_size: u64 = results.iter().map(|r| r.output_size).sum();
            
            // Calculate average speed across all files
            let total_video_duration_secs: f64 = results.iter()
                .map(|r| r.video_duration_secs)
                .sum();
            let average_speed = if total_duration.as_secs_f64() > 0.0 {
                (total_video_duration_secs / total_duration.as_secs_f64()) as f32
            } else {
                0.0f32
            };
            
            // Build per-file results for the summary
            let file_results: Vec<(String, f64)> = results.iter()
                .map(|r| {
                    let reduction = (r.input_size - r.output_size) as f64 / r.input_size as f64 * 100.0;
                    (r.filename.clone(), reduction)
                })
                .collect();
            
            // Count validation results
            let validation_passed_count = results.iter().filter(|r| r.validation_passed).count();
            let validation_failed_count = results.len() - validation_passed_count;
            
            emit_event(event_dispatcher, Event::BatchComplete {
                successful_count: results.len(),
                total_files: files_to_process.len(),
                total_original_size,
                total_encoded_size,
                total_duration,
                average_speed: average_speed as f32,
                file_results,
                validation_passed_count,
                validation_failed_count,
            });
        }
    }

    Ok(results)
}
