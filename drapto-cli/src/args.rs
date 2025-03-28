//! Command-line argument parsing for Drapto
//!
//! Responsibilities:
//! - Define the command-line interface structure
//! - Parse and validate user-provided arguments
//! - Provide help and documentation for CLI options
//! - Define command subgroups and their parameters
//!
//! This module uses clap to define a structured CLI with commands for
//! encoding, validation, and system information retrieval.

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

    /// Set log level
    #[arg(
        long, 
        help = "Set logging level (debug, info, warn, error)", 
        value_parser=["debug", "info", "warn", "error"],
        default_value = "info"
    )]
    pub log_level: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Encode a video file or directory with optimal settings
    Encode {
        /// Input file or directory path
        input: PathBuf,
        
        /// Output file or directory path
        output: PathBuf,
        
        /// Target VMAF quality for SDR content (0-100)
        #[arg(
            short,
            long,
            help = "Target video quality on VMAF scale (0-100) for SDR content (HDR content uses CRF by default)"
        )]
        quality: Option<f32>,
        
        
        /// Use CRF as quality metric for all content (HDR content uses CRF by default)
        #[arg(
            long,
            help = "Force CRF mode for all content (Note: HDR content automatically uses CRF by default)",
            default_value = "false"
        )]
        use_crf: bool,
        
        /// CRF value for standard definition
        #[arg(
            long,
            help = "CRF value for standard definition content (0-63, lower is better quality)",
            default_value = "25"
        )]
        crf_sd: u8,
        
        /// CRF value for high definition
        #[arg(
            long,
            help = "CRF value for high definition content (0-63, lower is better quality)",
            default_value = "28"
        )]
        crf_hd: u8,
        
        /// CRF value for 4K content
        #[arg(
            long,
            help = "CRF value for 4K content (0-63, lower is better quality)",
            default_value = "28"
        )]
        crf_4k: u8,
        
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

        /// Disable automatic crop detection
        #[arg(
            long,
            help = "Disable automatic crop detection",
            default_value = "false"
        )]
        disable_crop: bool,

        /// Memory limit per encoding job in MB
        #[arg(
            long,
            help = "Memory limit per encoding job in MB (default: 2048, auto-adjusted based on encoder and resolution)"
        )]
        memory_per_job: Option<usize>,
    },
    
    /// Check if FFmpeg is available and print details about capabilities
    #[command(
        name = "info",
        about = "Display information about the FFmpeg installation and capabilities"
    )]
    FfmpegInfo,
    
    /// Validate a media file for encoding compatibility
    Validate {
        /// Input file path
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