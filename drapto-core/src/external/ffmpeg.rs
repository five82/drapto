// drapto-core/src/external/ffmpeg.rs
//
// This module encapsulates the logic for executing ffmpeg commands using ffmpeg-sidecar.

use crate::error::{CoreError, CoreResult};
use crate::processing::audio; // To access calculate_audio_bitrate
use crate::processing::detection::grain_analysis::GrainLevel; // Import GrainLevel
use crate::external::{FfmpegSpawner, FfmpegProcess}; // Remove is_macos import
use crate::progress::{ProgressCallback, ProgressEvent, LogLevel};
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::{FfmpegEvent, LogLevel as FfmpegLogLevel}; // Renamed LogLevel to avoid conflict
use colored::Colorize;
use std::time::Instant;
use std::path::PathBuf; // Keep PathBuf, remove unused Path
use log::{info, warn, error, debug, log}; // Import log macros

/// Parameters required for running an FFmpeg encode operation.
#[derive(Debug, Clone)]
pub struct EncodeParams {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub quality: u32, // CRF value
    pub preset: u8,   // SVT-AV1 preset
    /// Whether to use hardware acceleration for decoding (when available)
    pub use_hw_decode: bool,
    pub crop_filter: Option<String>, // Optional crop filter string "crop=W:H:X:Y"
    pub audio_channels: Vec<u32>, // Detected audio channels for bitrate mapping
    pub duration: f64, // Total video duration in seconds for progress calculation
    /// The final hqdn3d parameters determined by analysis (used if override is not provided).
    pub hqdn3d_params: Option<String>,
    // Add other parameters as needed (e.g., specific audio/subtitle stream selection)
}

/// Builds the list of FFmpeg arguments based on EncodeParams, excluding input/output paths.
/// Allows overriding the hqdn3d filter for testing purposes (e.g., grain analysis).
pub fn build_ffmpeg_args(
    params: &EncodeParams,
    hqdn3d_override: Option<&str>, // Added override parameter
    disable_audio: bool, // Added flag to disable audio args
    is_grain_analysis_sample: bool, // Flag to simplify args for grain samples
) -> CoreResult<Vec<String>> {
    let mut args: Vec<String> = Vec::new();

    // --- Input Arguments ---
    args.push("-hide_banner".to_string());

    // Hardware acceleration is now handled separately in run_ffmpeg_encode
    // to ensure it's placed before the input file

    // --- Filters and Stream Mapping ---
    // Conditionally add audio filter
    if !is_grain_analysis_sample && !disable_audio {
        // Audio filter for channel layout workaround (only if not grain sample and audio not disabled)
        args.push("-af".to_string());
        args.push("aformat=channel_layouts=7.1|5.1|stereo|mono".to_string());
    }

    // Video filter logic - Use override if provided, otherwise use params
    let hqdn3d_to_use = hqdn3d_override.or(params.hqdn3d_params.as_deref());
    let crop_filter_opt = params.crop_filter.as_deref();

    // --- Film Grain Synthesis Logic ---
    let film_grain_value = if let Some(denoise_params) = hqdn3d_to_use {
        map_hqdn3d_to_film_grain(denoise_params)
    } else {
        0 // No denoise = no synthetic grain
    };

    // Build filter string and determine if we need filter_complex
    let mut filters = Vec::new();
    let mut use_filter_complex = false;

    // Add crop filter if present
    if let Some(crop_str) = crop_filter_opt {
        filters.push(crop_str.to_string());
        use_filter_complex = true; // Crop requires filter_complex
    }

    // Add denoise filter if present
    if let Some(hqdn3d_str) = hqdn3d_to_use {
        filters.push(hqdn3d_str.to_string());
    }

    // Log filters being applied
    if !filters.is_empty() {
        let filters_str = filters.join(", ");
        if !is_grain_analysis_sample {
            info!("Applying video filters: {}", filters_str);
        } else {
            debug!("Applying video filters (grain sample): {}", filters_str);
        }
    } else if !is_grain_analysis_sample {
        info!("No video filters applied.");
    }

    // Apply filters
    if !filters.is_empty() {
        if use_filter_complex {
            // Use filter_complex for crop or multiple filters
            let filtergraph = format!("[0:v:0]{}[vout]", filters.join(","));
            args.push("-filter_complex".to_string());
            args.push(filtergraph);
            args.push("-map".to_string());
            args.push("[vout]".to_string());
        } else {
            // Use -vf for simple filters
            args.push("-vf".to_string());
            args.push(filters.join(","));
            args.push("-map".to_string());
            args.push("0:v:0".to_string());
        }
    } else {
        // No filters, just map video stream
        args.push("-map".to_string());
        args.push("0:v:0".to_string());
    }

    // Map other streams (conditionally map audio/subtitles)
    if !is_grain_analysis_sample {
        if !disable_audio {
            args.push("-map".to_string());
            args.push("0:a".to_string()); // Made mapping mandatory
        }
        args.push("-map_metadata".to_string());
        args.push("0".to_string());
        args.push("-map_chapters".to_string());
        args.push("0".to_string());
    }
    // Note: Video stream mapping is handled within the filter logic above

    // --- Output Arguments ---

    // Video Codec and Params
    args.push("-c:v".to_string());
    args.push("libsvtav1".to_string());
    args.push("-pix_fmt".to_string());
    args.push("yuv420p10le".to_string());
    args.push("-crf".to_string());
    args.push(params.quality.to_string());
    args.push("-preset".to_string());
    args.push(params.preset.to_string());
    args.push("-svtav1-params".to_string());
    if film_grain_value > 0 {
        args.push(format!("tune=3:film-grain={}:film-grain-denoise=0", film_grain_value));
        if !is_grain_analysis_sample { // Log only for main encode
            info!("Applying film grain synthesis: level={}", film_grain_value);
        } else {
            debug!("Applying film grain synthesis (grain sample): level={}", film_grain_value);
        }
    } else {
        args.push("tune=3".to_string()); // Keep original if no film grain
        if !is_grain_analysis_sample { // Log only for main encode
             info!("No film grain synthesis applied (denoise level is None or 0).");
        }
    }

    // Audio Codec and Params (conditional)
    if !is_grain_analysis_sample && !disable_audio {
        args.push("-c:a".to_string());
        args.push("libopus".to_string());
        for (i, &channels) in params.audio_channels.iter().enumerate() {
            let bitrate = audio::calculate_audio_bitrate(channels);
            args.push(format!("-b:a:{}", i));
            args.push(format!("{}k", bitrate));
        }
    } else {
        // Explicitly disable audio if grain sample or if flag is set
        args.push("-an".to_string());
    }

    // Subtitles are explicitly excluded, no arguments needed.


    // Progress Reporting (-progress -) is handled by sidecar

    Ok(args)
}


