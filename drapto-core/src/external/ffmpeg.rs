// ============================================================================
// drapto-core/src/external/ffmpeg.rs
// ============================================================================
//
// FFMPEG INTEGRATION: FFmpeg Command Building and Execution
//
// This module encapsulates the logic for executing ffmpeg commands using 
// ffmpeg-sidecar. It handles building complex ffmpeg command lines with 
// appropriate arguments for video and audio encoding, progress reporting,
// and error handling.
//
// KEY COMPONENTS:
// - EncodeParams: Structure defining encoding configuration parameters
// - build_ffmpeg_args: Constructs command line arguments for ffmpeg
// - run_ffmpeg_encode: Executes and monitors the encoding process
// - Film grain synthesis mapping from denoise parameters
// - Progress reporting with real-time updates
//
// ARCHITECTURE:
// The module uses dependency injection for the FFmpeg spawner, allowing for
// testing without actual ffmpeg execution. It communicates with the progress
// reporting system to provide user feedback on encoding status.
//
// AI-ASSISTANT-INFO: FFmpeg command generation and execution for encoding

// ---- Internal crate imports ----
use crate::error::{CoreError, CoreResult, command_failed_error};
use crate::external::{FfmpegProcess, FfmpegSpawner};
use crate::hardware_accel::add_hardware_acceleration_to_command;
use crate::processing::audio; // To access calculate_audio_bitrate
use crate::processing::detection::grain_analysis::GrainLevel; 
use crate::progress_reporting::{report_encode_error, report_encode_progress, report_encode_start};

// ---- External crate imports ----
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::{FfmpegEvent, LogLevel as FfmpegLogLevel}; // Renamed LogLevel to avoid conflict
use log::{debug, error, log, trace, warn};

// ---- Standard library imports ----
use std::path::PathBuf;
use std::time::Instant;

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
    pub audio_channels: Vec<u32>,    // Detected audio channels for bitrate mapping
    pub duration: f64,               // Total video duration in seconds for progress calculation
    /// The final hqdn3d parameters determined by analysis (used if override is not provided).
    pub hqdn3d_params: Option<String>,
    // Add other parameters as needed (e.g., specific audio/subtitle stream selection)
}

