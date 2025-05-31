//! Implementation of the 'encode' subcommand.
//!
//! This module handles video file conversion to AV1 format, including file discovery,
//! configuration setup, and delegation to the drapto-core library.

use crate::cli::EncodeArgs;
use crate::error::CliResult;
use crate::terminal;

use drapto_core::notifications::NtfyNotificationSender;
use drapto_core::{CoreError, EncodeResult};

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use log::{debug, info, warn};

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
    let input_path = args
        .input_path
        .canonicalize()
        .map_err(|e| CoreError::PathError(
            format!("Invalid input path '{}': {}", args.input_path.display(), e)
        ))?;

    let metadata = fs::metadata(&input_path)
        .map_err(|e| CoreError::PathError(
            format!("Failed to access input path '{}': {}", input_path.display(), e)
        ))?;

    if metadata.is_dir() {
        match drapto_core::find_processable_files(&input_path) {
            Ok(files) => Ok((files, input_path.clone())),
            Err(CoreError::NoFilesFound) => Ok((Vec::new(), input_path.clone())),
            Err(e) => Err(e),
        }
    } else if metadata.is_file() {
        if input_path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("mkv"))
        {
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
        Err(CoreError::OperationFailed(format!(
            "Input path '{}' is neither a file nor a directory",
            input_path.display()
        )))
    }
}

