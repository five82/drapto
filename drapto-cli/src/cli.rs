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

    /// Run in foreground instead of daemonizing.
    #[arg(long, global = true, default_value_t = false)]
    pub foreground: bool,

    /// Enable verbose output for troubleshooting.
    #[arg(short, long, global = true, default_value_t = false)]
    pub verbose: bool,

}

/// Available CLI subcommands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Encode video files to AV1 format.
    Encode(EncodeArgs),
    // Future subcommands will be added here as the application evolves
    // Examples:
    // - Analyze: Analyze video files without encoding
    // - Config: Manage application configuration
}

/// Arguments for the encode command.
#[derive(Parser, Debug)]
pub struct EncodeArgs {
    // Required Arguments
    /// Input video file or directory containing video files.
    #[arg(
        short = 'i',
        long = "input",
        required = true,
        value_name = "INPUT_PATH"
    )]
    pub input_path: PathBuf,

    /// Output directory (or filename if input is a single file with extension).
    #[arg(
        short = 'o',
        long = "output",
        required = true,
        value_name = "OUTPUT_DIR"
    )]
    pub output_dir: PathBuf,

    /// Log directory (defaults to OUTPUT_DIR/logs).
    #[arg(short, long, value_name = "LOG_DIR")]
    pub log_dir: Option<PathBuf>,

    // Quality Settings
    /// CRF quality for SD videos (<1920 width). Lower=better quality, larger files.
    #[arg(long, value_name = "CRF_SD")]
    pub quality_sd: Option<u8>,

    /// CRF quality for HD videos (≥1920 width). Lower=better quality, larger files.
    #[arg(long, value_name = "CRF_HD")]
    pub quality_hd: Option<u8>,

    /// CRF quality for UHD videos (≥3840 width). Lower=better quality, larger files.
    #[arg(long, value_name = "CRF_UHD")]
    pub quality_uhd: Option<u8>,

    /// Encoder preset (0-13). Lower=slower/better, higher=faster.
    #[arg(long, value_name = "PRESET_INT", value_parser = clap::value_parser!(u8).range(0..=13))]
    pub preset: Option<u8>,

    // Processing Options
    /// Disable automatic black bar crop detection.
    #[arg(long)]
    pub disable_autocrop: bool,

    /// Disable light denoising filter.
    #[arg(long, default_value_t = false)]
    pub no_denoise: bool,


    // Notification Options
    /// ntfy.sh topic URL for notifications (e.g., https://ntfy.sh/your_topic).
    #[arg(long, value_name = "TOPIC_URL", env = "DRAPTO_NTFY_TOPIC")]
    pub ntfy: Option<String>,

    /// Output progress as structured JSON to stdout for machine parsing (cannot be used with --foreground).
    #[arg(long)]
    pub progress_json: bool,
}
