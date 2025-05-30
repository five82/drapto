// ============================================================================
// drapto-cli/src/commands/encode.rs
// ============================================================================
//
// ENCODE COMMAND: Implementation of the 'encode' Subcommand
//
// This file contains the implementation of the 'encode' subcommand, which is
// responsible for converting video files to AV1 format. It handles file discovery,
// configuration setup, logging, and delegation to the drapto-core library.
//
// KEY COMPONENTS:
// - discover_encode_files: Finds .mkv files to process
// - run_encode: Main function that handles the encoding process
//
// WORKFLOW:
// 1. Discover files to encode
// 2. Set up output directories and logging
// 3. Configure encoding parameters
// 4. Process videos using drapto-core
// 5. Report results and clean up
//
// AI-ASSISTANT-INFO: Encode command implementation, handles video conversion to AV1

// ---- Internal crate imports ----
use crate::cli::EncodeArgs;
use crate::config;
use crate::error::{CliResult, CliErrorContext};
use crate::terminal;

// ---- External crate imports ----
use drapto_core::notifications::NtfyNotificationSender;
// Progress reporting is now handled through standard log levels
use drapto_core::{CoreError, EncodeResult};

// ---- Standard library imports ----
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Instant;

// ---- Logging imports ----
use log::{debug, info, warn};

// Use the format_bytes function from drapto_core::utils
use drapto_core::format_bytes;

/// Discovers .mkv files to encode based on the provided arguments.
///
/// This function handles both file and directory inputs:
/// - If the input is a directory, it finds all .mkv files in that directory
/// - If the input is a file, it verifies that it's a .mkv file
///
/// # Arguments
/// * `args` - The encode command arguments containing the input path
///
/// # Returns
/// * `Ok((files, input_dir))` - A tuple containing:
///   - A vector of paths to the discovered .mkv files
///   - The effective input directory (parent directory if input is a file)
/// * `Err(...)` - An error if the input path is invalid or contains no valid files
///
/// # Errors
/// - If the input path doesn't exist or is inaccessible
/// - If the input is a file but not a .mkv file
/// - If the input is neither a file nor a directory
pub fn discover_encode_files(args: &EncodeArgs) -> CliResult<(Vec<PathBuf>, PathBuf)> {
    // Resolve the input path to its canonical form (absolute path with symlinks resolved)
    let input_path = args
        .input_path
        .canonicalize()
        .cli_with_context(|| format!("Invalid input path '{}'", args.input_path.display()))?;

    // Get metadata to determine if the input is a file or directory
    let metadata = fs::metadata(&input_path)
        .cli_with_context(|| format!("Failed to access input path '{}'", input_path.display()))?;

    if metadata.is_dir() {
        // Directory input: Find all .mkv files in the directory
        match drapto_core::find_processable_files(&input_path) {
            Ok(files) => Ok((files, input_path.clone())),
            Err(CoreError::NoFilesFound) => Ok((Vec::new(), input_path.clone())), // Empty vector if no files found
            Err(e) => Err(e), // Core error already has context
        }
    } else if metadata.is_file() {
        // File input: Verify it's a .mkv file
        if input_path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("mkv"))
        {
            // Get the parent directory to use as the effective input directory
            let parent_dir = input_path
                .parent()
                .ok_or_else(|| {
                    CoreError::OperationFailed(format!(
                        "Could not determine parent directory for file '{}'",
                        input_path.display()
                    ))
                })?
                .to_path_buf();
            Ok((vec![input_path.clone()], parent_dir))
        } else {
            Err(CoreError::OperationFailed(format!(
                "Input file '{}' is not a .mkv file",
                input_path.display()
            )))
        }
    } else {
        // Neither file nor directory
        Err(CoreError::OperationFailed(format!(
            "Input path '{}' is neither a file nor a directory",
            input_path.display()
        )))
    }
}

