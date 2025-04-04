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
    long_about = "Handles video encoding tasks using HandBrakeCLI via drapto-core library."
)]
pub struct Cli { // Made public
    #[command(subcommand)]
    pub command: Commands, // Made public
}

#[derive(Subcommand, Debug)]
pub enum Commands { // Made public
    /// Encodes video files from an input directory to an output directory
    Encode(EncodeArgs),
    // Add other subcommands here later (e.g., analyze, config)
}

#[derive(Parser, Debug)] // Use Parser derive for args struct as well
pub struct EncodeArgs { // Made public
    /// Input file or directory containing .mkv files
    #[arg(required = true, value_name = "INPUT_PATH")]
    pub input_path: PathBuf, // Made public

    /// Directory where encoded files will be saved
    #[arg(required = true, value_name = "OUTPUT_DIR")]
    pub output_dir: PathBuf, // Made public

    /// Optional: Directory for log files (defaults to OUTPUT_DIR/logs)
    #[arg(short, long, value_name = "LOG_DIR")]
    pub log_dir: Option<PathBuf>, // Made public

    // --- Quality Overrides ---
    /// Optional: Override CRF quality for SD videos (<1920 width)
    #[arg(long, value_name = "CRF_SD")]
    pub quality_sd: Option<u8>, // Made public

    /// Optional: Override CRF quality for HD videos (>=1920 width)
    #[arg(long, value_name = "CRF_HD")]
    pub quality_hd: Option<u8>, // Made public

    /// Optional: Override CRF quality for UHD videos (>=3840 width)
    #[arg(long, value_name = "CRF_UHD")]
    pub quality_uhd: Option<u8>, // Made public

    // --- Film Grain Optimization Flags ---
    /// Disable automatic film grain optimization (it's enabled by default)
    #[arg(long)]
    pub disable_grain_optimization: bool, // Made public
    /// Duration (seconds) for each optimization sample clip
    #[arg(long, value_name = "SECONDS")]
    pub grain_sample_duration: Option<u32>, // Made public
    /// Number of sample points for optimization
    #[arg(long, value_name = "COUNT")]
    pub grain_sample_count: Option<usize>, // Made public
    /// Comma-separated initial grain values to test (e.g., 0,8,20)
    #[arg(long, value_delimiter = ',', value_name = "VALS")]
    pub grain_initial_values: Option<Vec<u8>>, // Made public
    /// Fallback grain value if optimization fails/disabled (default: 0)
    #[arg(long, value_name = "VALUE")]
    pub grain_fallback_value: Option<u8>, // Made public
}


#[cfg(test)]
mod tests {
    use super::*; // Import items from parent module (cli.rs)
    use std::path::PathBuf;

    // Helper function to create temporary directories/files for tests
    // Note: Real file system interaction is often avoided in pure unit tests,
    // but clap parsing tests often need valid paths.
    // For simplicity here, we'll assume paths exist or use relative ones.

    #[test]
    fn test_parse_encode_basic_args() {
        let args = vec![
            "drapto-cli", // Program name
            "encode",     // Subcommand
            "input_dir",  // input_path
            "output_dir", // output_dir
        ];
        let cli = Cli::parse_from(args);

        match cli.command {
            Commands::Encode(encode_args) => {
                assert_eq!(encode_args.input_path, PathBuf::from("input_dir"));
                assert_eq!(encode_args.output_dir, PathBuf::from("output_dir"));
                assert!(encode_args.log_dir.is_none());
                // Check new quality args are None by default
                assert!(encode_args.quality_sd.is_none());
                assert!(encode_args.quality_hd.is_none());
                assert!(encode_args.quality_uhd.is_none());
                // Check grain args
                assert!(!encode_args.disable_grain_optimization); // Default is false (optimization enabled)
                assert!(encode_args.grain_sample_duration.is_none());
                assert!(encode_args.grain_sample_count.is_none());
                assert!(encode_args.grain_initial_values.is_none());
                assert!(encode_args.grain_fallback_value.is_none());
            }
        }
    }

     #[test]
    fn test_parse_encode_with_log_dir() {
        let args = vec![
            "drapto-cli",
            "encode",
            "input.mkv", // Can be a file too
            "out",
            "--log-dir",
            "custom_logs",
        ];
        let cli = Cli::parse_from(args);

        match cli.command {
            Commands::Encode(encode_args) => {
                assert_eq!(encode_args.input_path, PathBuf::from("input.mkv"));
                assert_eq!(encode_args.output_dir, PathBuf::from("out"));
                assert_eq!(encode_args.log_dir, Some(PathBuf::from("custom_logs")));
                // Check quality args are still None
                assert!(encode_args.quality_sd.is_none());
                assert!(encode_args.quality_hd.is_none());
                assert!(encode_args.quality_uhd.is_none());
            }
        }
    }

     #[test]
    fn test_parse_encode_with_grain_args() {
        let args = vec![
            "drapto-cli",
            "encode",
            "input",
            "output",
            "--disable-grain-optimization",
            "--grain-sample-duration", "15",
            "--grain-sample-count", "5",
            "--grain-initial-values", "4,12,24",
            "--grain-fallback-value", "6",
        ];
        let cli = Cli::parse_from(args);

        match cli.command {
            Commands::Encode(encode_args) => {
                assert!(encode_args.disable_grain_optimization);
                assert_eq!(encode_args.grain_sample_duration, Some(15));
                assert_eq!(encode_args.grain_sample_count, Some(5));
                assert_eq!(encode_args.grain_initial_values, Some(vec![4, 12, 24]));
                assert_eq!(encode_args.grain_fallback_value, Some(6));
                // Check quality args are still None
                assert!(encode_args.quality_sd.is_none());
                assert!(encode_args.quality_hd.is_none());
                assert!(encode_args.quality_uhd.is_none());
            }
        }
    }
    #[test]
    fn test_parse_encode_with_quality_args() {
        let args = vec![
            "drapto-cli",
            "encode",
            "input",
            "output",
            "--quality-sd", "30",
            "--quality-hd", "25",
            "--quality-uhd", "22",
        ];
        let cli = Cli::parse_from(args);

        match cli.command {
            Commands::Encode(encode_args) => {
                assert_eq!(encode_args.quality_sd, Some(30));
                assert_eq!(encode_args.quality_hd, Some(25));
                assert_eq!(encode_args.quality_uhd, Some(22));
            }
        }
    }
    // Add more tests here for edge cases, invalid inputs (if clap allows testing this easily),
    // or specific flag combinations.
}