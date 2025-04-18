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
pub struct Cli { // Made public
    #[command(subcommand)]
    pub command: Commands, // Made public

    /// Run in interactive mode (foreground) instead of daemonizing.
    #[arg(long, global = true, default_value_t = false)]
    pub interactive: bool,
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
    #[arg(long, value_name = "PRESET_INT")]
    pub preset: Option<u8>,

    /// Disable automatic crop detection (uses ffmpeg's cropdetect)
    #[arg(long)]
    pub disable_autocrop: bool,

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
        // Temporarily unset env var to test CLI parsing in isolation
        let original_env = std::env::var("DRAPTO_NTFY_TOPIC").ok();
        unsafe { std::env::remove_var("DRAPTO_NTFY_TOPIC"); }

        let args = vec![
            "drapto-cli", // Program name
            "encode",     // Subcommand
            "--input", "input_dir",  // input_path using long flag
            "--output", "output_dir", // output_dir using long flag
        ];
        let cli = Cli::parse_from(args);

        assert!(!cli.interactive); // Check default interactive flag is false

        match cli.command {
            Commands::Encode(encode_args) => {
                assert_eq!(encode_args.input_path, PathBuf::from("input_dir"));
                assert_eq!(encode_args.output_dir, PathBuf::from("output_dir"));
                assert!(encode_args.log_dir.is_none());
                // Check new quality args are None by default
                assert!(encode_args.quality_sd.is_none());
                assert!(encode_args.quality_hd.is_none());
                assert!(encode_args.quality_uhd.is_none());
                assert!(encode_args.ntfy.is_none()); // Check new ntfy arg
                assert!(encode_args.preset.is_none()); // Check new preset arg (u8)
                assert!(!encode_args.disable_autocrop); // Check default
            },
            // Add other command checks if necessary
        }

