// drapto-cli/src/commands/encode.rs
//
// Contains the logic for the 'encode' subcommand.

use crate::cli::EncodeArgs; // Use the definition from cli.rs
use crate::config; // Access defaults from config.rs
use crate::logging::{create_log_callback, get_timestamp}; // Use logging helpers
use drapto_core::{CoreConfig, CoreError, EncodeResult};
use std::fs::{self, File};
// use std::path::PathBuf; // Removed unused import
use std::time::Instant;
use std::path::PathBuf; // Ensure PathBuf is imported, remove unused Path

// --- New function to discover files ---
pub fn discover_encode_files(args: &EncodeArgs) -> Result<(Vec<PathBuf>, PathBuf), Box<dyn std::error::Error>> {
    let input_path = args.input_path.canonicalize()
        .map_err(|e| format!("Invalid input path '{}': {}", args.input_path.display(), e))?;

    let metadata = fs::metadata(&input_path)
        .map_err(|e| format!("Failed to access input path '{}': {}", input_path.display(), e))?;

    if metadata.is_dir() {
        // Input is a directory
        match drapto_core::find_processable_files(&input_path) {
             Ok(files) => Ok((files, input_path.clone())),
             Err(CoreError::NoFilesFound) => Ok((Vec::new(), input_path.clone())), // Return empty vec if no files found
             Err(e) => Err(e.into()), // Propagate other core errors
        }
    } else if metadata.is_file() {
        // Input is a file
        if input_path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("mkv")) {
            let parent_dir = input_path.parent().ok_or_else(|| {
                CoreError::PathError(format!("Could not determine parent directory for file '{}'", input_path.display()))
            })?.to_path_buf();
            Ok((vec![input_path.clone()], parent_dir))
        } else {
            Err(format!("Input file '{}' is not a .mkv file.", input_path.display()).into())
        }
    } else {
        Err(format!("Input path '{}' is neither a file nor a directory.", input_path.display()).into())
    }
}