/// Executes an FFmpeg encode operation using ffmpeg-sidecar based on the provided parameters.
/// Uses the standard `log` facade for logging and the progress callback for user-facing output.
/// Accepts a generic `FfmpegSpawner` to allow for mocking.
pub fn run_ffmpeg_encode<S: FfmpegSpawner, C: ProgressCallback>(
    spawner: &S,
    params: &EncodeParams,
    disable_audio: bool,
    is_grain_analysis_sample: bool, // Flag to control logging verbosity
    _grain_level_being_tested: Option<GrainLevel>,
    progress_callback: &C,
) -> CoreResult<()> {
    // Log start differently based on context
    if is_grain_analysis_sample {
        // Less verbose for grain samples
        // Extract filename for logging
        let filename_cow = params.input_path
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_else(|| params.input_path.to_string_lossy());
        debug!(
            "Starting grain sample FFmpeg encode for: {}",
            filename_cow // Use the extracted filename
        );
    } else {
        // Standard verbose logging for main encode
        // Extract filename for logging using to_string_lossy for consistent Cow<'_, str> type
        let filename_cow = params.input_path
            .file_name()
            .map(|name| name.to_string_lossy()) // Returns Cow<'_, str>
            .unwrap_or_else(|| params.input_path.to_string_lossy()); // Also returns Cow<'_, str>
        info!(
            "Starting FFmpeg encode for: {}",
            filename_cow.yellow() // Use the Cow (implicitly derefs to &str)
        );
    }

    // Log output path for main encodes, or for grain samples only if debug is enabled.
    if !is_grain_analysis_sample || log::log_enabled!(log::Level::Debug) {
        info!(
            "  Output: {}",
            params.output_path.display()
        );

        // Log hardware acceleration status for main encodes
        if params.use_hw_decode {
            log_hardware_acceleration_status(progress_callback);
        }
    }

    debug!("Encode parameters: {:?}", params);

    // Build other arguments using the helper function, passing flags down
    let ffmpeg_args = build_ffmpeg_args(params, None, disable_audio, is_grain_analysis_sample)?;

    // Create the command and set input/output/args
    // Use mutable command object and sequential calls
    let mut cmd = FfmpegCommand::new();

    // Add hardware acceleration options BEFORE the input
    let hw_accel_added = add_hardware_acceleration_to_command(&mut cmd, params.use_hw_decode, is_grain_analysis_sample);

    // Log hardware acceleration status if it was added
    if hw_accel_added {
        progress_callback.on_progress(ProgressEvent::LogMessage {
            message: "Using VideoToolbox hardware decoding".to_string(),
            level: LogLevel::Info,
        });
    }

    cmd.input(params.input_path.to_string_lossy());
    cmd.args(ffmpeg_args.iter().map(|s| s.as_str())); // Add the built arguments
    cmd.output(params.output_path.to_string_lossy());

    // Log the constructed command before spawning using Debug format
    let cmd_debug = format!("{:?}", cmd); // Log the final command state
    // Conditionally log user-facing command details
    if !is_grain_analysis_sample {
        info!(
            "🔧 FFmpeg command details:\n  {}",
            cmd_debug
        );
    } else {
        // Log command details at debug level for grain samples
        debug!("🔧 FFmpeg command (grain sample):\n  {}", cmd_debug);
    }


    // --- Execution and Progress ---
    // Log start differently based on context
    // Log start at info level only for main encode
    if !is_grain_analysis_sample {
        info!("🚀 Starting encode process...");
    } else {
        debug!("🚀 Starting grain sample encode..."); // Use debug for grain samples
    }
    let start_time = Instant::now();

    // Use the injected spawner
    // Pass the owned cmd by value, matching the trait signature
    let mut child = spawner.spawn(cmd)?;

    // Initialize duration from params
    let duration_secs: Option<f64> = if params.duration > 0.0 { Some(params.duration) } else { None };
    if duration_secs.is_some() {
        // Log duration at info level only for main encode
        if !is_grain_analysis_sample {
            info!("Using provided duration for progress: {}", format_duration_seconds(params.duration));
        } else {
            debug!("Using provided duration for progress (grain sample): {}", format_duration_seconds(params.duration));
        }
    } else {
        warn!("Video duration not provided or zero; progress percentage will not be accurate.");
    }
    let mut stderr_buffer = String::new();
    let mut last_reported_percent = -3.0;

    // Event loop using handle_events
    child.handle_events(|event| {
            match event {
                FfmpegEvent::Progress(progress) => {
                    let current_secs = parse_ffmpeg_time(&progress.time).unwrap_or(0.0);
                    let percent = duration_secs
                        .filter(|&d| d > 0.0)
                        .map(|d| (current_secs / d * 100.0).min(100.0))
                        .unwrap_or(0.0);

                    // Only report progress at certain intervals or at 100%
                    if percent >= last_reported_percent + 3.0 || (percent >= 100.0 && last_reported_percent < 100.0) {
                        // Calculate ETA
                        let eta_seconds = calculate_eta(duration_secs, current_secs, progress.speed);
                        let eta_str = format_duration_seconds(eta_seconds);

                        // Calculate average encoding FPS
                        let elapsed_wall_clock = start_time.elapsed().as_secs_f64();
                        let avg_encoding_fps = if elapsed_wall_clock > 0.01 {
                            progress.frame as f64 / elapsed_wall_clock
                        } else {
                            0.0
                        };

                        // Report progress for main encodes (not grain analysis samples)
                        if !is_grain_analysis_sample {
                            // Use progress callback for user-facing output
                            progress_callback.on_progress(ProgressEvent::EncodeProgress {
                                percent: percent as f32,
                                current_secs,
                                total_secs: duration_secs.unwrap_or(0.0),
                                speed: progress.speed,
                                fps: avg_encoding_fps as f32,
                                eta: std::time::Duration::from_secs(eta_seconds as u64),
                            });

                            // Log to debug level to avoid duplication
                            debug!(
                                "Encoding progress: {:.2}% ({} / {}), Speed: {:.2}x, Avg FPS: {:.2}, ETA: {}",
                                percent,
                                format_duration_seconds(current_secs),
                                duration_secs.map_or("??:??:??".to_string(), format_duration_seconds),
                                progress.speed,
                                avg_encoding_fps,
                                eta_str
                            );
                        }

                        // Always log detailed debug progress regardless of sample type
                        debug!(
                            "Progress ({}): frame={}, fps={:.2}, time={}, bitrate={:.2}kbits/s, speed={:.2}x, size={}kB, percent={:.2}%, ETA={}",
                            if is_grain_analysis_sample { "grain sample" } else { "main encode" },
                            progress.frame,
                            avg_encoding_fps,
                            format_duration_seconds(current_secs),
                            progress.bitrate_kbps,
                            progress.speed,
                            progress.size_kb,
                            percent,
                            eta_str
                        );

                        last_reported_percent = percent;
                    }
                }
            FfmpegEvent::Error(err_str) => {
                // Filter out "No streams found" errors from logs
                // This is a non-fatal FFmpeg error that appears frequently during both
                // grain analysis and main encode, creating noise in the logs.
                // We filter it here to prevent duplicate error messages while still
                // maintaining proper error handling.
                if !err_str.contains("No streams found") {
                    // Only log errors that aren't "No streams found"
                    error!("ffmpeg stderr error: {}", err_str);
                }

                // Always capture errors in the buffer for later processing
                // even if we don't log them, so error handling still works.
                // This ensures the error is still properly propagated as a
                // CoreError::NoStreamsFound when needed.
                stderr_buffer.push_str(&err_str);
                stderr_buffer.push('\n');
            }
            FfmpegEvent::Log(level, log_str) => {
                let rust_log_level = map_ffmpeg_log_level(&level);
                log!(target: "ffmpeg_log", rust_log_level, "{}", log_str);

                if log_str.starts_with("Svt[info]:") {
                    // Conditionally log SVT info to info level
                    if !is_grain_analysis_sample {
                        info!("{}", log_str);
                    } else {
                        // Log SVT info at debug level for grain samples
                        debug!("SVT Info (grain sample): {}", log_str);
                    }
                }

                stderr_buffer.push_str(&format!("[{:?}] {}", level, log_str));
                stderr_buffer.push('\n');
            }
            FfmpegEvent::ParsedOutput(parsed) => {
                 log::debug!("ffmpeg parsed output: {:?}", parsed);
            }
            _ => {}
        }
        Ok(())
       })?;

    // Wait for process exit
    let status = child.wait()?;

    // Extract filename for logging
    let filename_cow = params.input_path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| params.input_path.to_string_lossy());

    if status.success() {
        // Log success based on context
        if is_grain_analysis_sample {
            debug!("✅ Grain sample encode finished successfully for {}", filename_cow);
        } else {
            info!("✅ Encode finished successfully for {}", filename_cow);
        }
        Ok(())
    } else {
        let error_message = format!(
            "FFmpeg process exited with non-zero status ({:?}). Stderr output:\n{}",
            status.code(),
            stderr_buffer.trim()
        );

        // Log error based on context
        if is_grain_analysis_sample {
            error!("❌ Grain sample encode failed for {}: {}", filename_cow, error_message);
        } else {
            error!("FFmpeg encode failed for {}: {}", filename_cow, error_message);
        }

        // Create a more specific error type based on stderr content
        if stderr_buffer.contains("No streams found") {
            // Handle "No streams found" as a specific error type
            // Note: We filter the logging of this error above, but still
            // propagate it properly here for correct error handling
            Err(CoreError::NoStreamsFound(filename_cow.to_string()))
        } else {
            Err(CoreError::CommandFailed(
                "ffmpeg (sidecar)".to_string(),
                status,
                error_message,
            ))
        }
    }
}

