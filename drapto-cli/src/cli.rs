//! Command-line interface definitions for Drapto.
//!
//! This module defines the CLI structure using clap, including all commands,
//! subcommands, and their associated arguments.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Main CLI structure with global flags and subcommands.
#[derive(Parser, Debug)]
#[command(
    author,                                                      // Author from Cargo.toml
    version,                                                     // Version from Cargo.toml
    about = "Drapto: Video encoding tool",                       // Short description
    long_about = "Handles video encoding tasks using ffmpeg via drapto-core library." // Detailed description
)]
pub struct Cli {
    /// The subcommand to execute (e.g., encode)
    #[command(subcommand)]
    pub command: Commands,

    /// Run in interactive mode (foreground) instead of daemonizing.
    /// When this flag is present, the application runs in the foreground
    /// and logs directly to the console instead of running as a daemon.
    #[arg(long, global = true, default_value_t = false)]
    pub interactive: bool,

    /// Enable verbose output with detailed information.
    /// Shows additional technical details for troubleshooting.
    #[arg(short, long, global = true, default_value_t = false)]
    pub verbose: bool,

}

/// Available CLI subcommands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Encodes video files from an input directory to an output directory.
    /// This command converts video files (typically .mkv) to AV1 format
    /// with configurable quality settings and processing options.
    Encode(EncodeArgs),
    // Future subcommands will be added here as the application evolves
    // Examples:
    // - Analyze: Analyze video files without encoding
    // - Config: Manage application configuration
}

/// Arguments for the `encode` command.
///
/// This structure defines all the parameters that can be passed to the encode command,
/// including input/output paths, quality settings, and processing options.
///
/// # Examples
///
/// Basic usage:
/// ```no_run
/// // Command-line: drapto encode -i /path/to/videos -o /path/to/output
/// use drapto_cli::cli::{EncodeArgs, Commands};
/// use std::path::PathBuf;
///
/// let encode_args = EncodeArgs {
///     input_path: PathBuf::from("/path/to/videos"),
///     output_dir: PathBuf::from("/path/to/output"),
///     log_dir: None,
///     quality_sd: None,
///     quality_hd: None,
///     quality_uhd: None,
///     preset: None,
///     disable_autocrop: false,
///     no_denoise: false,
///     ntfy: None,
/// };
/// ```
///
/// Advanced usage with quality overrides:
/// ```no_run
/// // Command-line: drapto encode -i input.mkv -o output.av1.mkv --quality-hd 24 --preset 6 --no-denoise
/// use drapto_cli::cli::EncodeArgs;
/// use std::path::PathBuf;
///
/// let encode_args = EncodeArgs {
///     input_path: PathBuf::from("input.mkv"),
///     output_dir: PathBuf::from("output.av1.mkv"),
///     log_dir: None,
///     quality_sd: None,
///     quality_hd: Some(24),
///     quality_uhd: None,
///     preset: Some(6),
///     disable_autocrop: false,
///     no_denoise: true,
///     ntfy: None,
/// };
/// ```
#[derive(Parser, Debug)]
pub struct EncodeArgs {
    // Required Arguments
    /// Input file or directory containing .mkv files.
    /// This can be either a single .mkv file or a directory containing multiple .mkv files.
    /// If a directory is provided, all .mkv files in the directory will be processed.
    #[arg(
        short = 'i',
        long = "input",
        required = true,
        value_name = "INPUT_PATH"
    )]
    pub input_path: PathBuf,

    /// Directory where encoded files will be saved.
    /// If a single input file is provided and this argument has a file extension,
    /// it will be treated as the output filename instead of a directory.
    #[arg(
        short = 'o',
        long = "output",
        required = true,
        value_name = "OUTPUT_DIR"
    )]
    pub output_dir: PathBuf,

    /// Optional: Directory for log files (defaults to `OUTPUT_DIR/logs`).
    /// Log files include detailed information about the encoding process.
    #[arg(short, long, value_name = "LOG_DIR")]
    pub log_dir: Option<PathBuf>,

    // Quality Settings
    /// Optional: Override CRF quality for SD videos (<1920 width).
    /// Lower values produce higher quality but larger files.
    /// Typical range: 18-30 (default is determined by resolution).
    #[arg(long, value_name = "CRF_SD")]
    pub quality_sd: Option<u8>,

    /// Optional: Override CRF quality for HD videos (>=1920 width).
    /// Lower values produce higher quality but larger files.
    /// Typical range: 18-30 (default is determined by resolution).
    #[arg(long, value_name = "CRF_HD")]
    pub quality_hd: Option<u8>,

    /// Optional: Override CRF quality for UHD videos (>=3840 width).
    /// Lower values produce higher quality but larger files.
    /// Typical range: 18-30 (default is determined by resolution).
    #[arg(long, value_name = "CRF_UHD")]
    pub quality_uhd: Option<u8>,

    /// Optional: Override the ffmpeg libsvtav1 encoder preset (0-13).
    /// Lower values are slower but produce better quality.
    /// Higher values are faster but may reduce quality.
    #[arg(long, value_name = "PRESET_INT", value_parser = clap::value_parser!(u8).range(0..=13))]
    pub preset: Option<u8>,

    // Processing Options
    /// Disable automatic crop detection (uses ffmpeg's cropdetect).
    /// By default, black bars are automatically detected and cropped.
    #[arg(long)]
    pub disable_autocrop: bool,

    /// Disable light video denoising (hqdn3d).
    /// By default, light denoising is applied to improve compression.
    #[arg(long, default_value_t = false)]
    pub no_denoise: bool,


    // Notification Options
    /// Optional: ntfy.sh topic URL for sending notifications.
    /// Format: <https://ntfy.sh/your_topic>
    /// Can also be set via the `DRAPTO_NTFY_TOPIC` environment variable.
    #[arg(long, value_name = "TOPIC_URL", env = "DRAPTO_NTFY_TOPIC")]
    pub ntfy: Option<String>,
}
