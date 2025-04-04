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

// Renamed the main logic function to reflect the 'encode' action
pub fn run_encode(args: EncodeArgs) -> Result<(), Box<dyn std::error::Error>> { // Made public
    let total_start_time = Instant::now();

    // --- Determine Paths (using args from EncodeArgs) ---
    let input_path = args.input_path.canonicalize()
        .map_err(|e| format!("Invalid input path '{}': {}", args.input_path.display(), e))?;
    let output_dir = args.output_dir;
    let log_dir = args.log_dir.unwrap_or_else(|| output_dir.join("logs"));

    // --- Validate Input and Determine Files to Process ---
    let metadata = fs::metadata(&input_path)
        .map_err(|e| format!("Failed to access input path '{}': {}", input_path.display(), e))?;

    let (files_to_process, effective_input_dir) = if metadata.is_dir() {
        // Input is a directory
        match drapto_core::find_processable_files(&input_path) {
             Ok(files) => (files, input_path.clone()),
             Err(CoreError::NoFilesFound) => (Vec::new(), input_path.clone()),
             Err(e) => return Err(e.into()),
        }
    } else if metadata.is_file() {
        // Input is a file
        if input_path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("mkv")) {
            let parent_dir = input_path.parent().ok_or_else(|| {
                CoreError::PathError(format!("Could not determine parent directory for file '{}'", input_path.display()))
            })?.to_path_buf();
            (vec![input_path.clone()], parent_dir)
        } else {
            return Err(format!("Input file '{}' is not a .mkv file.", input_path.display()).into());
        }
    } else {
        return Err(format!("Input path '{}' is neither a file nor a directory.", input_path.display()).into());
    };

    // --- Create Output/Log Dirs ---
    fs::create_dir_all(&output_dir)?;
    fs::create_dir_all(&log_dir)?;

    // --- Setup Logging ---
    let main_log_filename = format!("drapto_encode_run_{}.log", get_timestamp());
    let main_log_path = log_dir.join(main_log_filename);
    let log_file = File::create(&main_log_path)?;
    let mut log_callback = create_log_callback(log_file)?; // Use the helper

    // --- Log Initial Info ---
    log_callback("========================================");
    log_callback(&format!("Drapto Encode Run Started: {}", chrono::Local::now()));
    log_callback(&format!("Input path: {}", input_path.display()));
    log_callback(&format!("Output directory: {}", output_dir.display()));
    log_callback(&format!("Log directory: {}", log_dir.display()));
    log_callback(&format!("Main log file: {}", main_log_path.display()));
    log_callback("========================================");

    // --- Prepare Core Configuration ---
    let config = CoreConfig {
        input_dir: effective_input_dir,
        output_dir: output_dir.clone(),
        log_dir: log_dir.clone(),
        default_encoder_preset: Some(config::DEFAULT_ENCODER_PRESET as u8),
        quality_sd: args.quality_sd,
        quality_hd: args.quality_hd,
        quality_uhd: args.quality_uhd,
        default_crop_mode: Some(config::DEFAULT_CROP_MODE.to_string()),
        film_grain_metric_type: None, // Keep defaults for now
        film_grain_knee_threshold: None,
        film_grain_refinement_range_delta: None,
        film_grain_max_value: None,
        film_grain_refinement_points_count: None,
        optimize_film_grain: !args.disable_grain_optimization,
        film_grain_sample_duration: args.grain_sample_duration,
        film_grain_sample_count: args.grain_sample_count,
        film_grain_initial_values: args.grain_initial_values,
        film_grain_fallback_value: args.grain_fallback_value,
    };

    // --- Execute Core Logic ---
    let processing_result: Result<Vec<EncodeResult>, CoreError>;
    log_callback(&format!("Found {} file(s) to process.", files_to_process.len()));
    if files_to_process.is_empty() {
         log_callback("No processable .mkv files found in the specified input path.");
         processing_result = Ok(Vec::new());
    } else {
         processing_result = drapto_core::process_videos(&config, &files_to_process, &mut *log_callback); // Deref Box
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