// drapto-core/src/processing/video.rs
//
// This module houses the main video processing orchestration logic for the
// `drapto-core` library. Its central piece is the `process_videos` function.
//
// Responsibilities of `process_videos`:
// - Takes a `CoreConfig`, a list of video files (`files_to_process`), and a
//   logging callback (`log_callback`) as input.
// - Performs initial checks for required external dependencies (`ffmpeg`, `ffprobe`)
//   using functions from the `external` module.
// - Iterates through each provided video file path.
// - For each file:
//   - Determines the output path based on the configuration.
//   - Retrieves audio track channel counts using `ffprobe` via the `external` module.
//   - Calculates appropriate audio bitrates based on channel counts using the
//     internal `calculate_audio_bitrate` helper function.
//   - It constructs the ffmpeg command with appropriate arguments using the `external::ffmpeg` module.
//   - Constructs the full argument list for the `ffmpeg` command, incorporating
//     settings from the `CoreConfig`, detected crop filter, and audio parameters.
//   - Spawns `ffmpeg` as a subprocess, capturing its stderr (stdout is ignored as progress goes to stderr).
//   - Uses separate threads to concurrently read stdout and stderr, sending chunks
//     of output through an MPSC channel to the main thread.
//   - The main thread receives these chunks and passes them to the `log_callback`
//     for real-time progress reporting. Stderr is also captured separately for
//     potential error messages.
//   - Waits for the `ffmpeg` process to complete.
//   - If the process succeeds:
//     - Retrieves input and output file sizes using `utils::get_file_size`.
//     - Creates an `EncodeResult` struct containing filename, duration, and sizes.
//     - Adds the `EncodeResult` to a list of successful results.
//   - If the process fails:
//     - Logs an error message including the exit status and the captured stderr content.
//     - Continues processing the next file (does not stop the entire batch).
// - Finally, returns a `CoreResult` containing a `Vec<EncodeResult>` for all files
//   that were processed successfully.

use crate::config::{CoreConfig, DEFAULT_CORE_QUALITY_HD, DEFAULT_CORE_QUALITY_SD, DEFAULT_CORE_QUALITY_UHD};
// Removed unused send_ntfy import
use crate::error::{CoreError, CoreResult};
#[cfg(not(feature = "test-mocks"))]
use crate::external::check_dependency;
use crate::external::{FfmpegSpawner, FfprobeExecutor}; // Import FfprobeExecutor trait
use crate::external::ffmpeg::{run_ffmpeg_encode, EncodeParams}; // Import function and params struct
use crate::notifications::Notifier; // Import the Notifier trait
use crate::processing::audio; // To access audio submodule
use crate::processing::detection; // Import the new detection module
use crate::utils::{format_bytes, format_duration, get_file_size}; // Added format_bytes, format_duration
use crate::EncodeResult; // Assuming EncodeResult stays in lib.rs or is re-exported from there

// Remove VecDeque as it's no longer needed for args
// Remove unused imports related to manual process handling
// use std::io::Read;
use std::path::PathBuf;
// use std::process::{Command, Stdio};
// use std::sync::mpsc;
// use std::thread;
use std::time::Instant;


/// Processes a list of video files based on the configuration.
/// Calls the `log_callback` for logging messages.
/// Returns a list of results for successfully processed files.

