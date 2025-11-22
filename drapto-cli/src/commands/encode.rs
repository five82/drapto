//! Encode command implementation with direct reporter integration.

use crate::EncodeArgs;
use crate::error::CliResult;

use drapto_core::{
    CoreConfig, CoreError,
    discovery::find_processable_files,
    processing::process_videos,
    reporting::{Reporter, ReporterError},
    utils::{SafePath, calculate_size_reduction, validate_paths},
};

use std::path::PathBuf;
use std::time::SystemTime;

/// Discover video files to encode based on the provided arguments
pub fn discover_encode_files(args: &EncodeArgs) -> CliResult<(Vec<PathBuf>, PathBuf)> {
    let input_path = &args.input_path;

    // Validate paths early
    validate_paths(input_path, &args.output_dir)?;

    if input_path.is_file() {
        // Single file input - get parent directory safely
        let effective_input_dir = SafePath::get_parent_safe(input_path)?.to_path_buf();
        Ok((vec![input_path.clone()], effective_input_dir))
    } else if input_path.is_dir() {
        // Directory input - find all processable files
        let files = find_processable_files(input_path)?;
        Ok((files, input_path.clone()))
    } else {
        Err(CoreError::PathError(format!(
            "Input path does not exist: {}",
            input_path.display()
        )))
    }
}

/// Run the encode command with the event-based architecture
pub fn run_encode(
    args: EncodeArgs,
    discovered_files: Vec<PathBuf>,
    effective_input_dir: PathBuf,
    target_filename_override: Option<std::ffi::OsString>,
    reporter: &dyn Reporter,
) -> CliResult<()> {
    let start_time = SystemTime::now();

    // Convert args to CoreConfig
    let mut config = CoreConfig::new(
        effective_input_dir.clone(),
        args.output_dir.clone(),
        args.log_dir
            .clone()
            .unwrap_or_else(|| args.output_dir.join("logs")),
    );

    // Apply command line arguments to config
    config.responsive_encoding = args.responsive;

    if args.responsive {
        log::info!(
            "Responsive mode enabled: reserving SVT-AV1 threads to keep the system interactive"
        );
    }

    if let Some(quality) = args.quality_sd {
        config.quality_sd = quality;
    }

    if let Some(quality) = args.quality_hd {
        config.quality_hd = quality;
    }

    if let Some(quality) = args.quality_uhd {
        config.quality_uhd = quality;
    }

    config.crop_mode = if args.disable_autocrop {
        "none".to_string()
    } else {
        drapto_core::config::DEFAULT_CROP_MODE.to_string()
    };

    if let Some(preset) = args.preset {
        config.svt_av1_preset = preset;
    }

    // Validate configuration
    config.validate()?;

    let results = if discovered_files.is_empty() {
        reporter.warning("No video files found to process");
        vec![]
    } else {
        // Process videos with the reporter
        let target_filename = target_filename_override.map(PathBuf::from);
        match process_videos(&config, &discovered_files, target_filename, Some(reporter)) {
            Ok(results) => results,
            Err(e) => {
                reporter.error(&ReporterError {
                    title: "Processing failed".to_string(),
                    message: e.to_string(),
                    context: None,
                    suggestion: None,
                });
                return Err(e);
            }
        }
    };

    // Summary
    let elapsed = start_time.elapsed().unwrap_or_default();
    let total_duration = format!(
        "{:02}:{:02}:{:02}",
        elapsed.as_secs() / 3600,
        (elapsed.as_secs() % 3600) / 60,
        elapsed.as_secs() % 60
    );

    if !results.is_empty() {
        log::info!("");
        log::info!("===== BATCH SUMMARY =====");
        log::info!("");
        log::info!("Total files processed: {}", results.len());
        log::info!("Total time: {}", total_duration);

        let total_input_size: u64 = results.iter().map(|r| r.input_size).sum();
        let total_output_size: u64 = results.iter().map(|r| r.output_size).sum();
        let total_reduction = calculate_size_reduction(total_input_size, total_output_size) as f64;

        log::info!(
            "Total size reduction: {:.1}% ({} → {})",
            total_reduction,
            drapto_core::format_bytes(total_input_size),
            drapto_core::format_bytes(total_output_size)
        );
    }

    Ok(())
}
