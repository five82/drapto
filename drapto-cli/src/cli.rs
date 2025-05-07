// ============================================================================
// drapto-cli/src/cli.rs
// ============================================================================
//
// COMMAND-LINE INTERFACE: Argument Definitions
//
// This file defines the command-line interface for the Drapto CLI application
// using the clap crate. It includes the main CLI structure, subcommands, and
// all command-specific arguments with their descriptions and constraints.
//
// KEY COMPONENTS:
// - Cli: Main CLI structure with global flags
// - Commands: Enum of available subcommands
// - EncodeArgs: Arguments specific to the encode command
//
// USAGE EXAMPLES:
// - Basic: drapto encode -i input_dir -o output_dir
// - Advanced: drapto encode -i input.mkv -o output.av1.mkv --preset 6 --quality-hd 24 --ntfy https://ntfy.sh/topic
//
// AI-ASSISTANT-INFO: CLI argument definitions using clap, includes all command parameters

// ---- External crate imports ----
use clap::{Parser, Subcommand};

// ---- Standard library imports ----
use std::path::PathBuf;

// ============================================================================
// CLI ARGUMENT DEFINITIONS
// ============================================================================

/// Main CLI structure that defines the application's command-line interface.
///
/// This structure is the entry point for the clap parser and contains:
/// - Global flags that apply to all subcommands
/// - The subcommand enum that contains command-specific arguments
///
/// # Example
/// ```
/// drapto --interactive encode -i input_dir -o output_dir
/// ```
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
}

/// Enum of available subcommands for the Drapto CLI application.
///
/// Each variant represents a different operation that the application can perform,
/// and contains the arguments specific to that operation.
///
/// # Available Commands
/// - `Encode`: Convert video files to AV1 format
/// - (Future: analyze, config, etc.)
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
/// ```
/// drapto encode -i /path/to/videos -o /path/to/output
/// ```
///
/// Advanced usage with quality overrides:
/// ```
/// drapto encode -i input.mkv -o output.av1.mkv --quality-hd 24 --preset 6 --no-denoise
/// ```
#[derive(Parser, Debug)]
pub struct EncodeArgs {
    // ---- Required Arguments ----

    /// Input file or directory containing .mkv files.
    /// This can be either a single .mkv file or a directory containing multiple .mkv files.
    /// If a directory is provided, all .mkv files in the directory will be processed.
    #[arg(short = 'i', long = "input", required = true, value_name = "INPUT_PATH")]
    pub input_path: PathBuf,

    /// Directory where encoded files will be saved.
    /// If a single input file is provided and this argument has a file extension,
    /// it will be treated as the output filename instead of a directory.
    #[arg(short = 'o', long = "output", required = true, value_name = "OUTPUT_DIR")]
    pub output_dir: PathBuf,

    /// Optional: Directory for log files (defaults to OUTPUT_DIR/logs).
    /// Log files include detailed information about the encoding process.
    #[arg(short, long, value_name = "LOG_DIR")]
    pub log_dir: Option<PathBuf>,

    // ---- Quality Settings ----

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

    // ---- Processing Options ----

    /// Disable automatic crop detection (uses ffmpeg's cropdetect).
    /// By default, black bars are automatically detected and cropped.
    #[arg(long)]
    pub disable_autocrop: bool,

    /// Disable light video denoising (hqdn3d).
    /// By default, light denoising is applied to improve compression.
    #[arg(long, default_value_t = false)]
    pub no_denoise: bool,

    // ---- Notification Options ----

    /// Optional: ntfy.sh topic URL for sending notifications.
    /// Format: https://ntfy.sh/your_topic
    /// Can also be set via the DRAPTO_NTFY_TOPIC environment variable.
    #[arg(long, value_name = "TOPIC_URL", env = "DRAPTO_NTFY_TOPIC")]
    pub ntfy: Option<String>,
}

