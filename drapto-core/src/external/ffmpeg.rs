// drapto-core/src/external/ffmpeg.rs
//
// This module encapsulates the logic for executing ffmpeg commands using ffmpeg-sidecar.

use crate::error::{CoreError, CoreResult};
use crate::processing::audio; // To access calculate_audio_bitrate
use crate::external::{FfmpegSpawner, FfmpegProcess}; // Imports are correct
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::{FfmpegEvent, LogLevel as FfmpegLogLevel}; // Renamed LogLevel to avoid conflict
use std::time::Instant;
use std::path::PathBuf; // Keep PathBuf, remove unused Path

/// Parameters required for running an FFmpeg encode operation.
#[derive(Debug, Clone)]
pub struct EncodeParams {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub quality: u32, // CRF value
    pub preset: u8,   // SVT-AV1 preset
    // hw_accel field removed
    pub crop_filter: Option<String>, // Optional crop filter string "crop=W:H:X:Y"
    pub audio_channels: Vec<u32>, // Detected audio channels for bitrate mapping
    pub duration: f64, // Total video duration in seconds for progress calculation
pub enable_denoise: bool, // Whether to apply the hqdn3d filter
    // Add other parameters as needed (e.g., specific audio/subtitle stream selection)
}

/// Executes an FFmpeg encode operation using ffmpeg-sidecar based on the provided parameters.
/// Uses the provided callback for logging user-facing messages and progress.
/// Accepts a generic `FfmpegSpawner` to allow for mocking.
pub fn run_ffmpeg_encode<S: FfmpegSpawner, F>(spawner: &S, params: &EncodeParams, mut log_callback: F) -> CoreResult<()>
where
    F: FnMut(&str), // Remove Send + 'static
{
    // Use log::info for internal/debug logging
    log::info!(
        "Starting FFmpeg encode for: {}",
        params.input_path.display()
    );
    log::debug!("Encode parameters: {:?}", params);

    let mut cmd = FfmpegCommand::new();

    // --- Input Arguments ---
    cmd.hide_banner(); // Equivalent to -hide_banner

    // Hardware Acceleration (Input Option - must come before input())
    // Hardware acceleration is no longer supported, only HardwareAccel::None exists.
    // No arguments needed for software decoding.
    // The match statement is removed as there's only one variant left.
    // If params.hw_accel is somehow not None (which shouldn't happen after other changes),
    // ffmpeg-sidecar will likely ignore it or error, but we rely on upstream logic
    // ensuring only HardwareAccel::None is passed.

    cmd.input(params.input_path.to_string_lossy().into_owned()); // Convert PathBuf -> String

    // --- Filters and Stream Mapping ---
    // Audio filter for channel layout workaround - use raw args
    let af_filters = vec!["aformat=channel_layouts=7.1|5.1|stereo|mono"];
    cmd.arg("-af").arg(af_filters.join(","));

    // Video filter logic
    let use_denoise = params.enable_denoise;
    let crop_filter_opt = params.crop_filter.as_deref(); // Get Option<&str>

    match (use_denoise, crop_filter_opt) {
        (true, Some(crop_filter)) => {
            // Both denoise and crop: Use filter_complex
            log::info!("Applying video filters: crop, hqdn3d");
            // Chain filters: crop first, then hqdn3d
            let filtergraph = format!("[0:v:0]{},hqdn3d[vout]", crop_filter);
            cmd.filter_complex(&filtergraph);
            cmd.arg("-map").arg("[vout]"); // Map the output of the filtergraph
        }
        (true, None) => {
            // Only denoise: Use -vf
            log::info!("Applying video filter: hqdn3d");
            // Use raw args as ffmpeg-sidecar might not have a dedicated method for -vf
            cmd.arg("-vf").arg("hqdn3d");
            cmd.arg("-map").arg("0:v:0?"); // Map the filtered video stream (optional)
        }
        (false, Some(crop_filter)) => {
            // Only crop: Use filter_complex (existing logic)
            log::info!("Applying video filter: crop");
            let filtergraph = format!("[0:v:0]{}[vout]", crop_filter);
            cmd.filter_complex(&filtergraph);
            cmd.arg("-map").arg("[vout]");
        }
        (false, None) => {
            // No video filters: Map directly (existing logic)
            log::info!("No video filters applied.");
            cmd.arg("-map").arg("0:v:0?"); // Map first video stream (optional)
        }
    }

    // Map other streams (audio, subs, metadata, chapters - remains the same)
    cmd.arg("-map").arg("0:a?");   // Map all audio streams (optional)
    cmd.arg("-map").arg("0:s?");   // Map all subtitle streams (optional)
    cmd.arg("-map_metadata").arg("0"); // Copy global metadata
    cmd.arg("-map_chapters").arg("0"); // Copy chapters

    // --- Output Arguments ---

    // Video Codec and Params
    cmd.codec_video("libsvtav1");
    cmd.pix_fmt("yuv420p10le"); // Ensure 10-bit
    cmd.arg("-crf").arg(params.quality.to_string());
    cmd.arg("-preset").arg(params.preset.to_string());
    // Set tune=3 for psychovisual optimization (svt-av1-psy)
    cmd.arg("-svtav1-params").arg("tune=3");

    // Audio Codec and Params
    cmd.codec_audio("libopus");
    for (i, &channels) in params.audio_channels.iter().enumerate() {
        let bitrate = audio::calculate_audio_bitrate(channels);
        // Use raw args for stream-specific bitrate
        cmd.arg(format!("-b:a:{}", i)).arg(format!("{}k", bitrate));
    }

    // Subtitle Codec (copy)
    cmd.codec_subtitle("copy");

    // Progress Reporting (-progress -) is handled automatically by sidecar's event loop

    // Output Path
    cmd.output(params.output_path.to_string_lossy().into_owned()); // Convert PathBuf -> String

    // Log the constructed command before spawning using Debug format
    let cmd_debug = format!("{:?}", cmd); // Format the debug string once
    log::info!(
        "Executing FFmpeg command (Debug representation):\n  {}",
        cmd_debug
    );
    // Use callback for user-facing command details
    log_callback(&format!(
        "🔧 FFmpeg command details:\n  {}",
        cmd_debug
    ));


    // --- Execution and Progress ---
    log_callback("🚀 Starting encode process..."); // Use callback
    let start_time = Instant::now(); // Record start time

    // Use the injected spawner
    let mut child = spawner.spawn(cmd)?;

    // Initialize duration from params
    let duration_secs: Option<f64> = if params.duration > 0.0 { Some(params.duration) } else { None };
    if duration_secs.is_some() {
        log::info!("Using provided duration for progress: {}", format_duration_seconds(params.duration)); // Use correct format
    } else {
        log::warn!("Video duration not provided or zero; progress percentage will not be accurate.");
    }
    let mut stderr_buffer = String::new(); // Buffer to capture stderr lines
    let mut last_reported_percent = -3.0; // Initialize to ensure the first report (near 0%) happens

    // Event loop using try_for_each, handling errors from iter() and the closure
    // Use the handle_events method from the FfmpegProcess trait
    // Pass mutable references to the closure instead of moving
    child.handle_events(|event| {
    		match event {
    			FfmpegEvent::Progress(progress) => {
                    // Duration is now initialized from params

                    // progress.time is a String "HH:MM:SS.ms" - parse it
                    let current_secs = parse_ffmpeg_time(&progress.time).unwrap_or(0.0);

                    let percent = duration_secs
                        .filter(|&d| d > 0.0)
                        .map(|d| (current_secs / d * 100.0).min(100.0)) // Ensure percent doesn't exceed 100
                        .unwrap_or(0.0); // Default to 0% if duration is unknown or zero

                    // Only report progress every 3% or at 100%
                    // Need mutable access to last_reported_percent and log_callback
                    if percent >= last_reported_percent + 3.0 || (percent >= 100.0 && last_reported_percent < 100.0) { // Remove deref (*)
                        // Calculate ETA
                        let eta_str = if let Some(total_duration) = duration_secs {
                            if progress.speed > 0.01 && total_duration > current_secs { // Avoid division by zero/small numbers and negative ETA
                                let remaining_seconds = (total_duration - current_secs) / (progress.speed as f64);
                                format_duration_seconds(remaining_seconds) // Use the new function
                            } else if percent < 100.0 { // If speed is too low but not finished
                                "??:??:??".to_string()
                            } else { // If finished or nearly finished
                                format_duration_seconds(0.0) // Use the new function
                            }
                        } else { // Duration unknown
                            "??:??:??".to_string()
                        };

                        // Calculate elapsed wall-clock time
                        let elapsed_wall_clock = start_time.elapsed().as_secs_f64();

                        // Calculate Average Encoding FPS using wall-clock time
                        let avg_encoding_fps = if elapsed_wall_clock > 0.01 { // Avoid division by zero early on
                            progress.frame as f64 / elapsed_wall_clock
                        } else {
                            0.0
                        };

                        // Use callback for progress updates
                        // Call the FnMut closure
                        log_callback(&format!(
                            "⏳ Encoding progress: {:.2}% ({} / {}), Speed: {:.2}x, Avg FPS: {:.2}, ETA: {}",
                            percent,
                            format_duration_seconds(current_secs),
                            duration_secs.map_or("??:??:??".to_string(), format_duration_seconds), // Use rounded format for video duration
                            progress.speed,
                            avg_encoding_fps, // Use calculated average *encoding* FPS
                            eta_str // Add ETA here
                        )); // <-- Added missing closing parenthesis and semicolon
                        log::debug!(
                            "Progress: frame={}, avg_encoding_fps={:.2}, time={}, bitrate={:.2}kbits/s, speed={:.2}x, size={}kB, percent={:.2}%, ETA={}", // Changed label in debug
                            progress.frame,
                            avg_encoding_fps, // Use calculated average *encoding* FPS in debug
                            format_duration(current_secs), // Keep original video time for debug log context
                            progress.bitrate_kbps,
                            progress.speed,
                            progress.size_kb,
                            percent,
                            eta_str // Also add ETA to debug log
                        );
                        last_reported_percent = percent; // Update last reported percentage (remove deref *)
                    }
                }
            FfmpegEvent::Error(err_str) => {
                // Error log line from ffmpeg stderr
                log::error!("ffmpeg stderr error: {}", err_str); // Log via macro
                // Need mutable access to stderr_buffer
                stderr_buffer.push_str(&err_str); // Also capture to buffer
                stderr_buffer.push('\n');
            }
            // FfmpegEvent::Warning does not exist
            FfmpegEvent::Log(level, log_str) => {
                // Other log lines from ffmpeg stderr, mapped to log levels
                // Pass level by reference to avoid move
                let rust_log_level = map_ffmpeg_log_level(&level);
                log::log!(target: "ffmpeg_log", rust_log_level, "{}", log_str); // Log via macro

                // Use callback for SVT-AV1 info lines
                if log_str.starts_with("Svt[info]:") {
                    // Call the FnMut closure
                    log_callback(&log_str);
                }

                // Capture ALL log messages to buffer to ensure we don't miss the error
                // Use Debug format for LogLevel as it doesn't implement Display
                // Need mutable access to stderr_buffer
                stderr_buffer.push_str(&format!("[{:?}] {}", level, log_str)); // Add level prefix
                stderr_buffer.push('\n');
            }
            FfmpegEvent::ParsedOutput(parsed) => {
                 // Structured info parsed from stderr (like stream maps, headers)
                 log::debug!("ffmpeg parsed output: {:?}", parsed);
                 // Duration is now passed via params, no need to extract here.
            }
            // FfmpegEvent::Input / FfmpegEvent::OutputFrame / FfmpegEvent::OutputStream / FfmpegEvent::Done
            // are less relevant when just running a command without piping data in/out.
            _ => {} // Ignore other event types for now
        }
        Ok(()) // Continue iteration
       })?; // Propagate errors from handle_events

    // After iterating through events, explicitly wait for the process to exit
    // and check its status code.
    let status = child.wait()?; // Use the wait method from the FfmpegProcess trait

    // Check if the iteration itself encountered an error (handled by `?` above)
    // Now check the final exit status

    if status.success() {
        log_callback(&format!("✅ Encode finished successfully for {}", params.output_path.display())); // Use callback
        log::info!("FFmpeg encode finished successfully for: {}", params.output_path.display()); // Keep internal log
        Ok(())
    } else {
        // Log the failure status and include the captured stderr buffer
        let error_message = format!(
            "FFmpeg process exited with non-zero status ({:?}). Stderr output:\n{}",
            status.code(),
            stderr_buffer.trim()
        );
        log::error!(
            "FFmpeg encode failed for {}: {}",
            params.input_path.display(),
            error_message
        );
        Err(CoreError::CommandFailed(
            "ffmpeg (sidecar)".to_string(),
            status,
            error_message, // Include captured stderr in the error
        ))
    }
}