        // Restore env var
        if let Some(val) = original_env {
            unsafe { std::env::set_var("DRAPTO_NTFY_TOPIC", val); }
        }
    }

     #[test]
    fn test_parse_encode_with_log_dir() {
        // Temporarily unset env var
        let original_env = std::env::var("DRAPTO_NTFY_TOPIC").ok();
        unsafe { std::env::remove_var("DRAPTO_NTFY_TOPIC"); }

        let args = vec![
            "drapto-cli",
            "encode",
            "-i", "input.mkv", // Use short flag
            "-o", "out",       // Use short flag
            "--log-dir",
            "custom_logs",
        ];
        let cli = Cli::parse_from(args);

        assert!(!cli.interactive); // Check default interactive flag is false

        match cli.command {
            Commands::Encode(encode_args) => {
                assert_eq!(encode_args.input_path, PathBuf::from("input.mkv"));
                assert_eq!(encode_args.output_dir, PathBuf::from("out"));
                assert_eq!(encode_args.log_dir, Some(PathBuf::from("custom_logs")));
                // Check quality args are still None
                assert!(encode_args.quality_sd.is_none());
                assert!(encode_args.quality_hd.is_none());
                assert!(encode_args.quality_uhd.is_none());
                assert!(encode_args.ntfy.is_none()); // Check new ntfy arg
                assert!(encode_args.preset.is_none()); // Check new preset arg (u8)
                assert!(!encode_args.disable_autocrop); // Check default
            },
            // Add other command checks if necessary
        }

        // Restore env var
        if let Some(val) = original_env {
            unsafe { std::env::set_var("DRAPTO_NTFY_TOPIC", val); }
        }
    }

    // Test for removed grain args is deleted.
    #[test]
    fn test_parse_encode_with_quality_args() {
        // Temporarily unset env var
        let original_env = std::env::var("DRAPTO_NTFY_TOPIC").ok();
        unsafe { std::env::remove_var("DRAPTO_NTFY_TOPIC"); }

        let args = vec![
            "drapto-cli",
            "encode",
            "-i", "input",
            "-o", "output",
            "--quality-sd", "30",
            "--quality-hd", "25",
            "--quality-uhd", "22",
        ];
        let cli = Cli::parse_from(args);

        assert!(!cli.interactive); // Check default interactive flag is false

        match cli.command {
            Commands::Encode(encode_args) => {
                assert_eq!(encode_args.quality_sd, Some(30));
                assert_eq!(encode_args.quality_hd, Some(25));
                assert_eq!(encode_args.quality_uhd, Some(22));
                assert!(encode_args.ntfy.is_none()); // Check new ntfy arg
                assert!(encode_args.preset.is_none()); // Check new preset arg (u8)
                assert!(!encode_args.disable_autocrop); // Check default
            },
            // Add other command checks if necessary
        }

        // Restore env var
        if let Some(val) = original_env {
            unsafe { std::env::set_var("DRAPTO_NTFY_TOPIC", val); }
        }
    }
    // Add more tests here for edge cases, invalid inputs (if clap allows testing this easily),
    // or specific flag combinations.
    #[test]
    fn test_parse_encode_with_ntfy_arg() {
        let args = vec![
            "drapto-cli",
            "encode",
            "--input", "input",
            "--output", "output",
            "--ntfy", "https://ntfy.sh/mytopic",
        ];
        let cli = Cli::parse_from(args);

        assert!(!cli.interactive); // Check default interactive flag is false

        match cli.command {
            Commands::Encode(encode_args) => {
                assert_eq!(encode_args.ntfy, Some("https://ntfy.sh/mytopic".to_string()));
                // Check other args are default/None
                assert!(encode_args.log_dir.is_none());
                assert!(encode_args.quality_sd.is_none());
                assert!(encode_args.preset.is_none()); // Check new preset arg (u8)
                assert!(!encode_args.disable_autocrop); // Check default
            },
            // Add other command checks if necessary
        }
    }

    // Test with environment variable (requires setting it before running the test,
    // which is tricky in standard `cargo test`. Could use a helper crate or manual setup)
    // #[test]
    // fn test_parse_encode_with_ntfy_env() {
    //     std::env::set_var("DRAPTO_NTFY_TOPIC", "https://env.ntfy.sh/topic");
    //     let args = vec![
    //         "drapto-cli",
    //         "encode",
    //         "input",
    //         "output",
    //     ];
    //     let cli = Cli::parse_from(args);
    //     std::env::remove_var("DRAPTO_NTFY_TOPIC"); // Clean up env var

    //     match cli.command {
    //         Commands::Encode(encode_args) => {
    //             assert_eq!(encode_args.ntfy, Some("https://env.ntfy.sh/topic".to_string()));
    //         }
    //     }
    // }

    #[test]
    fn test_parse_encode_interactive_flag() {
        let args = vec![
            "drapto-cli",
            "--interactive", // Add the global flag
            "encode",
            "-i", "input",
            "-o", "output",
        ];
        let cli = Cli::parse_from(args);

        assert!(cli.interactive); // Check interactive flag is true

        match cli.command {
            Commands::Encode(encode_args) => {
                assert_eq!(encode_args.input_path, PathBuf::from("input"));
                assert_eq!(encode_args.output_dir, PathBuf::from("output"));
            },
            // Add other command checks if necessary
        }
    }
    #[test]
    fn test_parse_encode_disable_autocrop_flag() {
        let args = vec![
            "drapto-cli",
            "encode",
            "-i", "input",
            "-o", "output",
            "--disable-autocrop", // Add the flag
        ];
        let cli = Cli::parse_from(args);

        assert!(!cli.interactive); // Check default interactive flag is false

        match cli.command { // Add braces for clarity
            Commands::Encode(encode_args) => {
                assert!(encode_args.disable_autocrop); // Check flag is true
                // Check other args are default/None
                assert!(encode_args.log_dir.is_none());
                assert!(encode_args.quality_sd.is_none());
                assert!(encode_args.ntfy.is_none());
                assert!(encode_args.preset.is_none()); // Check new preset arg (u8)
            },
            // Add other command checks if necessary
        }
    }
}

    #[test]
    fn test_parse_encode_with_preset_arg() {
        let args = vec![
            "drapto-cli",
            "encode",
            "--input", "input", // Use long flags for clarity
            "--output", "output",
            "--preset", "4", // Use a numeric value
        ];
        let cli = Cli::parse_from(args);

        assert!(!cli.interactive); // Check default interactive flag is false

        match cli.command {
            Commands::Encode(encode_args) => {
                assert_eq!(encode_args.preset, Some(4)); // Check numeric value
                // Check other args are default/None
                assert!(encode_args.log_dir.is_none());
                assert!(encode_args.quality_sd.is_none());
                assert!(!encode_args.disable_autocrop);
                assert!(encode_args.ntfy.is_none());
            },
            // Add other command checks if necessary
        }
    }