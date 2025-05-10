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
use crate::platform::HardwareAcceleration;
use crate::progress::CliProgressCallback;

// ---- External crate imports ----
use drapto_core::{CoreError, EncodeResult};
use drapto_core::external::{FfmpegSpawner, FfprobeExecutor};
use drapto_core::external::StdFsMetadataProvider;
use drapto_core::notifications::NtfyNotificationSender;
use colored::*;

// ---- Standard library imports ----
use std::fs;
use std::time::Instant;
use std::path::PathBuf;
use std::str::FromStr;

// ---- Logging imports ----
use log::{info, warn, error};

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
pub fn discover_encode_files(args: &EncodeArgs) -> Result<(Vec<PathBuf>, PathBuf), Box<dyn std::error::Error>> {
    // Resolve the input path to its canonical form (absolute path with symlinks resolved)
    let input_path = args.input_path.canonicalize()
        .map_err(|e| format!("Invalid input path '{}': {}", args.input_path.display(), e))?;

    // Get metadata to determine if the input is a file or directory
    let metadata = fs::metadata(&input_path)
        .map_err(|e| format!("Failed to access input path '{}': {}", input_path.display(), e))?;

    if metadata.is_dir() {
        // Directory input: Find all .mkv files in the directory
        match drapto_core::find_processable_files(&input_path) {
             Ok(files) => Ok((files, input_path.clone())),
             Err(CoreError::NoFilesFound) => Ok((Vec::new(), input_path.clone())), // Empty vector if no files found
             Err(e) => Err(e.into()), // Propagate other core errors
        }
    } else if metadata.is_file() {
        // File input: Verify it's a .mkv file
        if input_path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("mkv")) {
            // Get the parent directory to use as the effective input directory
            let parent_dir = input_path.parent().ok_or_else(|| {
                CoreError::PathError(format!("Could not determine parent directory for file '{}'", input_path.display()))
            })?.to_path_buf();
            Ok((vec![input_path.clone()], parent_dir))
        } else {
            Err(format!("Input file '{}' is not a .mkv file.", input_path.display()).into())
        }
    } else {
        // Neither file nor directory
        Err(format!("Input path '{}' is neither a file nor a directory.", input_path.display()).into())
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
pub fn run_encode<S: FfmpegSpawner, P: FfprobeExecutor>(
    spawner: &S,
    ffprobe_executor: &P,
    notification_sender: Option<&NtfyNotificationSender>,
    args: EncodeArgs,
    interactive: bool,
    files_to_process: Vec<PathBuf>,
    effective_input_dir: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let total_start_time = Instant::now();

    // Determine actual output directory and potential target filename
    let (actual_output_dir, target_filename_override_os) =
        if files_to_process.len() == 1 && args.output_dir.extension().is_some() {
            // Input is single file and output looks like a file path
            let target_file = args.output_dir.clone();
            let parent_dir = target_file.parent()
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
    let log_dir = args.log_dir.unwrap_or_else(|| actual_output_dir.join("logs"));


    // --- Create Output Dir ---
    // Note: Log dir is already created in main.rs before daemonization if in daemon mode
    // We still create it here for interactive mode or in case it was deleted
    fs::create_dir_all(&actual_output_dir)?;
    fs::create_dir_all(&log_dir)?;

    // --- Logging Setup (Handled by env_logger via RUST_LOG) ---
    // We still need the log path for potential PID file and user info.
    let main_log_filename = format!("drapto_encode_run_{}.log", crate::logging::get_timestamp()); // Keep get_timestamp usage
    let main_log_path = log_dir.join(&main_log_filename); // Use reference

    // --- Log Initial Info using standard log macros with color ---
    info!("{}", "========================================".cyan().bold());
    info!("{} {}", "Drapto Encode Run Started:".green().bold(), chrono::Local::now().to_string().green());
    info!("  {:<25} {}", "Original Input arg:".cyan(), args.input_path.display().to_string().yellow());
    info!("  {:<25} {}", "Original Output arg:".cyan(), args.output_dir.display().to_string().yellow());
    info!("  {:<25} {}", "Effective Output directory:".cyan(), actual_output_dir.display().to_string().green());
    if let Some(fname) = &target_filename_override {
        info!("  {:<25} {}", "Effective Output filename:".cyan(), fname.display().to_string().green());
    }
    info!("  {:<25} {}", "Log directory:".cyan(), log_dir.display().to_string().green());
    info!("  {:<25} {}", "Main log file (info):".cyan(), main_log_path.display().to_string().green());
    info!("  {:<25} {}", "Interactive mode:".cyan(), interactive.to_string().green());
    // Hardware acceleration has been removed. Software decoding is always used.
    info!("{}", "========================================".cyan().bold());

    // --- PID File Handling (Daemon Mode Only) ---
    if !interactive {
        let pid_path = log_dir.join("drapto.pid");
        // Create PID file with current process ID after daemonization
        if let Err(e) = std::fs::write(&pid_path, std::process::id().to_string()) {
            warn!("{} Failed to create PID file at {}: {}", "Warning:".yellow().bold(), pid_path.display(), e);
        } else {
            info!("{} {}", "PID file created at:".green(), pid_path.display().to_string().yellow());
        }
    }

    // --- Prepare Core Configuration ---

    // Parse grain level strings to GrainLevel enum values using FromStr implementation
    let grain_max_level = args.grain_max_level.as_deref().and_then(|level_str| {
        match drapto_core::processing::detection::grain_analysis::GrainLevel::from_str(level_str) {
            Ok(level) => Some(level),
            Err(_) => {
                warn!("{} Invalid grain_max_level '{}'. Using default.", "Warning:".yellow().bold(), level_str);
                None
            }
        }
    });

    let grain_fallback_level = args.grain_fallback_level.as_deref().and_then(|level_str| {
        match drapto_core::processing::detection::grain_analysis::GrainLevel::from_str(level_str) {
            Ok(level) => Some(level),
            Err(_) => {
                warn!("{} Invalid grain_fallback_level '{}'. Using default.", "Warning:".yellow().bold(), level_str);
                None
            }
        }
    });

    // Validate knee threshold is within valid range (0.1 to 1.0)
    let grain_knee_threshold = args.grain_knee_threshold.and_then(|threshold| {
        if !(0.1..=1.0).contains(&threshold) {
            warn!("{} Knee threshold {} is outside valid range (0.1-1.0). Using default.",
                "Warning:".yellow().bold(), threshold);
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
        .default_encoder_preset(config::DEFAULT_ENCODER_PRESET as u8)
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
    builder = builder.default_crop_mode(crop_mode);

    // Add ntfy topic if provided
    if let Some(topic) = args.ntfy {
        builder = builder.ntfy_topic(&topic);
    }

    // Add preset if provided
    if let Some(preset) = args.preset {
        builder = builder.preset(preset);
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

    if let Some(level) = grain_fallback_level {
        builder = builder.film_grain_fallback_level(level);
    }

    // Build the final config
    let config = builder.build();

    // --- Create Progress Callback ---
    let progress_callback = CliProgressCallback::new(interactive);

    // --- Detect Hardware Acceleration ---
    let hw_accel = HardwareAcceleration::detect();
    hw_accel.log_capabilities();

    // --- Execute Core Logic ---
    info!("{} {} file(s).", "Processing".green(), files_to_process.len().to_string().green().bold());
    let processing_result = if files_to_process.is_empty() {
         warn!("{} No processable .mkv files found in the specified input path.", "Warning:".yellow().bold()); // Use warn level
         Ok(Vec::new())
    } else {
         // Pass the Option<PathBuf> target_filename_override
         // Spawner is now passed in as an argument
         // Notifier is now passed in as an argument

         // Call drapto_core::process_videos with our progress callback
         drapto_core::process_videos(
             spawner, // Pass the injected spawner instance (S)
             ffprobe_executor, // Pass the injected ffprobe executor instance (P)
             notification_sender, // Pass the injected notification sender instance (N)
             &StdFsMetadataProvider, // Pass the standard metadata provider
             &config, // Pass config (&CoreConfig)
             &files_to_process,
             target_filename_override,
             &progress_callback // Pass our progress callback
         )
    };

    // --- Handle Core Results ---
    let successfully_encoded: Vec<EncodeResult>;
    match processing_result {
        Ok(ref results) => {
            successfully_encoded = results.to_vec();
            // Use warn level if no files encoded (unless it was expected due to no files found)
            if successfully_encoded.is_empty() && !matches!(processing_result, Err(CoreError::NoFilesFound)) {
                 warn!("{} No files were successfully encoded.", "Warning:".yellow().bold());
            } else if !successfully_encoded.is_empty() {
                 info!("{} {} file(s).", "Successfully encoded".green(), successfully_encoded.len().to_string().green().bold());
            }
        }
        Err(e) => {
            // Use error level for fatal errors
            error!("{} {}", "FATAL CORE ERROR during processing:".red().bold(), e);
            return Err(e.into());
        }
    }

    // --- Clean up temporary directories ---
    info!("{}", "Cleaning up temporary directories...".cyan());
    if let Err(e) = drapto_core::temp_files::cleanup_base_dirs(&config) {
        warn!("{} Failed to clean up temporary directories: {}", "Warning:".yellow().bold(), e);
    }


    // --- Print Summary ---
    if !successfully_encoded.is_empty() {
        info!("{}", "========================================".cyan().bold());
        info!("{}", "Encoding Summary:".green().bold());
        info!("{}", "========================================".cyan().bold());
        for result in &successfully_encoded {
            let reduction = if result.input_size > 0 {
                100u64.saturating_sub(result.output_size.saturating_mul(100) / result.input_size)
            } else {
                0
            };
            info!("{}", result.filename.to_string().yellow().bold()); // Log filename directly, bold yellow
            info!("  {:<13} {}", "Encode time:".cyan(), drapto_core::format_duration(result.duration).green());
            info!("  {:<13} {}", "Input size:".cyan(), drapto_core::format_bytes(result.input_size).green());
            info!("  {:<13} {}", "Output size:".cyan(), drapto_core::format_bytes(result.output_size).green());
            info!("  {:<13} {}", "Reduced by:".cyan(), format!("{}%", reduction).green());
            info!("{}", "----------------------------------------".cyan());
        }
    }

    // --- Final Timing ---
    let total_elapsed_time = total_start_time.elapsed();
    info!("{}", "========================================".cyan().bold());
    info!("{} {}", "Total encode execution time:".green().bold(), drapto_core::format_duration(total_elapsed_time).green());
    info!("{} {}", "Drapto Encode Run Finished:".green().bold(), chrono::Local::now().to_string().green());
    info!("{}", "========================================".cyan().bold());

    // env_logger handles flushing automatically.

    Ok(())
}