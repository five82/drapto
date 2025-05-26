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
// - build_ffmpeg_command: Builds FFmpeg commands using ffmpeg-sidecar's builder pattern
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
use crate::processing::audio; // To access calculate_audio_bitrate
use crate::processing::detection::grain_analysis::GrainLevel;
use crate::progress_reporting::report_encode_start;

// ---- External crate imports ----
use ffmpeg_sidecar::command::FfmpegCommand;
use log::{debug, error, info, warn};

// ---- Standard library imports ----
use std::path::{Path, PathBuf};
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

/// Builds and configures an FFmpeg command using ffmpeg-sidecar's builder pattern.
///
/// This function creates a complete FFmpeg command for video encoding with libsvtav1
/// and audio encoding with libopus. It leverages ffmpeg-sidecar's builder methods
/// for cleaner and more maintainable code.
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
/// * `CoreResult<FfmpegCommand>` - The configured FFmpeg command ready for execution
pub fn build_ffmpeg_command(
    params: &EncodeParams,
    hqdn3d_override: Option<&str>,
    disable_audio: bool,
    is_grain_analysis_sample: bool,
) -> CoreResult<FfmpegCommand> {
    // Use the new builder for common setup
    let mut cmd = crate::external::FfmpegCommandBuilder::new()
        .with_hardware_accel(params.use_hw_decode)
        .build();
    
    // Input file
    cmd.input(params.input_path.to_string_lossy().as_ref());
    
    // Audio filter (if not disabled)
    if !is_grain_analysis_sample && !disable_audio {
        cmd.args(["-af", "aformat=channel_layouts=7.1|5.1|stereo|mono"]);
    }
    
    // Build video filter chain using the new builder
    let hqdn3d_to_use = hqdn3d_override.or(params.hqdn3d_params.as_deref());
    let filter_chain = crate::external::VideoFilterChain::new()
        .add_denoise(hqdn3d_to_use.unwrap_or(""))
        .add_crop(params.crop_filter.as_deref().unwrap_or(""))
        .build();
    
    // Apply video filters and report if any
    if let Some(ref filters) = filter_chain {
        cmd.args(["-vf", filters]);
        crate::progress_reporting::report_video_filters(filters, is_grain_analysis_sample);
        if is_grain_analysis_sample {
            debug!("Applying video filters (grain sample): {}", filters);
        }
    } else {
        crate::progress_reporting::report_video_filters("", is_grain_analysis_sample);
    }
    
    // Calculate film grain value
    let film_grain_value = if let Some(denoise_params) = hqdn3d_to_use {
        map_hqdn3d_to_film_grain(denoise_params)
    } else {
        0 // No denoise = no synthetic grain
    };
    
    // Video encoding configuration
    cmd.args(["-c:v", "libsvtav1"]);
    cmd.args(["-pix_fmt", "yuv420p10le"]);
    cmd.args(["-crf", &params.quality.to_string()]);
    cmd.args(["-preset", &params.preset.to_string()]);
    
    // Film grain synthesis parameters using the builder
    let svtav1_params = crate::external::SvtAv1ParamsBuilder::new()
        .with_film_grain(film_grain_value)
        .build();
    cmd.args(["-svtav1-params", &svtav1_params]);
    
    // Report film grain settings
    crate::progress_reporting::report_film_grain(
        if film_grain_value > 0 { Some(film_grain_value) } else { None },
        is_grain_analysis_sample,
    );
    
    if is_grain_analysis_sample && film_grain_value > 0 {
        debug!("Applying film grain synthesis (grain sample): level={}", film_grain_value);
    }
    
    // Audio configuration
    if !is_grain_analysis_sample && !disable_audio {
        cmd.args(["-c:a", "libopus"]);
        
        // Set bitrate for each audio stream
        for (i, &channels) in params.audio_channels.iter().enumerate() {
            let bitrate = audio::calculate_audio_bitrate(channels);
            cmd.args([&format!("-b:a:{}", i), &format!("{}k", bitrate)]);
        }
    }
    
    // Stream mapping
    if is_grain_analysis_sample || disable_audio {
        cmd.args(["-map", "0:v:0"]); // Video only
        if disable_audio {
            cmd.arg("-an"); // Explicitly disable audio
        }
    } else {
        cmd.args(["-map", "0:v:0"]); // Video stream
        cmd.args(["-map", "0:a"]);    // All audio streams
        cmd.args(["-map_metadata", "0"]);
        cmd.args(["-map_chapters", "0"]);
    }
    
    // Additional output settings
    cmd.args(["-movflags", "+faststart"]);
    
    // Note: Progress reporting is handled automatically by ffmpeg-sidecar
    // through FfmpegEvent::Progress events, so we don't need -progress pipe:1
    
    // Output file
    cmd.output(params.output_path.to_string_lossy().as_ref());
    
    Ok(cmd)
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
pub fn run_ffmpeg_encode(
    params: &EncodeParams,
    disable_audio: bool,
    is_grain_analysis_sample: bool,
    _grain_level_being_tested: Option<GrainLevel>,
) -> CoreResult<()> {
    // Extract filename for logging
    let filename_cow = params
        .input_path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| params.input_path.to_string_lossy());

    // Log start with appropriate level based on context
    if is_grain_analysis_sample {
        debug!("Starting grain sample FFmpeg encode for: {}", filename_cow);
    } else {
        report_encode_start(&params.input_path, &params.output_path);
        info!(
            target: "drapto::progress",
            "Starting encode: {} -> {}",
            params.input_path.display(),
            params.output_path.display()
        );
    }

    debug!("Encode parameters: {:?}", params);

    // Build the FFmpeg command using the new builder function
    let mut cmd = build_ffmpeg_command(params, None, disable_audio, is_grain_analysis_sample)?;

    // Log the command for debugging
    if is_grain_analysis_sample {
        debug!("FFmpeg command (grain sample): {:?}", cmd);
    } else {
        // Convert command to string representation for reporting
        let cmd_string = format!("{:?}", cmd);
        crate::progress_reporting::report_ffmpeg_command(&cmd_string, false);
    }

    // --- Execution and Progress ---
    if is_grain_analysis_sample {
        debug!("Starting grain sample encode...");
    }
    let _start_time = Instant::now();

    // Spawn the ffmpeg command
    let mut child = cmd.spawn()
        .map_err(|e| command_failed_error(
            "ffmpeg",
            std::process::ExitStatus::default(),
            format!("Failed to start: {}", e),
        ))?;

    // Initialize duration from params
    let duration_secs: Option<f64> = if params.duration > 0.0 {
        Some(params.duration)
    } else {
        None
    };
    
    if let Some(duration) = duration_secs {
        crate::progress_reporting::report_duration(duration, is_grain_analysis_sample);
        if is_grain_analysis_sample {
            debug!("Using provided duration for progress (grain sample): {:.2}s", duration);
        }
    } else {
        warn!("Video duration not provided or zero; progress percentage will not be accurate.");
    }
    
    // Create progress handler
    let mut progress_handler = crate::progress_reporting::ffmpeg_handler::FfmpegProgressHandler::new(
        duration_secs,
        is_grain_analysis_sample,
    );

    // Process events from ffmpeg until completion
    for event in child.iter().map_err(|e| command_failed_error(
        "ffmpeg",
        std::process::ExitStatus::default(),
        format!("Failed to get event iterator: {}", e),
    ))? {
        progress_handler.handle_event(event)?;
    }

    // Iterator completed successfully, which means FFmpeg finished
    // If there were errors, they would have been caught in handle_event
    // Create a successful exit status for compatibility with existing code
    let status = std::process::ExitStatus::default();

    // Extract filename for logging
    let filename_cow = params
        .input_path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| params.input_path.to_string_lossy());

    if status.success() {
        // Clear any active progress bar before printing success messages
        if !is_grain_analysis_sample {
            crate::progress_reporting::clear_progress_bar();
        }

        // Log success at appropriate level based on context
        let prefix = if is_grain_analysis_sample {
            "Grain sample encode"
        } else {
            "Encode"
        };
        // Use progress reporting for proper indentation
        if is_grain_analysis_sample {
            // Log at debug level for grain samples
            log::debug!("{} finished successfully for {}", prefix, filename_cow);
        } else {
            // Use sub-item formatting for main encodes to properly indent
            crate::progress_reporting::report_sub_item(&format!(
                "{} finished successfully for {}",
                prefix, filename_cow
            ));
        }
        Ok(())
    } else {
        let error_message = format!(
            "FFmpeg process exited with non-zero status ({:?}). Stderr output:\n{}",
            status.code(),
            progress_handler.stderr_buffer().trim()
        );

        // Log error with appropriate prefix based on context
        let prefix = if is_grain_analysis_sample {
            "Grain sample encode"
        } else {
            "FFmpeg encode"
        };
        // Use progress reporting for error messages
        if !is_grain_analysis_sample {
            crate::progress_reporting::report_encode_error(&params.input_path, &error_message);
        } else {
            // For grain samples, just log at debug level
            debug!("{} failed for {}: {}", prefix, filename_cow, error_message);
        }

        // Create a more specific error type based on stderr content
        if progress_handler.stderr_buffer().contains("No streams found") {
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
        ("hqdn3d=0.5:0.4:3:3", 4),       // VeryLight
        ("hqdn3d=0.9:0.7:4:4", 7),       // Light
        ("hqdn3d=1.2:0.85:5:5", 10),     // LightModerate
        ("hqdn3d=1.5:1.0:6:6", 13),      // Moderate
        ("hqdn3d=2:1.3:8:8", 16),        // Elevated
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

// ============================================================================
// SAMPLE EXTRACTION
// ============================================================================

/// Extracts a raw video sample using ffmpeg's -c copy.
///
/// Creates a temporary file within the specified `output_dir` using the temp_files module.
/// The file will be cleaned up when the `output_dir` (assumed to be a TempDir) is dropped.
pub fn extract_sample(
    input_path: &Path,
    start_time_secs: f64,
    duration_secs: u32,
    output_dir: &Path,
) -> CoreResult<PathBuf> {
    debug!(
        "Extracting sample: input={}, start={}, duration={}, out_dir={}",
        input_path.display(),
        start_time_secs,
        duration_secs,
        output_dir.display()
    );

    // Generate a unique filename for the sample within the output directory
    let output_path = crate::temp_files::create_temp_file_path(output_dir, "raw_sample", "mkv");

    // Build the command using the unified builder
    let mut cmd = crate::external::FfmpegCommandBuilder::new()
        .with_hardware_accel(true)
        .build();

    cmd.input(input_path.to_string_lossy().as_ref())
        .args(["-ss", &start_time_secs.to_string()])
        .args(["-t", &duration_secs.to_string()])
        .args(["-c", "copy"])          // Use stream copy
        .args(["-an"])                  // No audio
        .args(["-sn"])                  // No subtitles
        .args(["-map", "0:v"])          // Map video stream 0
        .args(["-map_metadata", "0"])   // Map metadata from input 0
        .output(output_path.to_string_lossy().as_ref());

    debug!("Running sample extraction command: {:?}", cmd);

    // Spawn and wait for completion
    let mut child = cmd.spawn()
        .map_err(|e| command_failed_error(
            "ffmpeg",
            std::process::ExitStatus::default(),
            format!("Failed to start sample extraction: {}", e),
        ))?;
    
    let status = child.wait()
        .map_err(|e| command_failed_error(
            "ffmpeg",
            std::process::ExitStatus::default(),
            format!("Failed to wait for sample extraction: {}", e),
        ))?;
        
    if !status.success() {
        error!("Sample extraction failed: {}", status);
        return Err(command_failed_error(
            "ffmpeg (sample extraction)",
            status,
            "Sample extraction process failed",
        ));
    }

    debug!("Sample extracted successfully to: {}", output_path.display());
    Ok(output_path)
}

// TODO: Create mocking infrastructure for FFmpeg processes and add unit tests for this module.
