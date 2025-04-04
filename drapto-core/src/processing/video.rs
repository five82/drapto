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

use crate::config::{CoreConfig, DEFAULT_CORE_QUALITY_HD, DEFAULT_CORE_QUALITY_SD, DEFAULT_CORE_QUALITY_UHD}; // Import core defaults
use crate::error::{CoreError, CoreResult};
use crate::external::{check_dependency, get_audio_channels, get_video_width}; // Added get_video_width
use crate::utils::get_file_size;
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
    mut log_callback: F,
) -> CoreResult<Vec<EncodeResult>>
where
    F: FnMut(&str), // Closure to handle logging
{
    // --- Check Dependencies ---
    log_callback("Checking for required external commands...");
    // Store the command parts for HandBrakeCLI
    let handbrake_cmd_parts = check_dependency("HandBrakeCLI")?;
    log_callback(&format!("  [OK] HandBrakeCLI found (using: {:?}).", handbrake_cmd_parts));
    // Assuming ffprobe is direct for now, but could use check_dependency too
    let _ffprobe_cmd_parts = check_dependency("ffprobe")?;
    log_callback("  [OK] ffprobe found.");
    log_callback("External dependency check passed.");


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

        log_callback(&format!("Processing: {}", filename));

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
         let stdout_thread = thread::spawn(move || {
             let mut buffer = [0; 1024]; // Read in chunks
             loop {
                 match stdout_reader.read(&mut buffer) {
                     Ok(0) => break, // EOF
                     Ok(n) => {
                         // Send the chunk as lossy UTF-8
                         let chunk = String::from_utf8_lossy(&buffer[..n]);
                         if stdout_tx.send(chunk.to_string()).is_err() {
                             break; // Receiver disconnected
                         }
                     }
                     Err(_) => break, // Error reading
                 }
             }
         });

         let stderr_tx = tx; // No need to clone tx again
         let stderr_thread = thread::spawn(move || {
             let mut buffer = [0; 1024]; // Read in chunks
             let mut captured_stderr = String::new(); // Local stderr capture
             loop {
                 match stderr_reader.read(&mut buffer) {
                     Ok(0) => break, // EOF
                     Ok(n) => {
                         let chunk = String::from_utf8_lossy(&buffer[..n]);
                         captured_stderr.push_str(&chunk); // Capture locally
                         // Send the chunk for logging
                         if stderr_tx.send(chunk.to_string()).is_err() {
                             break; // Receiver disconnected
                         }
                     }
                     Err(_) => break, // Error reading
                 }
             }
             captured_stderr // Return captured stderr at the end
         });

         // --- Receive and log output from both threads ---
         // Drop the original tx so the loop terminates when threads finish
         // drop(tx); // This was incorrect, tx is moved into stderr_thread

         // Receive messages until the channel is closed (both threads exit)
         for received_chunk in rx {
             // Log chunks as they arrive. Note: This might interleave stdout/stderr
             // and split lines, but ensures real-time display.
             log_callback(&received_chunk);
             // We need to reconstruct stderr_output here if needed for CommandFailed error
             // For simplicity now, we'll get it from the thread join result.
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

            log_callback(&format!("Completed: {} in {:?}", filename, file_elapsed_time));

        } else {
            // Log error including captured stderr, then continue processing other files
             log_callback(&format!(
                "ERROR: HandBrakeCLI failed for {} with status {}. Stderr:\n{}",
                 filename, status, stderr_output.trim() // Use the captured stderr
             ));
            // Continue processing other files without adding this one to results.
            // Log error but continue processing other files
             log_callback(&format!(
                "ERROR: HandBrakeCLI failed for {} with status {}. Check log for details.",
                 filename, status
             ));
            // Consider returning a partial success / error report instead of just Vec<EncodeResult>
            // Or just log it and don't add to results, as done here.
        }
         log_callback("----------------------------------------");

    } // End loop through files

    Ok(results)
}