/// Helper to format seconds into HH:MM:SS, rounded to the nearest second.
    fn format_duration_seconds(total_seconds: f64) -> String {
        if total_seconds < 0.0 || !total_seconds.is_finite() {
            return "??:??:??".to_string();
        }
        let rounded_seconds = total_seconds.round() as u64;
        let hours = rounded_seconds / 3600;
        let minutes = (rounded_seconds % 3600) / 60;
        let seconds = rounded_seconds % 60;
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    }

/// Maps a hqdn3d parameter set to the corresponding SVT-AV1 film_grain value.
/// Handles both standard and refined/interpolated parameter sets.
///
/// This function uses a more direct mapping between denoising strength and film grain
/// synthesis values, with a continuous scale that provides better granularity.
fn map_hqdn3d_to_film_grain(hqdn3d_params: &str) -> u8 {
    // No denoising = no film grain synthesis
    if hqdn3d_params.is_empty() {
        return 0;
    }

    // Fixed mapping for standard levels (for optimization)
    for (params, film_grain) in &[
        ("hqdn3d=0.5:0.3:3:3", 4),  // VeryLight
        ("hqdn3d=1:0.7:4:4", 8),    // Light
        ("hqdn3d=1.5:1.0:6:6", 12), // Moderate
        ("hqdn3d=2:1.3:8:8", 16),   // Elevated
    ] {
        // Exact match for standard levels
        if hqdn3d_params == *params {
            return *film_grain;
        }
    }

    // For interpolated/custom parameter sets, extract the luma spatial strength
    // which is the most indicative parameter for denoising intensity
    let luma_spatial = parse_hqdn3d_first_param(hqdn3d_params);

    // Map the luma spatial value (0.0-2.0+) to film grain value (0-16)
    // using a more direct and granular mapping

    // No denoising = no grain synthesis
    if luma_spatial <= 0.1 {
        return 0;
    }

    // Use a square-root scale to reduce bias against higher grain values
    // This helps prevent the function from selecting overly low grain values
    // when the source video benefits from preserving more texture
    let adjusted_value = (luma_spatial * 8.0).sqrt() * 8.0;

    // Round to nearest integer and cap at 16
    let film_grain_value = adjusted_value.round() as u8;
    film_grain_value.min(16)
}

