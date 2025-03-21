mod args;
mod commands;
mod output;

use clap::Parser;
use drapto_core::error::Result;
use drapto_core::logging;
use log::info;

use args::{Cli, Commands};
use commands::{execute_encode, execute_validate, execute_ffmpeg_info};

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logger
    logging::init(cli.verbose);
    
    info!("Drapto starting up");
    
    match &cli.command {
        Commands::Encode {
            input,
            output,
            quality,
            jobs,
            no_hwaccel,
            keep_temp,
            temp_dir,
        } => {
            execute_encode(
                input.clone(),
                output.clone(),
                *quality,
                *jobs,
                *no_hwaccel,
                *keep_temp,
                temp_dir.clone(),
                cli.verbose,
            )
        },
        Commands::FfmpegInfo => {
            execute_ffmpeg_info()
        },
        Commands::Validate { input, reference, target_score } => {
            execute_validate(input.clone(), reference.clone(), *target_score)
        }
    }
}