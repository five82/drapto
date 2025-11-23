//! Command-line interface definitions for Drapto.
//!
//! This module defines the CLI structure using clap, including all commands,
//! subcommands, and their associated arguments.

use clap::{Command, CommandFactory, FromArgMatches, Parser, Subcommand};
use drapto_core::config::{
    DEFAULT_CORE_QUALITY_HD, DEFAULT_CORE_QUALITY_SD, DEFAULT_CORE_QUALITY_UHD,
    DEFAULT_SVT_AV1_PRESET, DraptoPreset,
};
use std::env;
use std::ffi::OsString;
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;

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
#[derive(Parser, Debug, Clone)]
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

    /// Drapto preset grouping core quality/tuning defaults.
    #[arg(
        long = "drapto-preset",
        value_name = "PRESET",
        value_parser = parse_drapto_preset
    )]
    pub drapto_preset: Option<DraptoPreset>,

    // Processing Options
    /// Disable automatic black bar crop detection.
    #[arg(long)]
    pub disable_autocrop: bool,

    /// Reserve CPU threads for improved system responsiveness during encoding.
    #[arg(long, default_value_t = false)]
    pub responsive: bool,

    /// Output progress as structured JSON to stdout for machine parsing (automatically runs in foreground).
    #[arg(long)]
    pub progress_json: bool,
}

/// Parse CLI arguments while dynamically embedding core defaults into the help text.
pub fn parse_cli() -> Cli {
    parse_cli_from(env::args_os())
}

/// Parse CLI arguments from a custom iterator (primarily for testing).
pub fn parse_cli_from<I, T>(itr: I) -> Cli
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let command = command_with_dynamic_defaults();
    let matches = command
        .try_get_matches_from(itr)
        .unwrap_or_else(|err| err.exit());

    Cli::from_arg_matches(&matches).unwrap_or_else(|err| err.exit())
}

fn command_with_dynamic_defaults() -> Command {
    apply_encode_default_help(Cli::command())
}

fn apply_encode_default_help(command: Command) -> Command {
    command.mut_subcommand("encode", |encode_cmd| {
        encode_cmd.mut_args(|arg| match arg.get_long() {
            Some("quality-sd") => {
                arg.help(help_with_default(QUALITY_SD_HELP, DEFAULT_CORE_QUALITY_SD))
            }
            Some("quality-hd") => {
                arg.help(help_with_default(QUALITY_HD_HELP, DEFAULT_CORE_QUALITY_HD))
            }
            Some("quality-uhd") => arg.help(help_with_default(
                QUALITY_UHD_HELP,
                DEFAULT_CORE_QUALITY_UHD,
            )),
            Some("preset") => arg.help(help_with_default(PRESET_HELP, DEFAULT_SVT_AV1_PRESET)),
            Some("drapto-preset") => arg.help(DRAPTO_PRESET_HELP),
            _ => arg,
        })
    })
}

const QUALITY_SD_HELP: &str =
    "CRF quality for SD videos (<1920 width). Lower=better quality, larger files.";
const QUALITY_HD_HELP: &str =
    "CRF quality for HD videos (≥1920 width). Lower=better quality, larger files.";
const QUALITY_UHD_HELP: &str =
    "CRF quality for UHD videos (≥3840 width). Lower=better quality, larger files.";
const PRESET_HELP: &str = "Encoder preset (0-13). Lower=slower/better, higher=faster.";
const DRAPTO_PRESET_HELP: &str =
    "Apply grouped Drapto defaults (grain, clean, quick). Later flags can override.";

fn help_with_default<T: Display>(base: &str, default: T) -> String {
    format!("{base} Default: {default}.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_help_includes_core_defaults() {
        let mut command = command_with_dynamic_defaults();
        let encode_help = command
            .get_subcommands_mut()
            .find(|sub| sub.get_name() == "encode")
            .expect("encode subcommand missing");
        let help = encode_help.render_long_help().to_string();

        for default in [
            DEFAULT_CORE_QUALITY_SD.to_string(),
            DEFAULT_CORE_QUALITY_HD.to_string(),
            DEFAULT_CORE_QUALITY_UHD.to_string(),
            DEFAULT_SVT_AV1_PRESET.to_string(),
        ] {
            let needle = format!("Default: {default}");
            assert!(
                help.contains(&needle),
                "Expected help output to contain `{needle}`, but it was missing.\nHelp:\n{help}"
            );
        }
    }
}

fn parse_drapto_preset(value: &str) -> Result<DraptoPreset, String> {
    DraptoPreset::from_str(value).map_err(|err| err.to_string())
}
