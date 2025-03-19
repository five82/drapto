use clap::{Parser, Subcommand};
use log::{debug, info};
use std::path::PathBuf;

use drapto::config::Config;
use drapto::error::Result;
use drapto::ffprobe::{FFprobe, MediaInfo};
use drapto::validation;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Encode a video file
    Encode {
        /// Input file path
        #[arg(short, long)]
        input: PathBuf,
        
        /// Output file path
        #[arg(short, long)]
        output: PathBuf,
        
        /// Target VMAF quality (0-100)
        #[arg(short, long)]
        quality: Option<f32>,
        
        /// Number of parallel encoding jobs
        #[arg(short, long)]
        jobs: Option<usize>,
        
        /// Disable hardware acceleration
        #[arg(long)]
        no_hwaccel: bool,
        
        /// Keep temporary files after encoding
        #[arg(long)]
        keep_temp: bool,
        
        /// Temporary directory for intermediate files
        #[arg(long)]
        temp_dir: Option<PathBuf>,
    },
    
    /// Check if FFmpeg is available and print version info
    FfmpegInfo,
    
    /// Validate a media file
    Validate {
        /// Input file path
        #[arg(short, long)]
        input: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logger
    drapto::logging::init(cli.verbose);
    
    info!("Drapto v{} starting up", drapto::VERSION);
    
    match cli.command {
        Commands::Encode {
            input,
            output,
            quality,
            jobs,
            no_hwaccel,
            keep_temp,
            temp_dir,
        } => {
            // Configure encoding options
            let mut config = Config::new();
            config.input = input;
            config.output = output;
            config.target_quality = quality;
            config.hardware_acceleration = !no_hwaccel;
            config.keep_temp_files = keep_temp;
            config.verbose = cli.verbose;
            
            if let Some(jobs) = jobs {
                config.parallel_jobs = jobs;
            }
            
            if let Some(temp_dir) = temp_dir {
                config.temp_dir = temp_dir;
            }
            
            debug!("Configuration: {:?}", config);
            
            // Validate configuration
            config.validate()?;
            
            // Execute the encoding
            encode(config)?;
        },
        Commands::FfmpegInfo => {
            if FFprobe::is_available() {
                let version = FFprobe::version()?;
                println!("FFmpeg is available: {}", version);
            } else {
                println!("FFmpeg is not available on this system");
            }
        },
        Commands::Validate { input } => {
            if !input.exists() {
                return Err(drapto::error::DraptoError::MediaFile(
                    format!("File not found: {:?}", input)
                ));
            }
            
            println!("Validating: {:?}", input);
            
            // Run validation and print report
            let report = validation::validate_media(&input)?;
            println!("\nValidation Report:");
            println!("{}", report);
            
            // Also run A/V sync validation
            let av_sync_report = validation::validate_av_sync(&input)?;
            println!("\nA/V Sync Report:");
            println!("{}", av_sync_report);
        }
    }
    
    info!("Drapto completed successfully");
    Ok(())
}

fn encode(config: Config) -> Result<()> {
    info!("Encoding: {:?} -> {:?}", config.input, config.output);
    info!("Target VMAF quality: {:?}", config.target_quality);
    info!("Using {} parallel jobs", config.parallel_jobs);
    
    // Get media information
    let media_info = MediaInfo::from_path(&config.input)?;
    
    println!("Input file information:");
    println!("Format: {}", media_info.format.format_name);
    println!("Duration: {} seconds", media_info.duration().unwrap_or(0.0));
    println!("Video streams: {}", media_info.video_streams().len());
    println!("Audio streams: {}", media_info.audio_streams().len());
    
    if let Some(dimensions) = media_info.video_dimensions() {
        println!("Video dimensions: {}x{}", dimensions.0, dimensions.1);
    }
    
    println!("HDR content: {}", if media_info.is_hdr() { "Yes" } else { "No" });
    println!("Dolby Vision: {}", if media_info.is_dolby_vision() { "Yes" } else { "No" });
    
    // Placeholder for the actual encoding logic
    // This will be implemented in Phase 4
    
    info!("Encoding not yet implemented - this is a skeleton project");
    Ok(())
}