// Renamed the main logic function to reflect the 'encode' action
// --- Modified run_encode function ---
pub fn run_encode(
    args: EncodeArgs,
    interactive: bool,
    files_to_process: Vec<PathBuf>, // Accept discovered files
    effective_input_dir: PathBuf,   // Accept effective input dir
) -> Result<(), Box<dyn std::error::Error>> {
    let total_start_time = Instant::now();

    // Determine actual output directory and potential target filename
    let (actual_output_dir, target_filename_override_os) = // Renamed variable for clarity
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

    // File discovery logic moved to discover_encode_files

    // --- Create Output/Log Dirs ---
    fs::create_dir_all(&actual_output_dir)?; // Create the actual output directory
    fs::create_dir_all(&log_dir)?;

    // --- Setup Logging ---
    let main_log_filename = format!("drapto_encode_run_{}.log", get_timestamp());
    let main_log_path = log_dir.join(main_log_filename);
    let log_file = File::create(&main_log_path)?;
    // create_log_callback returns Box<dyn FnMut...>
    let mut log_callback = create_log_callback(log_file, interactive)?; // Pass interactive flag

    // --- Log Initial Info ---
    log_callback("========================================");
    log_callback(&format!("Drapto Encode Run Started: {}", chrono::Local::now()));
    log_callback(&format!("Original Input arg: {}", args.input_path.display())); // Log original arg
    log_callback(&format!("Original Output arg: {}", args.output_dir.display())); // Log original arg
    log_callback(&format!("Effective Output directory: {}", actual_output_dir.display()));
    if let Some(fname) = &target_filename_override {
        log_callback(&format!("Effective Output filename: {}", fname.display()));
    }
    log_callback(&format!("Log directory: {}", log_dir.display()));
    log_callback(&format!("Main log file: {}", main_log_path.display()));
    log_callback(&format!("Interactive mode: {}", interactive)); // Log mode
    log_callback("========================================");

    // --- PID File Handling (Daemon Mode Only) ---
    if !interactive {
        let pid_path = log_dir.join("drapto.pid");
        // Use std::fs::write to create/overwrite the PID file with the current process ID.
        // Note: This happens *after* daemonization and log setup.
        match std::fs::write(&pid_path, std::process::id().to_string()) {
            Ok(_) => log_callback(&format!("[INFO] PID file created at: {}", pid_path.display())),
            Err(e) => log_callback(&format!("[WARN] Failed to create PID file at {}: {}", pid_path.display(), e)),
            // Consider adding cleanup for the PID file on exit (e.g., using signal handling or atexit crate),
            // but that adds complexity. For now, manual cleanup is assumed.
        }
    }

    // --- Prepare Core Configuration ---
    let config = CoreConfig {
        input_dir: effective_input_dir, // Use passed effective_input_dir
        output_dir: actual_output_dir.clone(), // Use the determined directory
        log_dir: log_dir.clone(),
        default_encoder_preset: Some(config::DEFAULT_ENCODER_PRESET as u8),
        quality_sd: args.quality_sd,
        quality_hd: args.quality_hd,
        quality_uhd: args.quality_uhd,
        default_crop_mode: Some(if args.disable_autocrop {
            "none".to_string() // Use "none" for main encode if flag is set
        } else {
            config::DEFAULT_CROP_MODE.to_string() // Use default otherwise
        }),
        film_grain_sample_crop_mode: Some(config::DEFAULT_CROP_MODE.to_string()), // Always use default for sampling
        film_grain_metric_type: None, // Keep defaults for now
        film_grain_knee_threshold: None,
        film_grain_refinement_range_delta: None,
        film_grain_max_value: None,
        film_grain_refinement_points_count: None,
        optimize_film_grain: args.enable_grain_optimization, // Use the new flag directly
        film_grain_sample_duration: args.grain_sample_duration,
        film_grain_sample_count: args.grain_sample_count,
        film_grain_initial_values: args.grain_initial_values,
        film_grain_fallback_value: args.grain_fallback_value,
        ntfy_topic: args.ntfy, // Pass the ntfy topic URL from CLI args/env
        preset: args.preset.clone(), // Pass the preset from CLI args
    };

    // --- Execute Core Logic ---
    let processing_result: Result<Vec<EncodeResult>, CoreError>;
    log_callback(&format!("Processing {} file(s).", files_to_process.len())); // Use passed files_to_process
    if files_to_process.is_empty() {
         log_callback("No processable .mkv files found in the specified input path.");
         processing_result = Ok(Vec::new());
    } else {
         // Pass mutable reference to the dereferenced Box<dyn FnMut...>
         // Pass the Option<PathBuf> target_filename_override
         processing_result = drapto_core::process_videos(
             &config,
             &files_to_process,
             target_filename_override, // Pass the override
             &mut *log_callback
         );
    }

    // --- Handle Core Results ---
    let successfully_encoded: Vec<EncodeResult>;
    match processing_result {
        Ok(ref results) => {
            successfully_encoded = results.to_vec();
            if successfully_encoded.is_empty() && !matches!(processing_result, Err(CoreError::NoFilesFound)) {
                 log_callback("No files were successfully encoded.");
            } else if !successfully_encoded.is_empty() {
                 log_callback(&format!("Successfully encoded {} file(s).", successfully_encoded.len()));
            }
        }
        Err(e) => {
            log_callback(&format!("FATAL CORE ERROR during processing: {}", e));
            // logger is no longer directly accessible here, log_callback handles file write
            return Err(e.into());
        }
    }
 
    // --- Clean up temporary grain sampling directory ---
    let grain_tmp_dir = actual_output_dir.join("grain_samples_tmp");
    if grain_tmp_dir.exists() { // Check if it exists before trying to remove
        log_callback(&format!("[INFO] Cleaning up temporary directory: {}", grain_tmp_dir.display()));
        match fs::remove_dir_all(&grain_tmp_dir) {
            Ok(_) => log_callback(&format!("[INFO] Successfully removed temporary directory: {}", grain_tmp_dir.display())),
            Err(e) => log_callback(&format!("[WARN] Failed to remove temporary directory {}: {}", grain_tmp_dir.display(), e)),
        }
    }
 
 
    // --- Print Summary ---
    if !successfully_encoded.is_empty() {
        log_callback("========================================");
        log_callback("Encoding Summary:");
        log_callback("========================================");
        for result in &successfully_encoded {
            let reduction = if result.input_size > 0 {
                100u64.saturating_sub(result.output_size.saturating_mul(100) / result.input_size)
            } else {
                0
            };
            log_callback(&format!("{}", result.filename));
            log_callback(&format!("  Encode time: {}", drapto_core::format_duration(result.duration)));
            log_callback(&format!("  Input size:  {}", drapto_core::format_bytes(result.input_size)));
            log_callback(&format!("  Output size: {}", drapto_core::format_bytes(result.output_size)));
            log_callback(&format!("  Reduced by:  {}%", reduction));
            log_callback("----------------------------------------");
        }
    }

    // --- Final Timing ---
    let total_elapsed_time = total_start_time.elapsed();
    log_callback("========================================");
    log_callback(&format!("Total encode execution time: {}", drapto_core::format_duration(total_elapsed_time)));
    log_callback(&format!("Drapto Encode Run Finished: {}", chrono::Local::now()));
    log_callback("========================================");

    // Flushing the logger is handled implicitly when log_callback goes out of scope
    // or potentially needs explicit handling if create_log_callback changes.
    // For now, assume drop handles it.

    Ok(())
}