pub fn process_videos<S: FfmpegSpawner, P: FfprobeExecutor, N: Notifier, F>( // Add FfprobeExecutor generic
    spawner: &S,
    ffprobe_executor: &P, // Add ffprobe_executor argument
    notifier: &N,
    config: &CoreConfig,
    files_to_process: &[PathBuf],
    target_filename_override: Option<PathBuf>,
    mut log_callback: F,
) -> CoreResult<Vec<EncodeResult>>
where
    F: FnMut(&str), // Remove Send + 'static bounds
{
    // --- Check Dependencies ---
    // No need to clone here, use the mutable reference directly
    log_callback("Checking for required external commands...");
    // Check for ffmpeg and ffprobe only if not using mock feature
    #[cfg(not(feature = "test-mocks"))]
    {
        log_callback("Checking for required external commands...");
        let _ffmpeg_cmd_parts = check_dependency("ffmpeg")?;
        log_callback("  [OK] ffmpeg found.");
        let _ffprobe_cmd_parts = check_dependency("ffprobe")?;
        log_callback("  [OK] ffprobe found.");
        log_callback("External dependency check passed.");
    }
    #[cfg(feature = "test-mocks")]
    {
        log_callback("Skipping external command check (test-mocks enabled).");
    }
    log_callback("External dependency check passed.");

    // --- Get Hostname ---
    let hostname = hostname::get()
        .map(|s| s.into_string().unwrap_or_else(|_| "unknown-host-invalid-utf8".to_string()))
        .unwrap_or_else(|_| "unknown-host-error".to_string());
    log_callback(&format!("Running on host: {}", hostname));


    let mut results: Vec<EncodeResult> = Vec::new();
    // Removed cmd_handbrake reference

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
            // If an override filename is provided (meaning single input file + output file path given)
            Some(target_filename) if files_to_process.len() == 1 => {
                config.output_dir.join(target_filename) // Join the *actual* output dir with the target filename
            }
            // Otherwise (multiple input files OR output path was a directory), use the input filename
            _ => config.output_dir.join(&filename),
        };

        // --- Check for Existing Output File ---
        if output_path.exists() {
            let error_msg = format!(
                "ERROR: Output file already exists: {}. Skipping encode.",
                output_path.display()
            );
            log_callback(&error_msg);

            // Send notification if configured
            if let Some(topic) = &config.ntfy_topic {
                let ntfy_message = format!(
                    "[{hostname}]: Skipped encode for {filename}: Output file already exists at {output_display}",
                    hostname = hostname, // Already fetched earlier
                    filename = filename,
                    output_display = output_path.display()
                );
                // Use notifier trait
                if let Err(e) = notifier.send(topic, &ntfy_message, Some("Drapto Encode Skipped"), Some(3), Some("warning")) {
                    log_callback(&format!("Warning: Failed to send ntfy skip notification for {}: {}", filename, e));
                }
            }
            log_callback("----------------------------------------"); // Add separator like other skips/errors
            continue; // Skip to the next file
        }

        // Use the mutable reference directly
        log_callback(&format!("Processing: {}", filename));

        // --- Send Start Notification ---
        if let Some(topic) = &config.ntfy_topic {
            let start_message = format!("[{}]: Starting encode for: {}", hostname, filename); // Add hostname
            // Use notifier trait
            if let Err(e) = notifier.send(topic, &start_message, Some("Drapto Encode Start"), Some(3), Some("arrow_forward")) {
                log_callback(&format!("Warning: Failed to send ntfy start notification for {}: {}", filename, e));
            }
        }

        // --- Get Video Properties (including width) ---
        let video_props = match ffprobe_executor.get_video_properties(input_path) { // Use executor
            Ok(props) => props,
            Err(e) => {
                log_callback(&format!("ERROR: Failed to get video properties for {}: {}. Skipping file.", filename, e));
                continue; // Skip if we can't get essential properties
            }
        };
        let video_width = video_props.width; // Use width from fetched properties

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
        let category = if video_width >= UHD_WIDTH_THRESHOLD {
            "UHD"
        } else if video_width >= HD_WIDTH_THRESHOLD {
            "HD"
        } else {
            "SD"
        };
        // Log the combined message immediately after determining width and quality
        log_callback(&format!(
            "Detected video width: {} ({}) - CRF set to {}",
            video_width, category, quality
        ));

        // --- Crop Detection ---
        // Determine if crop should be disabled (using config default_crop_mode for now)
        // TODO: Add a specific config option like `disable_crop_detection`?
        let disable_crop = config.default_crop_mode.as_deref() == Some("off"); // Treat "off" as disable
        // Pass video_props and spawner to detect_crop
        let (crop_filter_opt, _is_hdr) = match detection::detect_crop(spawner, input_path, &video_props, disable_crop) { // Pass spawner
            Ok(result) => result,
            Err(e) => {
                log_callback(&format!("Warning: Crop detection failed for {}: {}. Proceeding without cropping.", filename, e));
                (None, false) // Default to no crop on error
            }
        };

        // --- Prepare Audio Options ---
        // Log audio info (channels, calculated bitrates)
        // We ignore the result as errors are logged internally by log_audio_info
        let _ = audio::log_audio_info(ffprobe_executor, input_path, &mut log_callback); // Pass executor

        // --- Build ffmpeg Command ---
        // Get preset value (same logic as before HandBrakeCLI removal)
        let preset_value = config.preset.or(config.default_encoder_preset).unwrap_or(6);

        // Get audio channels using the injected executor
        let audio_channels = match ffprobe_executor.get_audio_channels(input_path) {
             Ok(channels) => channels,
             Err(e) => {
                 log_callback(&format!("Warning: Error getting audio channels for ffmpeg command build: {}. Using empty list.", e));
                 vec![] // Default to empty if error
             }
         };

        // Prepare parameters for the new encode function
        let encode_params = EncodeParams {
            input_path: input_path.to_path_buf(), // Clone path for ownership
            // hw_accel field removed from EncodeParams and CoreConfig
            output_path: output_path.clone(),    // Clone path
            quality: quality.into(), // Use quality determined earlier, CONVERT u8 to u32
            preset: preset_value,
            crop_filter: crop_filter_opt, // Use crop filter determined earlier
            audio_channels, // Use fetched channels
            duration: video_props.duration_secs, // Use renamed field duration_secs
        };

        log_callback(&format!("Starting ffmpeg (via sidecar) for {}...", filename));
        // Log command is handled inside run_ffmpeg_encode now

        // --- Execute ffmpeg via sidecar ---
        // Pass the log_callback to the encode function
        let encode_result = run_ffmpeg_encode(spawner, &encode_params, &mut log_callback); // Pass the injected spawner


        // --- Handle Result ---
        match encode_result {
            Ok(()) => { // Encode succeeded
            let file_elapsed_time = file_start_time.elapsed();
            // Use ?. below, as they now return CoreResult
            let input_size = get_file_size(input_path)?; // Use crate::utils::get_file_size
            let output_size = get_file_size(&output_path)?; // Use crate::utils::get_file_size

            results.push(EncodeResult {
                filename: filename.clone(),
                duration: file_elapsed_time,
                input_size,
                output_size,
            });

            let completion_log_msg = format!("Completed: {} in {}", filename, format_duration(file_elapsed_time));
            // Use the clone for logging within the iteration
            log_callback(&completion_log_msg);

            // --- Send Success Notification ---
            if let Some(topic) = &config.ntfy_topic {
                let reduction = if input_size > 0 {
                    100u64.saturating_sub(output_size.saturating_mul(100) / input_size)
                } else {
                    0
                };
                let success_message = format!(
                    "[{hostname}]: Successfully encoded {filename} in {duration}.\nSize: {in_size} -> {out_size} (Reduced by {reduct}%)",
                    hostname = hostname, // Add hostname
                    filename = filename,
                    duration = format_duration(file_elapsed_time),
                    in_size = format_bytes(input_size),
                    out_size = format_bytes(output_size),
                    reduct = reduction
                );
                // Use notifier trait
                if let Err(e) = notifier.send(topic, &success_message, Some("Drapto Encode Success"), Some(4), Some("white_check_mark")) {
                    // Use the clone for logging within the iteration
                    log_callback(&format!("Warning: Failed to send ntfy success notification for {}: {}", filename, e));
                }
            }

            }
            Err(e) => { // Encode failed
                // run_ffmpeg_encode logs details internally via log::error
                // Log a high-level error message here using the callback
                log_callback(&format!(
                    "ERROR: ffmpeg encode failed for {}: {}. Check logs for details.",
                    filename, e
                ));
            // Consider returning a partial success / error report instead of just Vec<EncodeResult>
            // Or just log it and don't add to results, as done here.

            // --- Send Error Notification ---
            if let Some(topic) = &config.ntfy_topic {
                let error_message = format!(
                    "[{hostname}]: Error encoding {filename}: ffmpeg failed.", // Simplified error message
                    hostname = hostname,
                    filename = filename
                );
                // Use notifier trait
                if let Err(e) = notifier.send(topic, &error_message, Some("Drapto Encode Error"), Some(5), Some("x,rotating_light")) {
                    // Use the clone for logging within the iteration
                    log_callback(&format!("Warning: Failed to send ntfy error notification for {}: {}", filename, e));
                }
            }
            // Logging already done above
            }
        }
         log_callback("----------------------------------------");

    } // End loop through files

    Ok(results)
}