/// Helper function to extract the first parameter (luma_spatial) from hqdn3d string
fn parse_hqdn3d_first_param(params: &str) -> f32 {
    if let Some(suffix) = params.strip_prefix("hqdn3d=") {
        if let Some(index) = suffix.find(':') {
            let first_param = &suffix[0..index];
            return first_param.parse::<f32>().unwrap_or(0.0);
        }
    }
    0.0 // Default fallback value if parsing fails or format is unexpected
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
         if sec_ms.len() == 1 {
             let seconds: f64 = sec_ms[0].parse().map_err(|_| "Failed to parse seconds")?;
             return Ok(hours * 3600.0 + minutes * 60.0 + seconds);
         }
        return Err("Invalid seconds/milliseconds format");
    }
    let seconds: f64 = sec_ms[0].parse().map_err(|_| "Failed to parse seconds")?;
    let ms_str = format!("{:0<3}", sec_ms[1]);
    let milliseconds: f64 = ms_str[..3].parse().map_err(|_| "Failed to parse milliseconds")?;

    Ok(hours * 3600.0 + minutes * 60.0 + seconds + milliseconds / 1000.0)
}

/// Helper to map ffmpeg log levels to Rust log levels
fn map_ffmpeg_log_level(level: &FfmpegLogLevel) -> log::Level {
    match level {
        FfmpegLogLevel::Unknown => log::Level::Trace,
        FfmpegLogLevel::Info => log::Level::Info,
        FfmpegLogLevel::Warning => log::Level::Warn,
        FfmpegLogLevel::Error => log::Level::Error,
        _ => log::Level::Debug,
    }
}