/// Main function that handles the encoding process.
///
/// This function:
/// 1. Sets up output directories and logging
/// 2. Configures encoding parameters
/// 3. Processes videos using drapto-core
/// 4. Reports results and cleans up
///
/// The function is generic over the types that implement the required traits:
/// - `S`: FfmpegSpawner - For spawning ffmpeg processes
/// - `P`: FfprobeExecutor - For executing ffprobe commands
/// - `N`: NotificationSender - For sending notifications
///
/// This design allows for dependency injection and easier testing.
///
/// # Arguments
/// * `spawner` - Implementation of FfmpegSpawner for executing ffmpeg
/// * `ffprobe_executor` - Implementation of FfprobeExecutor for executing ffprobe
/// * `notification_sender` - Implementation of NotificationSender for sending notifications
/// * `args` - The encode command arguments
/// * `interactive` - Whether the application is running in interactive mode
/// * `files_to_process` - Vector of paths to the files to encode
/// * `effective_input_dir` - The effective input directory
///
/// # Returns
/// * `Ok(())` - If the encoding process completes successfully
/// * `Err(...)` - If an error occurs during the encoding process
pub fn run_encode(
    notification_sender: Option<&NtfyNotificationSender>,
    args: EncodeArgs,
    interactive: bool,
    files_to_process: Vec<PathBuf>,
    effective_input_dir: PathBuf,
) -> CliResult<()> {
    let total_start_time = Instant::now();

    // Determine actual output directory and potential target filename
    let (actual_output_dir, target_filename_override_os) =
        if files_to_process.len() == 1 && args.output_dir.extension().is_some() {
            // Input is single file and output looks like a file path
            let target_file = args.output_dir.clone();
            let parent_dir = target_file
                .parent()
                .map(|p| p.to_path_buf())
                .filter(|p| !p.as_os_str().is_empty()) // Handle cases where parent might be empty (e.g., root)
                .unwrap_or_else(|| PathBuf::from(".")); // Default to current dir if no parent
            // Extract OsString filename, handle potential failure (though unlikely if extension exists)
            let filename_os = target_file.file_name().map(|name| name.to_os_string());
            (parent_dir, filename_os)
        } else {
            // Input is directory or output looks like a directory
            (args.output_dir.clone(), None)
        };

    // Convert Option<OsString> to Option<PathBuf> for the core function call
    let target_filename_override = target_filename_override_os.map(PathBuf::from);

    // Use the determined actual_output_dir for logs unless a specific log_dir is given
    let log_dir = args
        .log_dir
        .unwrap_or_else(|| actual_output_dir.join("logs"));

    // --- Create Output Dir ---
    // Note: Log dir is already created in main.rs before daemonization if in daemon mode
    // We still create it here for interactive mode or in case it was deleted
    fs::create_dir_all(&actual_output_dir).cli_with_context(|| {
        format!(
            "Failed to create output directory '{}'",
            actual_output_dir.display()
        )
    })?;
    fs::create_dir_all(&log_dir)
        .cli_with_context(|| format!("Failed to create log directory '{}'", log_dir.display()))?;

    // --- Logging Setup (Handled by env_logger via RUST_LOG) ---
    // We still need the log path for potential PID file and user info.
    let main_log_filename = format!("drapto_encode_run_{}.log", crate::logging::get_timestamp()); // Keep get_timestamp usage
    let main_log_path = log_dir.join(&main_log_filename); // Use reference

    // --- Log Initial Info using our new terminal module ---

    // Get file information from ffprobe for more useful user info
    let file_info = if !files_to_process.is_empty() {
        drapto_core::get_media_info(&files_to_process[0]).ok()
    } else {
        None
    };

    // Simplify the display path for better readability
    let input_path_display = args.input_path.display().to_string();
    let output_display = if let Some(fname) = &target_filename_override {
        fname.display().to_string()
    } else {
        actual_output_dir.display().to_string()
    };

    // Format duration if available
    let duration_display = if let Some(info) = &file_info {
        if let Some(duration_secs) = info.duration {
            let hours = (duration_secs / 3600.0) as u64;
            let minutes = ((duration_secs % 3600.0) / 60.0) as u64;
            let secs = (duration_secs % 60.0) as u64;
            let formatted = format!("{:02}:{:02}:{:02}", hours, minutes, secs);
            Some(formatted)
        } else {
            None
        }
    } else {
        None
    };

    // Format resolution if available
    let resolution_display = if let Some(info) = &file_info {
        if let (Some(width), Some(height)) = (info.width, info.height) {
            if width > 0 && height > 0 {
                let resolution_type = if width >= 3840 {
                    "(UHD)"
                } else if width >= 1280 {
                    // Combines both 1080p and 720p into HD
                    "(HD)"
                } else {
                    "(SD)"
                };

                Some(format!("{}x{} {}", width, height, resolution_type))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // Print initialization section
    terminal::print_section("INITIALIZATION");

    // Show only the most essential information
    terminal::print_status("Input file", &input_path_display, false);
    terminal::print_status("Output file", &output_display, false);

    if let Some(duration) = duration_display {
        terminal::print_status("Duration", &duration, false);
    }

    if let Some(resolution) = resolution_display {
        terminal::print_status("Resolution", &resolution, false);
    }

    // Show hardware acceleration info
    let hw_accel_info = drapto_core::hardware_accel::get_hardware_accel_info();
    let hw_display = match hw_accel_info {
        Some(info) => format!("{} (decode only)", info),
        None => "None available".to_string(),
    };
    terminal::print_status("Hardware", &hw_display, false);

    // Debug information
    debug!("Log file: {}", main_log_path.display());
    debug!("Interactive: {}", interactive);
    debug!("Run started: {}", chrono::Local::now());

    // No divider here - we'll rely on section spacing
    // Note: Terminal module already sets verbosity in progress_reporting when initialized

    // --- PID File Handling (Daemon Mode Only) ---
    if !interactive {
        let pid_path = log_dir.join("drapto.pid");
        // Create PID file with current process ID after daemonization
        if let Err(e) = std::fs::write(&pid_path, std::process::id().to_string()) {
            warn!(
                "Warning: Failed to create PID file at {}: {}",
                pid_path.display(),
                e
            );
        } else {
            info!("PID file created at: {}", pid_path.display());
        }
    }

    // --- Prepare Core Configuration ---

    // Parse grain level strings to GrainLevel enum values using FromStr implementation
    let grain_max_level = args.grain_max_level.as_deref().and_then(|level_str| {
        match drapto_core::processing::detection::grain_analysis::GrainLevel::from_str(level_str) {
            Ok(level) => Some(level),
            Err(_) => {
                // Log warning for invalid grain level
                debug!("Warning: Invalid grain_max_level '{}'. Using default.", level_str);
                None
            }
        }
    });

    // grain_fallback_level is deprecated and no longer used

    // Validate knee threshold is within valid range (0.1 to 1.0)
    let grain_knee_threshold = args.grain_knee_threshold.and_then(|threshold| {
        if !(0.1..=1.0).contains(&threshold) {
            // Log warning for invalid knee threshold
            debug!("Warning: Knee threshold {} is outside valid range (0.1-1.0). Using default.", threshold);
            None
        } else {
            Some(threshold)
        }
    });

    // Use the builder pattern to create the CoreConfig
    let mut builder = drapto_core::config::CoreConfigBuilder::new()
        .input_dir(effective_input_dir)
        .output_dir(actual_output_dir.clone())
        .log_dir(log_dir.clone())
        .enable_denoise(!args.no_denoise); // Invert the flag: no_denoise=true means enable_denoise=false

    // Add optional parameters if they are provided
    if let Some(quality) = args.quality_sd {
        builder = builder.quality_sd(quality);
    }

    if let Some(quality) = args.quality_hd {
        builder = builder.quality_hd(quality);
    }

    if let Some(quality) = args.quality_uhd {
        builder = builder.quality_uhd(quality);
    }

    // Set crop mode based on disable_autocrop flag
    let crop_mode = if args.disable_autocrop {
        "none" // Use "none" for main encode if flag is set
    } else {
        config::DEFAULT_CROP_MODE // Use default otherwise
    };
    builder = builder.crop_mode(crop_mode);

    // Add ntfy topic if provided
    if let Some(topic) = args.ntfy {
        builder = builder.ntfy_topic(&topic);
    }

    // Add preset if provided
    if let Some(preset) = args.preset {
        builder = builder.encoder_preset(preset);
    }

    // Add grain analysis configuration if provided
    if let Some(duration) = args.grain_sample_duration {
        builder = builder.film_grain_sample_duration(duration);
    }

    if let Some(threshold) = grain_knee_threshold {
        builder = builder.film_grain_knee_threshold(threshold);
    }

    if let Some(level) = grain_max_level {
        builder = builder.film_grain_max_level(level);
    }

    // grain_fallback_level is deprecated and no longer used

    // Build the final config
    let config = builder.build();

    // --- Progress Reporting ---
    // The CLI progress reporter is already registered in main.rs via terminal::register_cli_reporter()
    // Progress will be displayed automatically through the ProgressReporter trait implementation

    // NOTE: We don't need to log hardware acceleration here
    // Hardware acceleration status is logged by the core library in process_videos

    // --- Execute Core Logic ---
    // Only print section if we have files to process
    if !files_to_process.is_empty() {
        terminal::print_section("VIDEO ANALYSIS");
        terminal::print_processing_no_spacing(&format!(
            "Analyzing {} file(s)",
            files_to_process.len()
        ));
    }

    let processing_result = if files_to_process.is_empty() {
        warn!("Warning: No processable .mkv files found in the specified input path."); // Use warn level
        Ok(Vec::new())
    } else {
        // Call drapto_core::process_videos
        drapto_core::process_videos(
            notification_sender,
            &config,
            &files_to_process,
            target_filename_override,
        )
        .cli_context("Video processing failed")
    };

    // --- Handle Core Results ---
    let successfully_encoded: Vec<EncodeResult>;
    match processing_result {
        Ok(ref results) => {
            successfully_encoded = results.clone();
            // Use warn level if no files encoded
            if successfully_encoded.is_empty() {
                terminal::print_error(
                    "No files encoded",
                    "No files were successfully encoded",
                    Some("Check that your input files are valid .mkv files"),
                );
            } else {
                // Print complete section
                terminal::print_section("ENCODING COMPLETE");
                terminal::print_success(&format!(
                    "Successfully encoded {} file(s)",
                    successfully_encoded.len()
                ));

                // Show summary for each file
                for result in &successfully_encoded {
                    let reduction = if result.input_size > 0 {
                        100.0 - ((result.output_size as f64 / result.input_size as f64) * 100.0)
                    } else {
                        0.0
                    };

                    terminal::print_subsection(&result.filename);
                    terminal::print_status(
                        "Encode time",
                        &drapto_core::utils::format_duration(result.duration),
                        false,
                    );
                    terminal::print_status("Input size", &format_bytes(result.input_size), false);
                    terminal::print_status("Output size", &format_bytes(result.output_size), false);
                    terminal::print_status("Reduced by", &format!("{:.1}%", reduction), true);
                }
            }
        }
        Err(e) => {
            // Use error level for fatal errors
            terminal::print_error("Fatal error during processing", &e.to_string(), None);
            return Err(e);
        }
    }

    // --- Clean up temporary directories ---
    debug!("Cleaning up temporary directories");

    if let Err(e) = drapto_core::temp_files::cleanup_base_dirs(&config)
        .cli_context("Failed to clean up temporary directories")
    {
        // Always show errors regardless of verbosity
        terminal::print_error("Cleanup warning", &e.to_string(), None);
    }

    // --- Print Summary ---
    if !successfully_encoded.is_empty() {
        terminal::print_section("Summary");

        for result in &successfully_encoded {
            let reduction = if result.input_size > 0 {
                100u64.saturating_sub(result.output_size.saturating_mul(100) / result.input_size)
            } else {
                0
            };

            // Only print filename if multiple files
            if successfully_encoded.len() > 1 {
                terminal::print_subsection(&result.filename);
            }

            // Print file statistics with shorter labels
            terminal::print_status(
                "Time",
                &drapto_core::format_duration(result.duration),
                false,
            );
            terminal::print_status(
                "Input",
                &drapto_core::format_bytes(result.input_size),
                false,
            );
            terminal::print_status(
                "Output",
                &drapto_core::format_bytes(result.output_size),
                true,
            );
            terminal::print_status("Reduction", &format!("{}%", reduction), true);
        }
    }

    // --- Final Timing ---
    let total_elapsed_time = total_start_time.elapsed();

    // Show completion information
    debug!("Finished at: {}", chrono::Local::now());
    
    // Always show total time regardless of verbosity
    terminal::print_status(
        "Total time",
        &drapto_core::format_duration(total_elapsed_time),
        true,
    );

    // env_logger handles flushing automatically.

    Ok(())
}
