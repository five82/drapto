// drapto-core/src/processing/video.rs
//
// This module houses the main video processing orchestration logic for the
// `drapto-core` library. Its central piece is the `process_videos` function.
//
// Responsibilities of `process_videos`:
// - Takes a `CoreConfig`, a list of video files (`files_to_process`), and a
//   logging callback (`log_callback`) as input.
// - Performs initial checks for required external dependencies (`HandBrakeCLI`, `ffprobe`)
//   using functions from the `external` module.
// - Iterates through each provided video file path.
// - For each file:
//   - Determines the output path based on the configuration.
//   - Retrieves audio track channel counts using `ffprobe` via the `external` module.
//   - Calculates appropriate audio bitrates based on channel counts using the
//     internal `calculate_audio_bitrate` helper function.
//   - If `optimize_film_grain` is enabled in the config, it calls the
//     `determine_optimal_grain` function (from the `film_grain` submodule)
//     to find the best film grain setting. Otherwise, it uses the configured
//     fallback value.
//   - Constructs the full argument list for the `HandBrakeCLI` command, incorporating
//     settings from the `CoreConfig`, calculated audio bitrates, and the determined
//     film grain value.
//   - Spawns `HandBrakeCLI` as a subprocess, capturing its stdout and stderr.
//   - Uses separate threads to concurrently read stdout and stderr, sending chunks
//     of output through an MPSC channel to the main thread.
//   - The main thread receives these chunks and passes them to the `log_callback`
//     for real-time progress reporting. Stderr is also captured separately for
//     potential error messages.
//   - Waits for the `HandBrakeCLI` process to complete.
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
use crate::external::{check_dependency, get_audio_channels, get_video_width}; // Added get_video_width
use crate::utils::{format_bytes, format_duration, get_file_size}; // Added format_bytes, format_duration
use crate::EncodeResult; // Assuming EncodeResult stays in lib.rs or is re-exported from there
use crate::processing; // To access film_grain submodule
// Import specific functions needed for dependency injection
use crate::processing::film_grain::sampling::{extract_and_test_sample, get_video_duration_secs};

use std::collections::VecDeque;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Instant;


// Calculates audio bitrate based on channel count (private helper)
fn calculate_audio_bitrate(channels: u32) -> u32 {
    match channels {
        1 => 64,   // Mono
        2 => 128,  // Stereo
        6 => 256,  // 5.1
        8 => 384,  // 7.1
        _ => channels * 48, // Default fallback
    }
}

