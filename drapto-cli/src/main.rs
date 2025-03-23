//! Drapto CLI Application Entry Point
//!
//! Responsibilities:
//! - Parse command line arguments
//! - Initialize logging subsystem
//! - Dispatch to appropriate command handlers
//! - Report overall execution status and timing
//!
//! This is the main entry point for the drapto command-line tool, which
//! provides a user-friendly interface to the drapto-core functionality.

mod args;
mod commands;
mod output;

use clap::Parser;
use drapto_core::error::{Result, DraptoError};
use drapto_core::logging;
use log::{info, error, LevelFilter};
use std::time::Instant;

use args::{Cli, Commands};
use commands::{execute_encode, execute_validate, execute_ffmpeg_info, execute_encode_directory};
use output::{print_success, print_separator};

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logger with proper level
    let log_level = match cli.log_level.to_lowercase().as_str() {
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => LevelFilter::Info,
    };
    
    logging::init_with_level(log_level, cli.verbose);
    
    info!("Drapto starting up");
    
    let start_time = Instant::now();
    
    let result = match &cli.command {
        Commands::Encode {
            input,
            output,
            quality,
            jobs,
            no_hwaccel,
            keep_temp,
            temp_dir,
            disable_crop,
            memory_per_job,
        } => {
            if input.is_file() {
                // File to file encoding
                execute_encode(
                    input.clone(),
                    output.clone(),
                    *quality,
                    *jobs,
                    *no_hwaccel,
                    *keep_temp,
                    temp_dir.clone(),
                    *disable_crop,
                    cli.verbose,
                    *memory_per_job,
                )
            } else if input.is_dir() {
                // Directory to directory encoding
                if !output.exists() && output.extension().is_none() {
                    std::fs::create_dir_all(output)?;
                }
                
                if !output.is_dir() {
                    error!("When input is a directory, output must also be a directory");
                    return Err(DraptoError::InvalidInput("Output must be a directory when input is a directory".to_string()));
                }
                
                execute_encode_directory(
                    input.clone(),
                    output.clone(),
                    *quality,
                    *jobs,
                    *no_hwaccel,
                    *keep_temp,
                    temp_dir.clone(),
                    *disable_crop,
                    cli.verbose,
                    *memory_per_job,
                )
            } else {
                error!("Input path does not exist: {}", input.display());
                Err(DraptoError::InputNotFound(input.to_string_lossy().to_string()))
            }
        },
        Commands::FfmpegInfo => {
            execute_ffmpeg_info()
        },
        Commands::Validate { input, reference, target_score } => {
            if !input.exists() {
                error!("Input path does not exist: {}", input.display());
                return Err(DraptoError::InputNotFound(input.to_string_lossy().to_string()));
            }
            
            execute_validate(input.clone(), reference.clone(), *target_score)
        }
    };
    
    // Calculate elapsed time and show it if operation was successful
    if result.is_ok() {
        let elapsed = start_time.elapsed();
        let hours = elapsed.as_secs() / 3600;
        let minutes = (elapsed.as_secs() % 3600) / 60;
        let seconds = elapsed.as_secs() % 60;
        
        print_separator();
        print_success(&format!("Total execution time: {:02}h {:02}m {:02}s", hours, minutes, seconds));
    }
    
    result
}