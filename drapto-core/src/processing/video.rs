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
use crate::notifications::send_ntfy; // Added for ntfy support
use crate::error::{CoreError, CoreResult};
use crate::external::{check_dependency, get_video_width, ffmpeg as ffmpeg_builder}; // Import the new ffmpeg module
use crate::processing::audio; // To access audio submodule
use crate::processing::detection; // Import the new detection module
use crate::utils::{format_bytes, format_duration, get_file_size}; // Added format_bytes, format_duration
use crate::EncodeResult; // Assuming EncodeResult stays in lib.rs or is re-exported from there

// Remove VecDeque as it's no longer needed for args
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Instant;


/// Processes a list of video files based on the configuration.
/// Calls the `log_callback` for logging messages.
/// Returns a list of results for successfully processed files.
pub fn process_videos<F>(
    config: &CoreConfig,
    files_to_process: &[PathBuf],
    target_filename_override: Option<PathBuf>, // <-- Add new parameter
    mut log_callback: F, // Accept by mutable reference (via Box deref)
) -> CoreResult<Vec<EncodeResult>> // <-- Keep return type
where
    F: FnMut(&str), // Remove Send + 'static bounds
{
    // --- Check Dependencies ---
    // No need to clone here, use the mutable reference directly
    log_callback("Checking for required external commands...");
    // Check for ffmpeg
    let _ffmpeg_cmd_parts = check_dependency("ffmpeg")?;
    log_callback("  [OK] ffmpeg found."); // Update log message
    // Check for ffprobe
    let _ffprobe_cmd_parts = check_dependency("ffprobe")?;
    log_callback("  [OK] ffprobe found.");
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
                 if let Err(e) = send_ntfy(topic, &ntfy_message, Some("Drapto Encode Skipped"), Some(3), Some("warning")) {
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
            if let Err(e) = send_ntfy(topic, &start_message, Some("Drapto Encode Start"), Some(3), Some("arrow_forward")) {
                log_callback(&format!("Warning: Failed to send ntfy start notification for {}: {}", filename, e));
            }
        }

        // --- Get Video Width ---
        let video_width = match get_video_width(input_path) {
             Ok(width) => width, // Get width, log combined message later
             Err(e) => {
                 log_callback(&format!("Warning: Error getting video width for {}: {}. Cannot determine resolution-specific quality. Skipping file.", filename, e));
                 continue; // Skip this file if we can't get the width
             }
         };

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
        let (crop_filter_opt, _is_hdr) = match detection::detect_crop(input_path, disable_crop) {
             Ok(result) => result,
             Err(e) => {
                 log_callback(&format!("Warning: Crop detection failed for {}: {}. Proceeding without cropping.", filename, e));
                 (None, false) // Default to no crop on error
             }
         };

        // --- Prepare Audio Options ---
        // Log audio info (channels, calculated bitrates)
        // We ignore the result as errors are logged internally by log_audio_info
        let _ = audio::log_audio_info(input_path, &mut log_callback);

        // --- Build ffmpeg Command ---
        // Get preset value (same logic as before HandBrakeCLI removal)
        let preset_value = config.preset.or(config.default_encoder_preset).unwrap_or(6);

        // Get audio channels (re-fetch for now, ideally refactor later)
        // Note: This might log channel detection warnings twice if prepare_audio_options also failed.
        let audio_channels = match crate::external::get_audio_channels(input_path) {
             Ok(channels) => channels,
             Err(e) => {
                 log_callback(&format!("Warning: Error getting audio channels for ffmpeg command build: {}. Using empty list.", e));
                 vec![] // Default to empty if error
             }
         };

        // Prepare args for the builder function
        let builder_input_args = ffmpeg_builder::FfmpegCommandArgs {
            input_path: input_path.to_path_buf(), // Clone path
            output_path: output_path.clone(),    // Clone path
            quality: quality.into(), // Use quality determined earlier, CONVERT u8 to u32
            preset: preset_value,
            crop_filter: crop_filter_opt, // Use crop filter determined earlier
            audio_channels, // Use fetched channels
        };

        // Build the actual ffmpeg arguments
        let ffmpeg_args = match ffmpeg_builder::build_ffmpeg_args(&builder_input_args) {
            Ok(args) => args,
            Err(e) => {
                // Handle error during argument building (e.g., log and skip)
                log_callback(&format!("ERROR: Failed to build ffmpeg arguments for {}: {}. Skipping file.", filename, e));
                continue;
            }
        };

        let ffmpeg_executable = "ffmpeg"; // Assuming ffmpeg is in PATH


        log_callback(&format!("Starting ffmpeg for {}...", filename));
        // Log the command correctly
        log_callback(&format!("Command: {} {}", ffmpeg_executable, ffmpeg_args.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(" ")));


        // --- Execute ffmpeg ---
        let mut child = Command::new(ffmpeg_executable)
            .args(&ffmpeg_args) // Use the generated ffmpeg args
            .stdout(Stdio::null()) // ffmpeg progress goes to stderr with "-progress -"
            .stderr(Stdio::piped()) // Capture stderr for progress and errors
            .spawn()
            .map_err(|e| CoreError::CommandStart(ffmpeg_executable.to_string(), e))?;

         // --- Stream Output to Log Callback ---
         // Combine stdout and stderr readers
         // Use raw readers, BufReader might add unwanted buffering here
         let mut stderr_reader = child.stderr.take().unwrap(); // Keep this line (was 249)

         // --- Process stderr concurrently ---

         let (tx, rx) = mpsc::channel(); // Channel to send output lines/chunks (was 254)

         // Thread to read stderr, capture lines, and send them
         let stderr_thread = thread::spawn(move || { // Use tx directly (was 300)
             let mut buffer = [0; 1024]; // Read in chunks
             let mut line_buffer = String::new(); // Buffer for incomplete lines
             let mut captured_stderr_lines = Vec::new(); // Capture lines for error reporting

             loop {
                 match stderr_reader.read(&mut buffer) {
                     Ok(0) => { // EOF
                         // Send any remaining data in the buffer as the last line
                         if !line_buffer.is_empty() {
                             captured_stderr_lines.push(line_buffer.clone()); // Capture final part
                             let _ = tx.send(line_buffer); // Use tx
                         }
                         break;
                     }
                     Ok(n) => {
                         let chunk = String::from_utf8_lossy(&buffer[..n]);
                         line_buffer.push_str(&chunk);

                         // Process complete lines within the buffer
                         while let Some(delimiter_pos) = line_buffer.find(|c| c == '\n' || c == '\r') {
                             let line_end = delimiter_pos + 1; // Include the delimiter
                             let line = line_buffer[..line_end].to_string();

                             captured_stderr_lines.push(line.clone()); // Capture the line

                             // Send the line
                             if tx.send(line).is_err() { // Use tx
                                 // Cannot send, receiver likely gone, stop thread
                                 // Return what we captured so far
                                 return captured_stderr_lines.join(""); // Return captured stderr on send error
                             }

                             // Remove the processed line from the buffer
                             line_buffer.drain(..line_end);
                         }
                         // Any remaining part stays in line_buffer for the next read
                     }
                     Err(_) => break, // Error reading
                 }
             }
             captured_stderr_lines.join("") // Return combined captured stderr lines
         });

         // --- Receive and log output from stderr thread ---
         // Receive messages (lines) until the channel is closed (when stderr_thread finishes)
         for received_line in rx {
             log_callback(&received_line); // Log received lines here
         }

         // --- Wait for stderr thread and get captured stderr ---
         // Join stderr thread and get the captured output
         let stderr_output = stderr_thread.join().expect("Stderr reading thread panicked");


         let status = child.wait().map_err(|e| CoreError::CommandWait(ffmpeg_executable.to_string(), e))?; // Use ffmpeg executable in error
 
 
        // --- Handle Result ---
        if status.success() {
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
                 if let Err(e) = send_ntfy(topic, &success_message, Some("Drapto Encode Success"), Some(4), Some("white_check_mark")) {
                    // Use the clone for logging within the iteration
                    log_callback(&format!("Warning: Failed to send ntfy success notification for {}: {}", filename, e));
                }
            }

        } else {
            // Log error including captured stderr, then continue processing other files
             // Log the error using the iteration's logger clone
             log_callback(&format!(
                "ERROR: ffmpeg failed for {} with status {}. Stderr:\n{}", // Update error message
                 filename, status, stderr_output.trim() // Use the captured stderr
             ));
             // Keep the second, simpler error log message
             log_callback(&format!(
                "ERROR: ffmpeg failed for {} with status {}. Check log for details.", // Update error message
                 filename, status
             ));
            // Consider returning a partial success / error report instead of just Vec<EncodeResult>
            // Or just log it and don't add to results, as done here.

            // --- Send Error Notification ---
            if let Some(topic) = &config.ntfy_topic {
                let error_message = format!(
                    "[{hostname}]: Error encoding {filename}: ffmpeg failed with status {status}.", // Update notification message
                     hostname = hostname, // Add hostname
                     filename = filename,
                    status = status
                );
                 if let Err(e) = send_ntfy(topic, &error_message, Some("Drapto Encode Error"), Some(5), Some("x,rotating_light")) {
                    // Use the clone for logging within the iteration
                    log_callback(&format!("Warning: Failed to send ntfy error notification for {}: {}", filename, e));
                }
            }
            // Logging already done above
        }
         log_callback("----------------------------------------");

    } // End loop through files

    Ok(results)
}