/// Processes a list of video files based on the configuration.
/// Calls the `log_callback` for logging messages.
/// Returns a list of results for successfully processed files.
pub fn process_videos<F>(
    config: &CoreConfig,
    files_to_process: &[PathBuf],
    mut log_callback: F, // Accept by mutable reference (via Box deref)
) -> CoreResult<Vec<EncodeResult>>
where
    F: FnMut(&str), // Remove Send + 'static bounds
{
    // --- Check Dependencies ---
    // No need to clone here, use the mutable reference directly
    log_callback("Checking for required external commands...");
    // Store the command parts for HandBrakeCLI
    let handbrake_cmd_parts = check_dependency("HandBrakeCLI")?;
    log_callback(&format!("  [OK] HandBrakeCLI found (using: {:?}).", handbrake_cmd_parts));
    // Assuming ffprobe is direct for now, but could use check_dependency too
    let _ffprobe_cmd_parts = check_dependency("ffprobe")?;
    log_callback("  [OK] ffprobe found.");
    log_callback("External dependency check passed.");

    // --- Get Hostname ---
    let hostname = hostname::get()
        .map(|s| s.into_string().unwrap_or_else(|_| "unknown-host-invalid-utf8".to_string()))
        .unwrap_or_else(|_| "unknown-host-error".to_string());
    log_callback(&format!("Running on host: {}", hostname));


    let mut results: Vec<EncodeResult> = Vec::new();
    // cmd_handbrake is now replaced by handbrake_cmd_parts

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

        let output_path = config.output_dir.join(&filename);

        // Use the mutable reference directly
        log_callback(&format!("Processing: {}", filename));

        // --- Send Start Notification ---
        if let Some(topic) = &config.ntfy_topic {
            let start_message = format!("[{}]: Starting encode for: {}", hostname, filename); // Add hostname
            if let Err(e) = send_ntfy(topic, &start_message, Some("Drapto Encode Start"), Some(3), Some("arrow_forward")) {
                log_callback(&format!("Warning: Failed to send ntfy start notification for {}: {}", filename, e));
            }
        }

        // --- Get Audio Info ---
        let audio_channels = match get_audio_channels(input_path) {
            Ok(channels) => {
                log_callback(&format!("Detected audio channels: {:?}", channels));
                channels
            }
            Err(e) => {
                log_callback(&format!("Warning: Error getting audio channels for {}: {}. Skipping audio bitrate options.", filename, e));
                // Continue without specific bitrates if ffprobe fails for this file
                vec![]
            }
        };

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

        // --- Determine Film Grain Value ---
        // (Film grain logic now runs *after* quality selection)
        let film_grain_value = if config.optimize_film_grain {
            log_callback(&format!(
                "Attempting to determine optimal film grain value for {}...",
                filename
            ));
            // Pass handbrake_cmd_parts to determine_optimal_grain
            // Pass a mutable reference to the clone for film grain optimization logging
            match processing::film_grain::determine_optimal_grain(input_path, config, &mut log_callback, get_video_duration_secs, extract_and_test_sample, &handbrake_cmd_parts) {
                Ok(optimal_value) => {
                    log_callback(&format!("Optimal film grain value determined: {}", optimal_value));
                    optimal_value
                }
                Err(e) => {
                    let fallback = config.film_grain_fallback_value.unwrap_or(0);
                    log_callback(&format!("Warning: Film grain optimization failed: {}. Using fallback value: {}", e, fallback));
                    fallback
                }
            }
        } else {
            config.film_grain_fallback_value.unwrap_or(0)
        };

        // --- Build HandBrakeCLI Command ---
        // TODO: Extract this logic into external/handbrake.rs as per plan
        let mut handbrake_args: VecDeque<String> = VecDeque::new();

        // --- Build HandBrakeCLI Command using Config Defaults ---

        // Fixed options (Encoder, Tune, etc.)
        handbrake_args.push_back("--encoder".to_string());
        handbrake_args.push_back("svt_av1_10bit".to_string());
        handbrake_args.push_back("--encoder-tune".to_string());
        handbrake_args.push_back("0".to_string()); // Assuming tune 0 is always desired
         // Dynamic film grain setting
         let encopts = format!("film-grain={}:film-grain-denoise=1", film_grain_value); // Use determined/fallback value
         handbrake_args.push_back("--encopts".to_string());
         handbrake_args.push_back(encopts);

        // Encoder Preset (Use default from config or fallback)
        let encoder_preset = config.default_encoder_preset.unwrap_or(6); // Fallback to 6
        handbrake_args.push_back("--encoder-preset".to_string());
        handbrake_args.push_back(encoder_preset.to_string());
        log_callback(&format!("Using encoder preset: {}", encoder_preset));

        // Quality (Use the value determined earlier based on width)
        handbrake_args.push_back("--quality".to_string());
        handbrake_args.push_back(quality.to_string());
        // Logging for quality is now done immediately after width detection

        // Crop Mode (Only add if specified in config)
        if let Some(crop_mode) = &config.default_crop_mode {
            handbrake_args.push_back("--crop-mode".to_string());
            handbrake_args.push_back(crop_mode.clone()); // Clone the string
             log_callback(&format!("Using crop mode: {}", crop_mode));
        } else {
             log_callback("Using Handbrake's default crop mode (likely 'off')");
        }


        // Other fixed options
         handbrake_args.push_back("--auto-anamorphic".to_string());
         handbrake_args.push_back("--all-subtitles".to_string());
         handbrake_args.push_back("--aencoder".to_string());
         handbrake_args.push_back("opus".to_string());
         handbrake_args.push_back("--all-audio".to_string());
         handbrake_args.push_back("--mixdown".to_string());
         handbrake_args.push_back("none".to_string());
         handbrake_args.push_back("--enable-hw-decoding".to_string());
         handbrake_args.push_back("--no-comb-detect".to_string());
         handbrake_args.push_back("--no-deinterlace".to_string());
         handbrake_args.push_back("--no-bwdif".to_string());
         handbrake_args.push_back("--no-decomb".to_string());
         handbrake_args.push_back("--no-detelecine".to_string());
         handbrake_args.push_back("--no-hqdn3d".to_string());
         handbrake_args.push_back("--no-nlmeans".to_string());
         handbrake_args.push_back("--no-chroma-smooth".to_string());
         handbrake_args.push_back("--no-unsharp".to_string());
         handbrake_args.push_back("--no-lapsharp".to_string());
         handbrake_args.push_back("--no-deblock".to_string());

         // Dynamic audio bitrate options
        let mut audio_bitrate_opts_log = String::new();
        for (index, &num_channels) in audio_channels.iter().enumerate() {
            let bitrate = calculate_audio_bitrate(num_channels); // Use local helper
            handbrake_args.push_back("--ab".to_string());
            handbrake_args.push_back(bitrate.to_string());
            let log_msg = format!(
                "Added bitrate for audio stream {} ({} channels): {}kbps",
                index, num_channels, bitrate
            );
            log_callback(&log_msg);
            audio_bitrate_opts_log.push_str(&format!(" --ab {}", bitrate));
        }
        if !audio_bitrate_opts_log.is_empty() {
            log_callback(&format!("Final audio bitrate options:{}", audio_bitrate_opts_log));
        }

         // Input and Output files
         handbrake_args.push_back("-i".to_string());
         handbrake_args.push_back(input_path.to_string_lossy().to_string());
         handbrake_args.push_back("-o".to_string());
         handbrake_args.push_back(output_path.to_string_lossy().to_string());


        // Combine the base command parts (e.g., ["flatpak", "run", "..."]) with the specific arguments
        let mut full_handbrake_args = VecDeque::from(handbrake_cmd_parts[1..].to_vec()); // Start with args like "run", "fr.handbrake..."
        full_handbrake_args.append(&mut handbrake_args); // Append the specific encode args

        let handbrake_executable = &handbrake_cmd_parts[0]; // The actual command to run (e.g., "HandBrakeCLI" or "flatpak")

        log_callback(&format!("Starting HandBrakeCLI for {}...", filename));
        // Log the command correctly, showing the executable and all arguments
        log_callback(&format!("Command: {} {}", handbrake_executable, full_handbrake_args.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(" ")));


        // --- Execute HandBrakeCLI ---
        let mut child = Command::new(handbrake_executable) // Use the determined executable
            .args(Vec::from(full_handbrake_args)) // Use the combined arguments
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()) // Capture stderr
            .spawn()
            .map_err(|e| CoreError::CommandStart(handbrake_executable.to_string(), e))?; // Use correct executable in error

         // --- Stream Output to Log Callback ---
         // Combine stdout and stderr readers
         // Use raw readers, BufReader might add unwanted buffering here
         let mut stdout_reader = child.stdout.take().unwrap();
         let mut stderr_reader = child.stderr.take().unwrap();
         // Removed initial declaration of stderr_output

         // --- Process stdout/stderr concurrently (more robust) ---

         let (tx, rx) = mpsc::channel(); // Channel to send output lines/chunks

         let stdout_tx = tx.clone();
         // Remove the clone attempt, threads will only send data
         // For now, assume logging only happens on the main thread receiving from the channel
         let stdout_thread = thread::spawn(move || {
             let mut buffer = [0; 1024]; // Read in chunks
             let mut line_buffer = String::new(); // Buffer for incomplete lines
             loop {
                 match stdout_reader.read(&mut buffer) {
                     Ok(0) => { // EOF
                         // Send any remaining data in the buffer as the last line
                         if !line_buffer.is_empty() {
                             let _ = stdout_tx.send(line_buffer);
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

                             // Send the complete line
                             // Only send the line, do not log here
                             // Send the line (might be redundant if logging handles everything, but keep for now)
                             if stdout_tx.send(line).is_err() {
                                 // Cannot send, receiver likely gone, stop thread
                                 return;
                             }

                             // Remove the processed line from the buffer
                             line_buffer.drain(..line_end);
                         }
                         // Any remaining part stays in line_buffer for the next read
                     }
                     Err(_) => break, // Error reading
                 }
             }
         });

         let stderr_tx = tx; // No need to clone tx again
         // Remove the clone attempt
         let stderr_thread = thread::spawn(move || {
             let mut buffer = [0; 1024]; // Read in chunks
             let mut line_buffer = String::new(); // Buffer for incomplete lines
             let mut captured_stderr_lines = Vec::new(); // Capture lines for error reporting

             loop {
                 match stderr_reader.read(&mut buffer) {
                     Ok(0) => { // EOF
                         // Send any remaining data in the buffer as the last line
                         if !line_buffer.is_empty() {
                             captured_stderr_lines.push(line_buffer.clone()); // Capture final part
                             let _ = stderr_tx.send(line_buffer);
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

                             // Only capture the line, do not log here
                             // Send the line (might be redundant if logging handles everything, but keep for now)
                             if stderr_tx.send(line).is_err() {
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

         // --- Receive and log output from both threads ---
         // Drop the original tx so the loop terminates when threads finish
         // drop(tx); // This was incorrect, tx is moved into stderr_thread

         // Receive messages (now guaranteed to be lines) until the channel is closed
         // Receive messages from threads and log them on the main thread
         for received_line in rx {
             log_callback(&received_line); // Log received lines here
         }

         // --- Wait for threads and get captured stderr ---
         stdout_thread.join().expect("Stdout reading thread panicked");
         // Join stderr thread and get the captured output
         let stderr_output = stderr_thread.join().expect("Stderr reading thread panicked"); // Declare and assign here


         let status = child.wait().map_err(|e| CoreError::CommandWait(handbrake_executable.to_string(), e))?; // Use correct executable in error


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
                "ERROR: HandBrakeCLI failed for {} with status {}. Stderr:\n{}",
                 filename, status, stderr_output.trim() // Use the captured stderr
             ));
             log_callback(&format!(
                "ERROR: HandBrakeCLI failed for {} with status {}. Check log for details.",
                 filename, status
             ));
            // Consider returning a partial success / error report instead of just Vec<EncodeResult>
            // Or just log it and don't add to results, as done here.

            // --- Send Error Notification ---
            if let Some(topic) = &config.ntfy_topic {
                let error_message = format!(
                    "[{hostname}]: Error encoding {filename}: HandBrakeCLI failed with status {status}.",
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