/// Main function that handles the encoding process.
///
/// Sets up output directories, configures encoding parameters, processes videos
/// using drapto-core, and reports results.
pub fn run_encode(
    notification_sender: Option<&NtfyNotificationSender>,
    args: EncodeArgs,
    interactive: bool,
    files_to_process: Vec<PathBuf>,
    effective_input_dir: PathBuf,
) -> CliResult<()> {
    let total_start_time = Instant::now();

    let (actual_output_dir, target_filename_override_os) =
        if files_to_process.len() == 1 && args.output_dir.extension().is_some() {
            let target_file = args.output_dir.clone();
            let parent_dir = target_file
                .parent()
                .map(std::path::Path::to_path_buf)
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| PathBuf::from("."));
            let filename_os = target_file.file_name().map(std::ffi::OsStr::to_os_string);
            (parent_dir, filename_os)
        } else {
            (args.output_dir.clone(), None)
        };

    let target_filename_override = target_filename_override_os.map(PathBuf::from);

    let log_dir = args
        .log_dir
        .unwrap_or_else(|| actual_output_dir.join("logs"));

    // Create output directories (log dir may already exist in daemon mode)
    fs::create_dir_all(&actual_output_dir).map_err(|e| {
        CoreError::PathError(format!(
            "Failed to create output directory '{}': {}",
            actual_output_dir.display(),
            e
        ))
    })?;
    fs::create_dir_all(&log_dir)
        .map_err(|e| CoreError::PathError(
            format!("Failed to create log directory '{}': {}", log_dir.display(), e)
        ))?;

    let main_log_filename = format!("drapto_encode_run_{}.log", crate::logging::get_timestamp());
    let main_log_path = log_dir.join(&main_log_filename);
    let file_info = if files_to_process.is_empty() {
        None
    } else {
        drapto_core::get_media_info(&files_to_process[0]).ok()
    };

    let input_path_display = args.input_path.display().to_string();
    let output_display = if let Some(fname) = &target_filename_override {
        fname.display().to_string()
    } else {
        actual_output_dir.display().to_string()
    };

    let duration_display = if let Some(info) = &file_info {
        if let Some(duration_secs) = info.duration {
            let hours = (duration_secs / 3600.0) as u64;
            let minutes = ((duration_secs % 3600.0) / 60.0) as u64;
            let secs = (duration_secs % 60.0) as u64;
            let formatted = format!("{hours:02}:{minutes:02}:{secs:02}");
            Some(formatted)
        } else {
            None
        }
    } else {
        None
    };

    let resolution_display = if let Some(info) = &file_info {
        if let (Some(width), Some(height)) = (info.width, info.height) {
            if width > 0 && height > 0 {
                let resolution_type = if width >= 3840 {
                    "(UHD)"
                } else if width >= 1280 {
                    "(HD)"
                } else {
                    "(SD)"
                };

                Some(format!("{width}x{height} {resolution_type}"))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    terminal::print_section("INITIALIZATION");

    terminal::print_status("Input file", &input_path_display, false);
    terminal::print_status("Output file", &output_display, false);

    if let Some(duration) = duration_display {
        terminal::print_status("Duration", &duration, false);
    }

    if let Some(resolution) = resolution_display {
        terminal::print_status("Resolution", &resolution, false);
    }

    let hw_decode_info = drapto_core::hardware_decode::get_hardware_decoding_info();
    let hw_display = match hw_decode_info {
        Some(info) => format!("{info} (decode only)"),
        None => "No hardware decoder available".to_string(),
    };
    terminal::print_status("Hardware", &hw_display, false);

    debug!("Log file: {}", main_log_path.display());
    debug!("Interactive: {interactive}");
    debug!("Run started: {}", chrono::Local::now());

    if !interactive {
        let pid_path = log_dir.join("drapto.pid");
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



    let mut builder = drapto_core::config::CoreConfigBuilder::new()
        .input_dir(effective_input_dir)
        .output_dir(actual_output_dir.clone())
        .log_dir(log_dir.clone())
        .enable_denoise(!args.no_denoise);

    if let Some(quality) = args.quality_sd {
        builder = builder.quality_sd(quality);
    }

    if let Some(quality) = args.quality_hd {
        builder = builder.quality_hd(quality);
    }

    if let Some(quality) = args.quality_uhd {
        builder = builder.quality_uhd(quality);
    }

    let crop_mode = if args.disable_autocrop {
        "none"
    } else {
        drapto_core::config::DEFAULT_CROP_MODE
    };
    builder = builder.crop_mode(crop_mode);

    if let Some(topic) = args.ntfy {
        builder = builder.ntfy_topic(&topic);
    }

    if let Some(preset) = args.preset {
        builder = builder.encoder_preset(preset);
    }


    let config = builder.build();

    // Progress reporting is handled automatically through the ProgressReporter trait

    if !files_to_process.is_empty() {
        terminal::print_section("VIDEO ANALYSIS");
        terminal::print_processing_no_spacing(&format!(
            "Analyzing {} file(s)",
            files_to_process.len()
        ));

        let decode_status = if drapto_core::hardware_decode::is_hardware_decoding_available() {
            "Hardware (VideoToolbox)"
        } else {
            "Software"
        };
        terminal::print_status("Decoding", decode_status, false);
    }

    let processing_result = if files_to_process.is_empty() {
        warn!("Warning: No processable .mkv files found in the specified input path.");
        Ok(Vec::new())
    } else {
        drapto_core::process_videos(
            notification_sender,
            &config,
            &files_to_process,
            target_filename_override,
        )
        .map_err(|e| CoreError::OperationFailed(
            format!("Video processing failed: {}", e)
        ))
    };

    let successfully_encoded: Vec<EncodeResult>;
    match processing_result {
        Ok(ref results) => {
            successfully_encoded = results.clone();
            if successfully_encoded.is_empty() {
                terminal::print_error(
                    "No files encoded",
                    "No files were successfully encoded",
                    Some("Check that your input files are valid .mkv files"),
                );
            } else {
                terminal::print_section("ENCODING COMPLETE");
                terminal::print_success(&format!(
                    "Successfully encoded {} file(s)",
                    successfully_encoded.len()
                ));

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
                    terminal::print_status("Reduced by", &format!("{reduction:.1}%"), true);
                }
            }
        }
        Err(e) => {
            terminal::print_error("Fatal error during processing", &e.to_string(), None);
            return Err(e);
        }
    }

    debug!("Cleaning up temporary directories");

    if let Err(e) = drapto_core::temp_files::cleanup_base_dirs(&config)
        .map_err(|e| CoreError::OperationFailed(
            format!("Failed to clean up temporary directories: {}", e)
        ))
    {
        terminal::print_error("Cleanup warning", &e.to_string(), None);
    }

    if !successfully_encoded.is_empty() {
        terminal::print_section("Summary");

        for result in &successfully_encoded {
            let reduction = if result.input_size > 0 {
                100u64.saturating_sub(result.output_size.saturating_mul(100) / result.input_size)
            } else {
                0
            };

            if successfully_encoded.len() > 1 {
                terminal::print_subsection(&result.filename);
            }

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
            terminal::print_status("Reduction", &format!("{reduction}%"), true);
        }
    }

    let total_elapsed_time = total_start_time.elapsed();

    debug!("Finished at: {}", chrono::Local::now());

    terminal::print_status(
        "Total time",
        &drapto_core::format_duration(total_elapsed_time),
        true,
    );

    Ok(())
}