/// Builds the list of FFmpeg arguments based on EncodeParams, excluding input/output paths.
///
/// This function constructs a complete set of FFmpeg command-line arguments for
/// video encoding with libsvtav1 and audio encoding with libopus. It supports
/// film grain synthesis, filtering, and hardware-accelerated decoding.
///
/// # Arguments
///
/// * `params` - Encoding parameters, including quality, preset, and filters
/// * `hqdn3d_override` - Optional override for the noise reduction filter parameters
/// * `disable_audio` - Whether to disable audio encoding
/// * `is_grain_analysis_sample` - Whether this is for grain analysis (simplified arguments)
///
/// # Returns
///
/// * `CoreResult<Vec<String>>` - The constructed FFmpeg arguments or error
pub fn build_ffmpeg_args(
    params: &EncodeParams,
    hqdn3d_override: Option<&str>,  // Added override parameter
    disable_audio: bool,            // Added flag to disable audio args
    is_grain_analysis_sample: bool, // Flag to simplify args for grain samples
) -> CoreResult<Vec<String>> {
    let mut args: Vec<String> = Vec::new();

    // --- Input Arguments ---
    args.push("-hide_banner".to_string());

    // --- Filters and Stream Mapping ---
    // Conditionally add audio filter
    if !is_grain_analysis_sample && !disable_audio {
        // Audio filter for channel layout workaround (only if not grain sample and audio not disabled)
        args.push("-af".to_string());
        args.push("aformat=channel_layouts=7.1|5.1|stereo|mono".to_string());
    }

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

    if !filters.is_empty() {
        let filters_str = filters.join(", ");
        crate::progress_reporting::report_video_filters(&filters_str, is_grain_analysis_sample);
        if is_grain_analysis_sample {
            debug!("Applying video filters (grain sample): {}", filters_str);
        }
    } else {
        crate::progress_reporting::report_video_filters("", is_grain_analysis_sample);
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
        args.push(format!(
            "tune=3:film-grain={}:film-grain-denoise=0",
            film_grain_value
        ));

        crate::progress_reporting::report_film_grain(
            Some(film_grain_value),
            is_grain_analysis_sample,
        );

        if is_grain_analysis_sample {
            debug!(
                "Applying film grain synthesis (grain sample): level={}",
                film_grain_value
            );
        }
    } else {
        args.push("tune=3".to_string());

        crate::progress_reporting::report_film_grain(None, is_grain_analysis_sample);
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


    Ok(args)
}

/// Executes an FFmpeg encode operation using the provided spawner and parameters.
///
/// This function handles the complete FFmpeg encoding process lifecycle, including:
/// - Constructing and executing the FFmpeg command
/// - Monitoring and reporting progress during encoding
/// - Processing and filtering FFmpeg output and error messages
/// - Determining encoding success or failure
///
/// The function uses dependency injection through the `FfmpegSpawner` trait to allow
/// for testing without actually running FFmpeg processes.
///
/// # Arguments
///
/// * `spawner` - The FFmpeg process spawner implementation to use
/// * `params` - Encoding parameters for this operation
/// * `disable_audio` - Whether to disable audio in the output
/// * `is_grain_analysis_sample` - Whether this is a grain analysis sample encode
/// * `_grain_level_being_tested` - Optional grain level for analysis runs
///
/// # Returns
///
/// * `CoreResult<()>` - Success or error with detailed information
pub fn run_ffmpeg_encode<S: FfmpegSpawner>(
    spawner: &S,
    params: &EncodeParams,
    disable_audio: bool,
    is_grain_analysis_sample: bool, // Flag to control logging verbosity
    _grain_level_being_tested: Option<GrainLevel>,
) -> CoreResult<()> {
    // Extract filename for logging (used in both contexts)
    let filename_cow = params
        .input_path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| params.input_path.to_string_lossy());

    // Log start with appropriate level based on context
    if is_grain_analysis_sample {
        // Less verbose for grain samples
        debug!("Starting grain sample FFmpeg encode for: {}", filename_cow);
    } else {
        // Standard verbose logging for main encode
        report_encode_start(&params.input_path, &params.output_path);
    }

    debug!("Encode parameters: {:?}", params);

    // Build other arguments using the helper function, passing flags down
    let ffmpeg_args = build_ffmpeg_args(params, None, disable_audio, is_grain_analysis_sample)?;

    // Create the command and set input/output/args
    // Use mutable command object and sequential calls
    let mut cmd = FfmpegCommand::new();

    // Add hardware acceleration options BEFORE the input
    let hw_accel_added = add_hardware_acceleration_to_command(
        &mut cmd,
        params.use_hw_decode,
        is_grain_analysis_sample,
    );

    // Only log hardware acceleration at debug level for detailed troubleshooting
    // Hardware acceleration status is already logged at the start of processing
    if hw_accel_added && log::log_enabled!(log::Level::Debug) {
        debug!("VideoToolbox hardware decoding enabled for this encode");
    }

    cmd.input(params.input_path.to_string_lossy());
    cmd.args(ffmpeg_args.iter().map(|s| s.as_str())); // Add the built arguments
    cmd.output(params.output_path.to_string_lossy());

    // Log the constructed command before spawning using Debug format
    let cmd_debug = format!("{:?}", cmd); // Log the final command state

    // Log command details at appropriate level based on context
    let log_level = if is_grain_analysis_sample {
        log::Level::Debug
    } else {
        log::Level::Info
    };
    let prefix = if is_grain_analysis_sample {
        "FFmpeg command (grain sample)"
    } else {
        "FFmpeg command details"
    };
    log!(log_level, "{}:\n  {}", prefix, cmd_debug);

    // --- Execution and Progress ---
    // Log start at appropriate level based on context
    let log_level = if is_grain_analysis_sample {
        log::Level::Debug
    } else {
        log::Level::Info
    };
    let message = if is_grain_analysis_sample {
        "Starting grain sample encode..."
    } else {
        "Starting encode process..."
    };
    log!(log_level, "{}", message);
    let start_time = Instant::now();

    // Use the injected spawner
    // Pass the owned cmd by value, matching the trait signature
    let mut child = spawner.spawn(cmd)?;

    // Initialize duration from params
    let duration_secs: Option<f64> = if params.duration > 0.0 {
        Some(params.duration)
    } else {
        None
    };
    if duration_secs.is_some() {
        // Use centralized reporting function for duration
        crate::progress_reporting::report_duration(params.duration, is_grain_analysis_sample);

        // Keep debug level log for grain samples
        if is_grain_analysis_sample {
            debug!(
                "Using provided duration for progress (grain sample): {}",
                format_duration_seconds(params.duration)
            );
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
                        report_encode_progress(
                            percent as f32,
                            current_secs,
                            duration_secs.unwrap_or(0.0),
                            progress.speed,
                            avg_encoding_fps as f32,
                            std::time::Duration::from_secs(eta_seconds as u64)
                        );
                    }

                    // Only log detailed progress at trace level to avoid redundancy
                    if log::log_enabled!(log::Level::Trace) {
                        trace!(
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
                    }

                    last_reported_percent = percent;
                }
            }
            FfmpegEvent::Error(err_str) => {
                let is_non_critical = is_non_critical_ffmpeg_error(&err_str);

                if is_non_critical {
                    // Log non-critical errors at debug level to reduce noise
                    debug!("ffmpeg non-critical message: {}", err_str);
                } else {
                    // Log critical errors at error level
                    error!("ffmpeg stderr error: {}", err_str);
                }

                // Always capture errors in the buffer for later processing
                // even if we don't log them at error level, so error handling still works.
                // This ensures errors are still properly propagated when needed.
                stderr_buffer.push_str(&format!("{}\n", err_str));
            }
            FfmpegEvent::Log(level, log_str) => {
                let rust_log_level = map_ffmpeg_log_level(&level);
                log!(target: "ffmpeg_log", rust_log_level, "{}", log_str);

                if log_str.starts_with("Svt[info]:") {
                    // Keep important configuration and summary SVT messages at info level
                    // but downgrade routine progress messages to debug level
                    let is_important_svt_message =
                        log_str.contains("SVT [config]") ||
                        log_str.contains("SVT [version]") ||
                        log_str.contains("SVT [build]") ||
                        log_str.contains("-------------------------------------------") ||
                        log_str.contains("Level of Parallelism") ||
                        log_str.contains("SUMMARY");

                    if is_important_svt_message {
                        // Use centralized reporting function for important encoder messages
                        crate::progress_reporting::report_encoder_message(&log_str, is_grain_analysis_sample);
                    }
                    // Always log at debug level regardless of importance for debugging
                    // Log routine SVT messages or all grain sample SVT messages at debug level
                    debug!("{}{}",
                        if is_grain_analysis_sample { "SVT Info (grain sample): " } else { "" },
                        log_str
                    );
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
    let filename_cow = params
        .input_path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| params.input_path.to_string_lossy());

    if status.success() {
        // Log success at appropriate level based on context
        let log_level = if is_grain_analysis_sample {
            log::Level::Debug
        } else {
            log::Level::Info
        };
        let prefix = if is_grain_analysis_sample {
            "Grain sample encode"
        } else {
            "Encode"
        };
        log!(
            log_level,
            "{} finished successfully for {}",
            prefix,
            filename_cow
        );
        Ok(())
    } else {
        let error_message = format!(
            "FFmpeg process exited with non-zero status ({:?}). Stderr output:\n{}",
            status.code(),
            stderr_buffer.trim()
        );

        // Log error with appropriate prefix based on context
        let prefix = if is_grain_analysis_sample {
            "Grain sample encode"
        } else {
            "FFmpeg encode"
        };
        error!("{} failed for {}: {}", prefix, filename_cow, error_message);

        // Use direct reporting for non-grain-analysis errors
        if !is_grain_analysis_sample {
            report_encode_error(&params.input_path, &error_message);
        }

        // Create a more specific error type based on stderr content
        if stderr_buffer.contains("No streams found") {
            // Handle "No streams found" as a specific error type
            // Note: We filter the logging of this error above, but still
            // propagate it properly here for correct error handling
            Err(CoreError::NoStreamsFound(filename_cow.to_string()))
        } else {
            Err(command_failed_error(
                "ffmpeg (sidecar)",
                status,
                error_message,
            ))
        }
    }
}

/// Formats a duration in seconds into a human-readable HH:MM:SS format.
///
/// This function converts a floating-point seconds value into a standardized
/// time format with hours, minutes, and seconds. It handles edge cases like
/// negative or non-finite values.
///
/// # Arguments
///
/// * `total_seconds` - The duration in seconds to format
///
/// # Returns
///
/// * A formatted time string in HH:MM:SS format
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

/// Maps hqdn3d denoising parameters to SVT-AV1 film grain synthesis values.
///
/// This function provides a mapping between FFmpeg's hqdn3d denoising filter parameters
/// and the corresponding film grain synthesis levels for SVT-AV1. It supports both
/// standard predefined parameter sets and interpolated/custom values.
///
/// The mapping uses a perceptually balanced approach that:
/// - Provides direct mappings for standard levels (VeryLight, Light, etc.)
/// - Uses a square-root scale for custom values to maintain perceptual linearity
/// - Optimizes for preserving natural-looking grain texture
///
/// # Arguments
///
/// * `hqdn3d_params` - The hqdn3d filter parameters as a string
///
/// # Returns
///
/// * The corresponding SVT-AV1 film grain synthesis value (0-16)
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

/// Extracts the luma spatial strength parameter from an hqdn3d filter string.
///
/// The luma spatial parameter is the first value in an hqdn3d filter string and
/// represents the most significant factor for determining denoising intensity.
///
/// # Arguments
///
/// * `params` - The complete hqdn3d filter string to parse
///
/// # Returns
///
/// * The extracted luma spatial strength as a float, or 0.0 if parsing fails
fn parse_hqdn3d_first_param(params: &str) -> f32 {
    if let Some(suffix) = params.strip_prefix("hqdn3d=") {
        if let Some(index) = suffix.find(':') {
            let first_param = &suffix[0..index];
            return first_param.parse::<f32>().unwrap_or(0.0);
        }
    }
    0.0 // Default fallback value if parsing fails or format is unexpected
}

/// Parses an FFmpeg time string in HH:MM:SS.ms format into seconds.
///
/// This function converts the standard FFmpeg time format into a floating-point
/// seconds value for easier calculation of progress and durations.
///
/// # Arguments
///
/// * `time_str` - The FFmpeg time string to parse (e.g., "01:30:45.500")
///
/// # Returns
///
/// * `Result<f64, &'static str>` - The parsed time in seconds or an error message
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
    let milliseconds: f64 = ms_str[..3]
        .parse()
        .map_err(|_| "Failed to parse milliseconds")?;

    Ok(hours * 3600.0 + minutes * 60.0 + seconds + milliseconds / 1000.0)
}

