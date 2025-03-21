use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Drapto - Distributed and Reliable Automated Parallel Transcoding Optimizer",
    long_about = "A video encoding tool with parallel processing capabilities, \
                 scene detection, and automated quality optimization."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    
    /// Enable verbose logging
    #[arg(short, long, help = "Enable detailed logging output")]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Encode a video file with optimal settings
    Encode {
        /// Input file path
        #[arg(short, long, help = "Path to the input video file")]
        input: PathBuf,
        
        /// Output file path
        #[arg(short, long, help = "Path where the encoded video will be saved")]
        output: PathBuf,
        
        /// Target VMAF quality (0-100)
        #[arg(
            short,
            long,
            help = "Target video quality on VMAF scale (0-100, higher is better)"
        )]
        quality: Option<f32>,
        
        /// Number of parallel encoding jobs
        #[arg(
            short,
            long,
            help = "Number of encoding jobs to run in parallel (default: number of CPU cores)"
        )]
        jobs: Option<usize>,
        
        /// Disable hardware acceleration
        #[arg(
            long,
            help = "Disable hardware acceleration even if available",
            default_value = "false"
        )]
        no_hwaccel: bool,
        
        /// Keep temporary files after encoding
        #[arg(
            long,
            help = "Keep temporary files after encoding completes",
            default_value = "false"
        )]
        keep_temp: bool,
        
        /// Temporary directory for intermediate files
        #[arg(
            long,
            help = "Directory to store temporary files during encoding (default: system temp dir)"
        )]
        temp_dir: Option<PathBuf>,
    },
    
    /// Check if FFmpeg is available and print details about capabilities
    #[command(
        name = "ffmpeg-info",
        about = "Display information about the FFmpeg installation and capabilities"
    )]
    FfmpegInfo,
    
    /// Validate a media file for encoding compatibility
    Validate {
        /// Input file path
        #[arg(
            short,
            long,
            help = "Path to the media file to validate"
        )]
        input: PathBuf,
        
        /// Reference file for VMAF validation
        #[arg(
            short,
            long,
            help = "Optional reference file for VMAF quality validation"
        )]
        reference: Option<PathBuf>,
        
        /// Target VMAF score
        #[arg(
            short,
            long,
            help = "Target VMAF score for quality validation (0-100, higher is better)",
            default_value = "90.0"
        )]
        target_score: f32,
    },
}