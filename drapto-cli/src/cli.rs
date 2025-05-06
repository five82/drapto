// drapto-cli/src/cli.rs
//
// Defines the command-line argument structures using clap.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

// --- CLI Argument Definition ---

#[derive(Parser, Debug)]
#[command(
    author,
    version, // Reads from Cargo.toml via "cargo" feature in clap
    about = "Drapto: Video encoding tool",
    long_about = "Handles video encoding tasks using ffmpeg via drapto-core library."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Run in interactive mode (foreground) instead of daemonizing.
    #[arg(long, global = true, default_value_t = false)]
    pub interactive: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Encodes video files from an input directory to an output directory
    Encode(EncodeArgs),
    // Add other subcommands here later (e.g., analyze, config)
}

#[derive(Parser, Debug)]
pub struct EncodeArgs {
    /// Input file or directory containing .mkv files
    #[arg(short = 'i', long = "input", required = true, value_name = "INPUT_PATH")]
    pub input_path: PathBuf,

    /// Directory where encoded files will be saved
    #[arg(short = 'o', long = "output", required = true, value_name = "OUTPUT_DIR")]
    pub output_dir: PathBuf,

    /// Optional: Directory for log files (defaults to OUTPUT_DIR/logs)
    #[arg(short, long, value_name = "LOG_DIR")]
    pub log_dir: Option<PathBuf>,

    // --- Quality Overrides ---
    /// Optional: Override CRF quality for SD videos (<1920 width)
    #[arg(long, value_name = "CRF_SD")]
    pub quality_sd: Option<u8>,

    /// Optional: Override CRF quality for HD videos (>=1920 width)
    #[arg(long, value_name = "CRF_HD")]
    pub quality_hd: Option<u8>,

    /// Optional: Override CRF quality for UHD videos (>=3840 width)
    #[arg(long, value_name = "CRF_UHD")]
    pub quality_uhd: Option<u8>,

    // --- Notifications ---
    /// Optional: ntfy.sh topic URL for sending notifications (e.g., https://ntfy.sh/your_topic)
    /// Can also be set via the DRAPTO_NTFY_TOPIC environment variable.
    #[arg(long, value_name = "TOPIC_URL", env = "DRAPTO_NTFY_TOPIC")]
    pub ntfy: Option<String>,

    /// Optional: Override the ffmpeg libsvtav1 encoder preset (0-13, lower is slower/better quality)
    #[arg(long, value_name = "PRESET_INT", value_parser = clap::value_parser!(u8).range(0..=13))]
    pub preset: Option<u8>,

    /// Disable automatic crop detection (uses ffmpeg's cropdetect)
    #[arg(long)]
    pub disable_autocrop: bool,

/// Disable light video denoising (hqdn3d). Denoising is enabled by default.
    #[arg(long, default_value_t = false)] // Standard boolean flag
    pub no_denoise: bool, // Defaults to false, flag sets it to true
}