/// Filters FFmpeg error messages to identify non-critical warnings.
///
/// FFmpeg outputs many error-like messages that don't actually indicate problems
/// with the encoding process. This function identifies common non-critical messages
/// to allow for appropriate logging and error handling.
///
/// # Arguments
///
/// * `error_message` - The FFmpeg error message to evaluate
///
/// # Returns
///
/// * `true` if the message is non-critical, `false` if it's a genuine error
fn is_non_critical_ffmpeg_error(error_message: &str) -> bool {
    // Filter common non-critical FFmpeg error messages
    // These messages appear frequently during encoding but don't indicate
    // actual problems that would affect the output.
    let non_critical_patterns = [
        "No streams found",
        "Could not find codec parameters",
        "Application provided invalid, non monotonically increasing dts",
        "Invalid timestamp",
        "Metadata:.*not found",
    ];

    non_critical_patterns
        .iter()
        .any(|pattern| error_message.contains(pattern))
}

/// Converts FFmpeg log levels to standard Rust log levels.
///
/// This function maps the FFmpeg-specific log levels to the standard log levels
/// used by the Rust `log` facade, ensuring consistent logging behavior throughout
/// the application.
///
/// The mapping follows these rules:
/// - FFmpeg fatal/error → Rust error
/// - FFmpeg warning → Rust warn
/// - FFmpeg info → Rust info
/// - FFmpeg unknown → Rust debug (fallback)
///
/// # Arguments
///
/// * `level` - The FFmpeg log level to convert
///
/// # Returns
///
/// * The corresponding Rust log level
fn map_ffmpeg_log_level(level: &FfmpegLogLevel) -> log::Level {
    match level {
        FfmpegLogLevel::Fatal | FfmpegLogLevel::Error => log::Level::Error,
        FfmpegLogLevel::Warning => log::Level::Warn,
        FfmpegLogLevel::Info => log::Level::Info,
        FfmpegLogLevel::Unknown => log::Level::Debug,
    }
}

/// Calculates the estimated time remaining for an encoding operation.
///
/// This function computes the estimated time remaining based on the current progress,
/// total duration, and processing speed. It handles edge cases like near-zero speeds
/// and missing duration information.
///
/// # Arguments
///
/// * `duration_secs` - The total duration of the media in seconds (if known)
/// * `current_secs` - The current position in the media in seconds
/// * `speed` - The current encoding speed multiplier
///
/// # Returns
///
/// * The estimated time remaining in seconds
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

// TODO: Create mocking infrastructure for FFmpeg processes and add unit tests for this module.