/// Helper to format seconds into HH:MM:SS.ms
fn format_duration(total_seconds: f64) -> String {
    if total_seconds < 0.0 || !total_seconds.is_finite() {
        return "??:??:??".to_string();
    }
    let seconds_int = total_seconds.trunc() as u64;
    let millis = (total_seconds.fract() * 1000.0).round() as u32;
    let hours = seconds_int / 3600;
    let minutes = (seconds_int % 3600) / 60;
    let seconds = seconds_int % 60;
    format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
}
/// Helper to format seconds into HH:MM:SS, rounded to the nearest second.
    fn format_duration_seconds(total_seconds: f64) -> String {
        if total_seconds < 0.0 || !total_seconds.is_finite() {
            return "??:??:??".to_string();
        }
        // Round to the nearest second
        let rounded_seconds = total_seconds.round() as u64;
        let hours = rounded_seconds / 3600;
        let minutes = (rounded_seconds % 3600) / 60;
        let seconds = rounded_seconds % 60;
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    }

/// Helper function to parse ffmpeg time string "HH:MM:SS.ms" into seconds (f64)
fn parse_ffmpeg_time(time_str: &str) -> Result<f64, &'static str> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 3 {
        return Err("Invalid time format: Expected HH:MM:SS.ms");
    }
    let hours: f64 = parts[0].parse().map_err(|_| "Failed to parse hours")?;
    let minutes: f64 = parts[1].parse().map_err(|_| "Failed to parse minutes")?;
    let sec_ms: Vec<&str> = parts[2].split('.').collect();
    if sec_ms.len() != 2 {
         // Handle cases like "00:00:00" without milliseconds
         if sec_ms.len() == 1 {
             let seconds: f64 = sec_ms[0].parse().map_err(|_| "Failed to parse seconds")?;
             return Ok(hours * 3600.0 + minutes * 60.0 + seconds);
         }
        return Err("Invalid seconds/milliseconds format");
    }
    let seconds: f64 = sec_ms[0].parse().map_err(|_| "Failed to parse seconds")?;
    // Ensure milliseconds part has consistent length (e.g., pad with zeros if needed, or truncate)
    let ms_str = format!("{:0<3}", sec_ms[1]); // Pad with zeros to 3 digits
    let milliseconds: f64 = ms_str[..3].parse().map_err(|_| "Failed to parse milliseconds")?;

    Ok(hours * 3600.0 + minutes * 60.0 + seconds + milliseconds / 1000.0)
}

// Removed unused function extract_duration_from_log

/// Helper to map ffmpeg log levels to Rust log levels
fn map_ffmpeg_log_level(level: &FfmpegLogLevel) -> log::Level { // Accept by reference
    match level { // Match on reference
        // Map based on available variants in ffmpeg-sidecar v2.0.5 LogLevel
        FfmpegLogLevel::Unknown => log::Level::Trace, // Treat Unknown as Trace
        FfmpegLogLevel::Info => log::Level::Info,
        FfmpegLogLevel::Warning => log::Level::Warn,
        FfmpegLogLevel::Error => log::Level::Error,
        // Handle potential future variants or unexpected values gracefully
        _ => log::Level::Debug, // Default to Debug for any other variants (future-proofing)
    }
}


// No tests for now, as testing requires mocking ffmpeg execution.
// The previous tests were for argument building, which is now handled by ffmpeg-sidecar.