// Helper function to calculate ETA in seconds
fn calculate_eta(duration_secs: Option<f64>, current_secs: f64, speed: f32) -> f64 {
    if let Some(total_duration) = duration_secs {
        if speed > 0.01 && total_duration > current_secs {
            (total_duration - current_secs) / (speed as f64)
        } else {
            0.0
        }
    } else {
        0.0
    }
}

// Helper function to check if hardware acceleration is available
fn is_hardware_acceleration_available() -> bool {
    std::env::consts::OS == "macos"
}

/// Adds hardware acceleration options to an FFmpeg command.
///
/// IMPORTANT: This must be called BEFORE adding the input file to the command.
///
/// # Arguments
///
/// * `cmd` - The FFmpeg command to add hardware acceleration options to
/// * `use_hw_decode` - Whether to use hardware acceleration
/// * `is_grain_analysis_sample` - Whether this is a grain analysis sample (hardware acceleration is disabled for grain analysis)
///
/// # Returns
///
/// * `bool` - Whether hardware acceleration was added
pub fn add_hardware_acceleration_to_command(
    cmd: &mut FfmpegCommand,
    use_hw_decode: bool,
    is_grain_analysis_sample: bool,
) -> bool {
    let hw_accel_available = is_hardware_acceleration_available();

    if use_hw_decode && hw_accel_available && !is_grain_analysis_sample {
        cmd.arg("-hwaccel");
        cmd.arg("videotoolbox");
        return true;
    }

    false
}

// Helper function to log hardware acceleration status
fn log_hardware_acceleration_status<C: ProgressCallback>(progress_callback: &C) {
    let hw_accel_available = is_hardware_acceleration_available();

    if hw_accel_available {
        progress_callback.on_progress(ProgressEvent::LogMessage {
            message: "VideoToolbox hardware decoding enabled".to_string(),
            level: LogLevel::Info,
        });
    } else {
        progress_callback.on_progress(ProgressEvent::LogMessage {
            message: "Software decoding (hardware acceleration not available on this platform)".to_string(),
            level: LogLevel::Info,
        });
    }
}

// No tests for now, as testing requires mocking ffmpeg execution.