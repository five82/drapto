// drapto-core/src/processing/video.rs
use colored::*; // Import colored for formatting
//
// This module houses the main video processing orchestration logic for the
// `drapto-core` library. Its central piece is the `process_videos` function.
// ... (module documentation remains the same) ...

use crate::config::{CoreConfig, DEFAULT_CORE_QUALITY_HD, DEFAULT_CORE_QUALITY_SD, DEFAULT_CORE_QUALITY_UHD};
use crate::error::{CoreError, CoreResult};
use crate::external::check_dependency;
use crate::external::{FileMetadataProvider, FfmpegSpawner, FfprobeExecutor}; // Added FileMetadataProvider
use crate::external::ffmpeg::{run_ffmpeg_encode, EncodeParams}; // Removed build_ffmpeg_args import
use crate::notifications::Notifier;
use crate::processing::audio;
use crate::processing::detection::{self, grain_analysis}; // Import grain_analysis submodule
use crate::utils::{format_bytes, format_duration, get_file_size};
use crate::EncodeResult;
use log::{info, warn, error};
use std::path::PathBuf;
use std::time::Instant;


/// Processes a list of video files based on the configuration.
/// Uses the standard `log` facade for logging.
/// Returns a list of results for successfully processed files.
pub fn process_videos<S: FfmpegSpawner, P: FfprobeExecutor, N: Notifier, M: FileMetadataProvider>(
    spawner: &S,
    ffprobe_executor: &P,
    notifier: &N,
    metadata_provider: &M, // Add metadata provider
    config: &CoreConfig,
    files_to_process: &[PathBuf],
    target_filename_override: Option<PathBuf>,
) -> CoreResult<Vec<EncodeResult>>
{
    // --- Check Dependencies ---
    info!("{}", "Checking for required external commands...".cyan());
    // Check Dependencies (test-mocks feature removed, check always runs)
    let _ffmpeg_cmd_parts = check_dependency("ffmpeg")?;
    info!("  {} {}", "[OK]".green().bold(), "ffmpeg found.");
    let _ffprobe_cmd_parts = check_dependency("ffprobe")?;
    info!("  {} {}", "[OK]".green().bold(), "ffprobe found.");
    info!("{}", "External dependency check passed.".green());

    // --- Get Hostname ---
    let hostname = hostname::get()
        .map(|s| s.into_string().unwrap_or_else(|_| "unknown-host-invalid-utf8".to_string()))
        .unwrap_or_else(|_| "unknown-host-error".to_string());
    info!("{} {}", "Running on host:".cyan(), hostname.yellow());


    let mut results: Vec<EncodeResult> = Vec::new();

    for input_path in files_to_process {
        let file_start_time = Instant::now();
        let filename = input_path
            .file_name()
            .ok_or_else(|| CoreError::PathError(format!("Failed to get filename for {}", input_path.display())))?
            .to_string_lossy()
            .to_string();
        let _filename_noext = input_path
            .file_stem()
            .ok_or_else(|| CoreError::PathError(format!("Failed to get filename stem for {}", input_path.display())))?
            .to_string_lossy()
            .to_string();

        // --- Determine Output Path ---
        let output_path = match &target_filename_override {
            Some(target_filename) if files_to_process.len() == 1 => {
                config.output_dir.join(target_filename)
            }
            _ => config.output_dir.join(&filename),
        };

        // --- Check for Existing Output File ---
        if output_path.exists() {
            let error_msg = format!(
                "ERROR: Output file already exists: {}. Skipping encode.",
                output_path.display()
            );
            error!("{}", error_msg);

            if let Some(topic) = &config.ntfy_topic {
                let ntfy_message = format!(
                    "[{hostname}]: Skipped encode for {filename}: Output file already exists at {output_display}",
                    hostname = hostname,
                    filename = filename,
                    output_display = output_path.display()
                );
                if let Err(e) = notifier.send(topic, &ntfy_message, Some("Drapto Encode Skipped"), Some(3), Some("warning")) {
                    warn!("Failed to send ntfy skip notification for {}: {}", filename, e);
                }
            }
            info!("----------------------------------------");
            continue;
        }

        info!("{} {}", "Processing:".cyan().bold(), filename.yellow());

        // --- Send Start Notification ---
        if let Some(topic) = &config.ntfy_topic {
            let start_message = format!("[{}]: Starting encode for: {}", hostname, filename);
            if let Err(e) = notifier.send(topic, &start_message, Some("Drapto Encode Start"), Some(3), Some("arrow_forward")) {
                warn!("Failed to send ntfy start notification for {}: {}", filename, e);
            }
        }

        // --- Get Video Properties (including width and duration) ---
        let video_props = match ffprobe_executor.get_video_properties(input_path) {
            Ok(props) => props,
            Err(e) => {
                error!("Failed to get video properties for {}: {}. Skipping file.", filename, e);
                // Send error notification if properties fail
                if let Some(topic) = &config.ntfy_topic {
                     let error_message = format!("[{hostname}]: Error processing {filename}: Failed to get video properties.");
                     if let Err(notify_err) = notifier.send(topic, &error_message, Some("Drapto Process Error"), Some(5), Some("x,rotating_light")) {
                         warn!("Failed to send ntfy error notification for {}: {}", filename, notify_err);
                     }
                 }
                info!("----------------------------------------");
                continue;
            }
        };
        let video_width = video_props.width;
        let duration_secs = video_props.duration_secs; // Store duration

        // --- Determine Quality based on Width ---
        const UHD_WIDTH_THRESHOLD: u32 = 3840;
        const HD_WIDTH_THRESHOLD: u32 = 1920;
        let quality = if video_width >= UHD_WIDTH_THRESHOLD {
            config.quality_uhd.unwrap_or(DEFAULT_CORE_QUALITY_UHD)
        } else if video_width >= HD_WIDTH_THRESHOLD {
            config.quality_hd.unwrap_or(DEFAULT_CORE_QUALITY_HD)
        } else {
            config.quality_sd.unwrap_or(DEFAULT_CORE_QUALITY_SD)
        };
        let category = if video_width >= UHD_WIDTH_THRESHOLD { "UHD" } else if video_width >= HD_WIDTH_THRESHOLD { "HD" } else { "SD" };
        info!(
            "Detected video width: {} ({}) - CRF set to {}",
            video_width.to_string().green(),
            category.green(),
            quality.to_string().green().bold()
        );

        // --- Crop Detection ---
        let disable_crop = config.default_crop_mode.as_deref() == Some("off");
        let (crop_filter_opt, _is_hdr) = match detection::detect_crop(spawner, input_path, &video_props, disable_crop) {
            Ok(result) => result,
            Err(e) => {
                warn!("Crop detection failed for {}: {}. Proceeding without cropping.", filename, e);
                 // Send error notification if crop detection fails critically (though it currently returns Ok(None))
                 // If detect_crop could return Err, we'd handle it here similarly to get_video_properties
                (None, false)
            }
        };

        // --- Prepare Audio Options ---
        let _ = audio::log_audio_info(ffprobe_executor, input_path);
        let audio_channels = match ffprobe_executor.get_audio_channels(input_path) {
             Ok(channels) => channels,
             Err(e) => {
                 warn!("Error getting audio channels for ffmpeg command build: {}. Using empty list.", e);
                 vec![]
             }
         };

        // --- Build Initial Encode Params (without denoise) ---
        let preset_value = config.preset.or(config.default_encoder_preset).unwrap_or(6);
        let mut initial_encode_params = EncodeParams {
            input_path: input_path.to_path_buf(),
            output_path: output_path.clone(), // Clone for initial params
            quality: quality.into(),
            preset: preset_value,
            crop_filter: crop_filter_opt.clone(), // Clone crop filter
            audio_channels: audio_channels.clone(), // Clone audio channels
            duration: duration_secs,
            hqdn3d_params: None, // Explicitly None for initial build and grain analysis
        };

        // --- Grain Detection & Denoise Parameter Selection (Plan 2: Relative Comparison) ---
        let final_hqdn3d_params_result = if config.enable_denoise {
            info!("{}", "Grain detection enabled, analyzing video using relative sample comparison...".cyan());
            grain_analysis::analyze_grain(input_path, config, &initial_encode_params, duration_secs, spawner, metadata_provider)
        } else {
            info!("Denoising disabled via config.");
            Ok(None) // Treat as Ok(None) if disabled
        };

        // Handle potential errors from grain analysis before proceeding
        let final_hqdn3d_params: Option<String> = match final_hqdn3d_params_result {
             Ok(Some(result)) => {
                 // Use the updated determine_hqdn3d_params which returns Option<String>
                 let params_opt = grain_analysis::determine_hqdn3d_params(result.detected_level);
                 info!(
                     "Grain analysis result: {:?}, applying filter: {}",
                     result.detected_level,
                     params_opt.as_deref().unwrap_or("None") // Log "None" if VeryClean
                 );
                 params_opt // This is already Option<String>
             }
             Ok(None) => {
                 // Analysis was skipped (e.g., short video) or did not produce a result
                 info!("Grain analysis skipped or did not produce a result. Proceeding without denoising.");
                 None
             }
             Err(e) => {
                 // Critical error during analysis
                 error!("Grain analysis failed critically: {}. Skipping file.", e);
                 // Send error notification
                 if let Some(topic) = &config.ntfy_topic {
                     let error_message = format!("[{hostname}]: Error processing {filename}: Grain analysis failed.");
                     if let Err(notify_err) = notifier.send(topic, &error_message, Some("Drapto Process Error"), Some(5), Some("x,rotating_light")) {
                         warn!("Failed to send ntfy error notification for {}: {}", filename, notify_err);
                     }
                 }
                 info!("----------------------------------------");
                 continue; // Skip this file
             }
         };

        // --- Finalize Encode Params with Denoise ---
        // Update the initial params struct with the determined hqdn3d filter
        initial_encode_params.hqdn3d_params = final_hqdn3d_params;
        // Now `initial_encode_params` contains the final set of parameters for the main encode
        let final_encode_params = initial_encode_params;


        // --- Execute ffmpeg via sidecar ---
        // The run_ffmpeg_encode function handles its own start logging.
        let encode_result = run_ffmpeg_encode(
            spawner,
            &final_encode_params,
            false, /* disable_audio */
            false, /* is_grain_analysis_sample */
            None,  /* grain_level_being_tested (not applicable for final encode) */
        );


        // --- Handle Result ---
        match encode_result {
            Ok(()) => { // Encode succeeded
            let file_elapsed_time = file_start_time.elapsed();
            let input_size = get_file_size(input_path)?;
            let output_size = get_file_size(&output_path)?;

            results.push(EncodeResult {
                filename: filename.clone(),
                duration: file_elapsed_time,
                input_size,
                output_size,
            });

            let completion_log_msg = format!("Completed: {} in {}", filename, format_duration(file_elapsed_time));
            info!("{}", completion_log_msg);

            // --- Send Success Notification ---
            if let Some(topic) = &config.ntfy_topic {
                let reduction = if input_size > 0 {
                    100u64.saturating_sub(output_size.saturating_mul(100) / input_size)
                } else { 0 };
                let success_message = format!(
                    "[{hostname}]: Successfully encoded {filename} in {duration}.\nSize: {in_size} -> {out_size} (Reduced by {reduct}%)",
                    hostname = hostname,
                    filename = filename,
                    duration = format_duration(file_elapsed_time),
                    in_size = format_bytes(input_size),
                    out_size = format_bytes(output_size),
                    reduct = reduction
                );
                if let Err(e) = notifier.send(topic, &success_message, Some("Drapto Encode Success"), Some(4), Some("white_check_mark")) {
                    warn!("Failed to send ntfy success notification for {}: {}", filename, e);
                }
            }

            }
            Err(CoreError::NoStreamsFound(path)) => { // Specific error: No streams found
                warn!(
                    "Skipping encode for {}: FFmpeg reported no processable streams found in '{}'.",
                    filename, path
                );
                // Optionally send a specific notification for this case
                if let Some(topic) = &config.ntfy_topic {
                    let skip_message = format!(
                        "[{hostname}]: Skipped encode for {filename}: No streams found.",
                        hostname = hostname,
                        filename = filename
                    );
                    if let Err(notify_err) = notifier.send(topic, &skip_message, Some("Drapto Encode Skipped"), Some(3), Some("warning")) {
                        warn!("Failed to send ntfy skip notification for {}: {}", filename, notify_err);
                    }
                }
                // No `continue` here, just let the loop proceed after logging/notifying
            }
            Err(e) => { // Generic encode failed
                error!(
                    "ffmpeg encode failed for {}: {}. Check logs for details.",
                    filename, e
                );

                // --- Send Error Notification ---
                if let Some(topic) = &config.ntfy_topic {
                    let error_message = format!(
                        "[{hostname}]: Error encoding {filename}: ffmpeg failed.",
                        hostname = hostname,
                        filename = filename
                    );
                    if let Err(notify_err) = notifier.send(topic, &error_message, Some("Drapto Encode Error"), Some(5), Some("x,rotating_light")) {
                        warn!("Failed to send ntfy error notification for {}: {}", filename, notify_err);
                    }
                }
                // No `continue` here either for general errors, loop proceeds
            }
        }
         info!("----------------------------------------");

    } // End loop through files

    Ok(results)
}