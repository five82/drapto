// drapto-cli/src/main.rs
//
// This file defines the command-line interface (CLI) for the Drapto video encoding tool.
// It uses the `clap` crate to parse command-line arguments for various operations,
// primarily the 'encode' command.
//
// Responsibilities include:
// - Defining CLI argument structures (`Cli`, `Commands`, `EncodeArgs`).
// - Parsing user-provided arguments.
// - Setting up logging to both console and file.
// - Validating input paths and identifying files to process.
// - Configuring the `drapto-core` library based on CLI arguments and defaults.
// - Invoking the core video processing logic (`drapto_core::process_videos`).
// - Handling results and errors from the core library.
// - Displaying a summary of encoding results.
// - Managing process exit codes based on success or failure.

use clap::{Parser, Subcommand};
use drapto_core::{CoreConfig, CoreError, EncodeResult};
use std::cell::Cell;
use std::fs::{self, File};
use std::io::{Write, BufWriter}; // Removed unused 'self' import
use std::path::PathBuf;
use std::process;
use std::time::Instant;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor}; // Added
mod config; // Import the new config module

// --- CLI Argument Definition ---

#[derive(Parser, Debug)]
#[command(
    author,
    version, // Reads from Cargo.toml via "cargo" feature in clap
    about = "Drapto: Video encoding tool",
    long_about = "Handles video encoding tasks using HandBrakeCLI via drapto-core library."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands, // Enum holds the specific subcommand
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Encodes video files from an input directory to an output directory
    Encode(EncodeArgs),
    // Add other subcommands here later (e.g., analyze, config)
}

#[derive(Parser, Debug)] // Use Parser derive for args struct as well
struct EncodeArgs {
    /// Input file or directory containing .mkv files
    #[arg(required = true, value_name = "INPUT_PATH")]
    input_path: PathBuf,

    /// Directory where encoded files will be saved
    #[arg(required = true, value_name = "OUTPUT_DIR")]
    output_dir: PathBuf,

    /// Optional: Directory for log files (defaults to OUTPUT_DIR/logs)
    #[arg(short, long, value_name = "LOG_DIR")]
    log_dir: Option<PathBuf>,

    // --- Film Grain Optimization Flags ---
    /// Disable automatic film grain optimization (it's enabled by default)
    #[arg(long)]
    disable_grain_optimization: bool,
    /// Duration (seconds) for each optimization sample clip
    #[arg(long, value_name = "SECONDS")]
    grain_sample_duration: Option<u32>,
    /// Number of sample points for optimization
    #[arg(long, value_name = "COUNT")]
    grain_sample_count: Option<usize>,
    /// Comma-separated initial grain values to test (e.g., 0,8,20)
    #[arg(long, value_delimiter = ',', value_name = "VALS")]
    grain_initial_values: Option<Vec<u8>>,
    /// Fallback grain value if optimization fails/disabled (default: 0)
    #[arg(long, value_name = "VALUE")]
    grain_fallback_value: Option<u8>,
}

// --- Helper Functions (Timestamp) ---
fn get_timestamp() -> String {
    chrono::Local::now().format("%Y%m%d_%H%M%S").to_string()
}



// --- Main Logic ---

// Renamed the main logic function to reflect the 'encode' action
fn run_encode(args: EncodeArgs) -> Result<(), Box<dyn std::error::Error>> {
    let total_start_time = Instant::now();
    let mut stdout = StandardStream::stdout(ColorChoice::Auto); // Added for colored output

    // Defaults are now hardcoded in config.rs

    // --- Determine Paths (using args from EncodeArgs) ---
    let input_path = args.input_path.canonicalize()
        .map_err(|e| format!("Invalid input path '{}': {}", args.input_path.display(), e))?;
    let output_dir = args.output_dir;
    let log_dir = args.log_dir.unwrap_or_else(|| output_dir.join("logs"));

    // --- Validate Input and Determine Files to Process ---
    let metadata = fs::metadata(&input_path)
        .map_err(|e| format!("Failed to access input path '{}': {}", input_path.display(), e))?;

    let (files_to_process, effective_input_dir) = if metadata.is_dir() {
        // Input is a directory
        // Use a match to handle NoFilesFound specifically, allowing processing to continue gracefully
        match drapto_core::find_processable_files(&input_path) {
             Ok(files) => (files, input_path.clone()), // Use the directory itself as the effective input dir
             Err(CoreError::NoFilesFound) => (Vec::new(), input_path.clone()), // No files found is okay, proceed with empty list
             Err(e) => return Err(e.into()), // Other errors are fatal
        }
    } else if metadata.is_file() {
        // Input is a file
        if input_path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("mkv")) {
            let parent_dir = input_path.parent().ok_or_else(|| {
                CoreError::PathError(format!("Could not determine parent directory for file '{}'", input_path.display()))
            })?.to_path_buf();
            (vec![input_path.clone()], parent_dir) // List contains only the input file, effective dir is parent
        } else {
            return Err(format!("Input file '{}' is not a .mkv file.", input_path.display()).into());
        }
    } else {
        return Err(format!("Input path '{}' is neither a file nor a directory.", input_path.display()).into());
    };

    // --- Create Output/Log Dirs ---
    fs::create_dir_all(&output_dir)?;
    fs::create_dir_all(&log_dir)?;

    // --- Setup Logging ---
    let main_log_filename = format!("drapto_encode_run_{}.log", get_timestamp()); // Log name reflects action
    let main_log_path = log_dir.join(main_log_filename);
    let log_file = File::create(&main_log_path)?;
    let mut logger = Box::new(BufWriter::new(log_file)); // Using Box for simplicity with closure

    // Use Cell to allow modifying state within FnMut closure
    let last_was_progress = Cell::new(false);

    // --- Log Callback (Console + File) ---
    // Captures stdout mutably for coloring console output
    let mut log_callback = |msg: &str| {
        // --- File Logging (Always raw) ---
        // Write the raw message to the log file first
        writeln!(logger, "{}", msg).ok();
        logger.flush().ok(); // Flush file buffer

        // --- Console Logging (Colored) ---
        let is_progress = msg.contains('\r');
        let msg_trimmed = msg.trim_end(); // Use trimmed for console logic

        if is_progress {
            // For progress, write directly, assuming HandBrake handles terminal control
            // Style HandBrakeCLI progress lines as Blue
            // Explicitly set Blue foreground, and ensure not bold/dimmed
            stdout.set_color(ColorSpec::new()
                .set_fg(Some(Color::Blue))
                .set_bold(false)
                .set_dimmed(false)
                .set_intense(false) // Also ensure not intense
            ).ok();
            write!(&mut stdout, "{}", msg).ok(); // Print original message
            stdout.reset().ok(); // Reset color immediately after writing
            stdout.flush().ok(); // Flush console buffer
            last_was_progress.set(true);
        } else {
            // For normal messages, handle potential preceding progress line
            if last_was_progress.get() {
                writeln!(&mut stdout).ok(); // Move to the next line after progress
            }

            // --- Apply Enhanced Styling ---
            // We need more granular control than a single color_spec for the whole line now.
            // Handle specific cases first.

            let mut handled = false; // Flag to check if we printed already

            // Define prefix arrays for styling rules
            let bold_label_prefixes = [
                "Input path:", "Output directory:", "Log directory:", "Main log file:",
                "Total encode execution time:", "Drapto Encode Run Finished:", "Drapto Encode Run Started:"
            ];
            let summary_value_prefixes = [ // Moved definition here
                "  Encode time: ", "  Input size:  ", "  Output size: ", "  Reduced by:  " // Note spaces for alignment
            ];

            // Style: Bold Labels, Normal Values (Initial Info & Final Timing)
            for prefix in bold_label_prefixes {
                if msg_trimmed.starts_with(prefix) {
                    if let Some(value) = msg_trimmed.strip_prefix(prefix) {
                        stdout.set_color(ColorSpec::new().set_bold(true)).ok(); // Bold label
                        write!(&mut stdout, "{}", prefix).ok();
                        stdout.reset().ok(); // Reset for value
                        writeln!(&mut stdout, "{}", value).ok();
                        handled = true;
                        break;
                    }
                }
            }

            // Style: Normal Labels, Bold Values (Summary Details)
            if !handled {
                // summary_value_prefixes is now defined above
                 for prefix in summary_value_prefixes {
                     if msg_trimmed.starts_with(prefix) {
                         if let Some(value) = msg_trimmed.strip_prefix(prefix) {
                             write!(&mut stdout, "{}", prefix).ok(); // Normal label
                             stdout.set_color(ColorSpec::new().set_bold(true)).ok(); // Bold value
                             writeln!(&mut stdout, "{}", value).ok();
                             stdout.reset().ok(); // Reset after value
                             handled = true;
                             break;
                         }
                     }
                 }
            }

            // Style: Success Count (Bold Green Number)
            if !handled && msg_trimmed.starts_with("Successfully encoded ") {
                 if let Some(rest) = msg_trimmed.strip_prefix("Successfully encoded ") {
                     if let Some((count_str, _suffix)) = rest.split_once(" file(s).") { // Prefix suffix with _
                         write!(&mut stdout, "Successfully encoded ").ok();
                         stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true)).ok();
                         write!(&mut stdout, "{}", count_str).ok();
                         stdout.reset().ok();
                         stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green))).ok(); // Green for suffix
                         writeln!(&mut stdout, " file(s).").ok();
                         stdout.reset().ok();
                         handled = true;
                     }
                 }
            }
// Style: Status Prefixes ([OK], [INFO], etc.)
if !handled {
    let status_prefixes = [
        ("[OK]", ColorSpec::new().set_fg(Some(Color::Green)).clone()),
        ("[INFO]", ColorSpec::new().set_fg(Some(Color::Cyan)).clone()),
        ("[WARN]", ColorSpec::new().set_fg(Some(Color::Yellow)).clone()),
        ("[ERROR]", ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true).clone()),
        ("[FAIL]", ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true).clone()),
        ("[DEBUG]", ColorSpec::new().set_fg(Some(Color::Magenta)).clone()),
        ("[TRACE]", ColorSpec::new().set_fg(Some(Color::Blue)).clone()),
    ];
    for (prefix, spec) in status_prefixes {
         // Add space to prefix for matching to avoid partial matches like "[INFO]rmation"
         let prefix_with_space = format!("{} ", prefix);
         if msg_trimmed.starts_with(&prefix_with_space) {
             if let Some(rest) = msg_trimmed.strip_prefix(&prefix_with_space) {
                 stdout.set_color(&spec).ok();
                 write!(&mut stdout, "{}", prefix).ok(); // Write only the prefix colored
                 stdout.reset().ok();
                 writeln!(&mut stdout, " {}", rest).ok(); // Write the rest uncolored (with space)
                 handled = true;
                 break;
             }
         }
         // Handle cases where the prefix might be the entire message (less likely but possible)
         else if msg_trimmed == prefix {
              stdout.set_color(&spec).ok();
              writeln!(&mut stdout, "{}", prefix).ok();
              stdout.reset().ok();
              handled = true;
              break;
         }
    }
}

// Style: Specific Lines ("Processing:", "External dependency check passed.")
if !handled {
    if msg_trimmed == "External dependency check passed." {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green))).ok();
        writeln!(&mut stdout, "{}", msg_trimmed).ok();
        stdout.reset().ok();
        handled = true;
    } else if let Some(filename) = msg_trimmed.strip_prefix("Processing: ") {
        stdout.set_color(ColorSpec::new().set_bold(true)).ok(); // Bold "Processing:"
        write!(&mut stdout, "Processing: ").ok();
        stdout.reset().ok(); // Reset for filename
        writeln!(&mut stdout, "{}", filename).ok();
        handled = true;
    }
}


// --- Fallback to Previous Simpler Styling for remaining unhandled cases ---
if !handled {
    let mut color_spec = ColorSpec::new();
    match msg_trimmed {
        // Separators
        m if m.starts_with("===") || m.starts_with("---") => {
            color_spec.set_fg(Some(Color::Cyan)).set_bold(true);
        }
        // Headers
        m if m.starts_with("Encoding Summary:") => {
            color_spec.set_bold(true); // Bold White
        }
        // Warnings (already handled FATAL CORE ERROR, Success, specific labels/values)
        m if m.starts_with("No processable .mkv files") || m.starts_with("No files were successfully encoded.") => {
            color_spec.set_fg(Some(Color::Yellow));
        }
        // Default case: Check for summary filename or just print default
        _ => {
            // Heuristic for summary filename: Not indented, not handled above
             if !msg_trimmed.starts_with(' ') &&
               !msg_trimmed.starts_with("===") && !msg_trimmed.starts_with("---") &&
               !msg_trimmed.starts_with("Encoding Summary:")
               // Add other known non-filename prefixes if needed
            {
                // Assume it's a filename in the summary
                color_spec.set_bold(true); // Bold White
            }
            // Otherwise, use default color spec (covers "Found X files...", etc.)
        }
    }
    // Print lines handled by this fallback logic
    stdout.set_color(&color_spec).ok();
    writeln!(&mut stdout, "{}", msg_trimmed).ok();
    stdout.reset().ok(); // Reset to default colors
}
            // Removed redundant reset and extra brace from previous fallback logic

            // Note: Printing is now handled within the specific styling blocks above or in the fallback match.
            // The reset is also handled within those blocks.

            last_was_progress.set(false);
            stdout.flush().ok(); // Flush console buffer
        }
    };

    // --- Log Initial Info ---
    log_callback("========================================");
    log_callback(&format!("Drapto Encode Run Started: {}", chrono::Local::now()));
    log_callback(&format!("Input path: {}", input_path.display())); // Log original input path
    log_callback(&format!("Output directory: {}", output_dir.display()));
    log_callback(&format!("Log directory: {}", log_dir.display()));
    log_callback(&format!("Main log file: {}", main_log_path.display()));
    log_callback("========================================");

    // --- Prepare Core Configuration (including defaults) ---
    let config = CoreConfig {
        input_dir: effective_input_dir,
        output_dir: output_dir.clone(),
        log_dir: log_dir.clone(),
        // Use the constants from the config module
        // Convert types as needed (i32 -> u8, &str -> String)
        default_encoder_preset: Some(config::DEFAULT_ENCODER_PRESET as u8),
        default_quality: Some(config::DEFAULT_QUALITY as u8),
        default_crop_mode: Some(config::DEFAULT_CROP_MODE.to_string()),
        // New film grain config fields - set to None, core will use defaults
        film_grain_metric_type: None,
        film_grain_knee_threshold: None,
        film_grain_refinement_range_delta: None,
        film_grain_max_value: None,
        film_grain_refinement_points_count: None,
        // --- Film Grain Args ---
        optimize_film_grain: !args.disable_grain_optimization, // Enabled by default, disable with flag
        film_grain_sample_duration: args.grain_sample_duration,
        film_grain_sample_count: args.grain_sample_count,
        film_grain_initial_values: args.grain_initial_values,
        film_grain_fallback_value: args.grain_fallback_value,
    };

    // --- Execute Core Logic ---
    let processing_result: Result<Vec<EncodeResult>, CoreError>;
    // --- Execute Core Logic ---
    // We already determined files_to_process above
    log_callback(&format!("Found {} file(s) to process.", files_to_process.len()));
    if files_to_process.is_empty() {
         // This case is hit if input was a dir with no .mkv files, or an empty dir.
         log_callback("No processable .mkv files found in the specified input path.");
         processing_result = Ok(Vec::new()); // Indicate success (nothing to do), but empty results
    } else {
         processing_result = drapto_core::process_videos(&config, &files_to_process, &mut log_callback);
    }

    // --- Handle Core Results ---
    let successfully_encoded: Vec<EncodeResult>;
    match processing_result {
        Ok(ref results) => {
            successfully_encoded = results.to_vec();
            if successfully_encoded.is_empty() && !matches!(processing_result, Err(CoreError::NoFilesFound)) { // Don't log if no files found was the reason
                 log_callback("No files were successfully encoded.");
            } else if !successfully_encoded.is_empty() {
                 log_callback(&format!("Successfully encoded {} file(s).", successfully_encoded.len()));
            }
            // If NoFilesFound, we already logged it and successfully_encoded is empty.
        }
        Err(e) => {
            log_callback(&format!("FATAL CORE ERROR during processing: {}", e));
            logger.flush()?;
            return Err(e.into());
        }
    }

    // --- Print Summary ---
    if !successfully_encoded.is_empty() {
        log_callback("========================================");
        log_callback("Encoding Summary:");
        log_callback("========================================");
        for result in &successfully_encoded {
            let reduction = if result.input_size > 0 {
                100u64.saturating_sub(result.output_size.saturating_mul(100) / result.input_size)
            } else {
                0
            };
            log_callback(&format!("{}", result.filename));
            log_callback(&format!("  Encode time: {}", drapto_core::format_duration(result.duration)));
            log_callback(&format!("  Input size:  {}", drapto_core::format_bytes(result.input_size)));
            log_callback(&format!("  Output size: {}", drapto_core::format_bytes(result.output_size)));
            log_callback(&format!("  Reduced by:  {}%", reduction));
            log_callback("----------------------------------------");
        }
    }

    // --- Final Timing ---
    let total_elapsed_time = total_start_time.elapsed();
    log_callback("========================================");
    log_callback(&format!("Total encode execution time: {}", drapto_core::format_duration(total_elapsed_time)));
    log_callback(&format!("Drapto Encode Run Finished: {}", chrono::Local::now()));
    log_callback("========================================");

    logger.flush()?;

    Ok(())
}


// Update main to return Result for potential IO errors during colored stderr printing
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse the top-level arguments
    let cli = Cli::parse();

    // Match on the command provided
    let result = match cli.command {
        Commands::Encode(args) => {
            // Call the specific function for the encode command
            run_encode(args)
        } // Add other command arms here -> { run_other_command(args) }
    };

    // Handle the result from the command function
    match result {
        Ok(()) => {
            // process::exit(0); // Exit is handled after main returns Ok
        }
        Err(e) => {
            // Use termcolor for stderr as well
            let mut stderr = StandardStream::stderr(ColorChoice::Auto);
            stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
            writeln!(&mut stderr, "Error: {}", e)?;
            stderr.reset()?;
            process::exit(1); // Exit with error code
        }
    };

    // If result was Ok, return Ok here to satisfy the function signature.
    // The process will exit with 0 implicitly when main returns Ok.
    if result.is_ok() {
        Ok(())
    } else {
        // Errors should have been handled and exited within the match,
        // but satisfy the compiler if somehow an Err propagates here.
        // This branch shouldn't realistically be hit due to process::exit(1) above.
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
                assert!(!encode_args.disable_grain_optimization); // Default is false (optimization enabled)
                assert!(encode_args.grain_sample_duration.is_none());
                assert!(encode_args.grain_sample_count.is_none());
                assert!(encode_args.grain_initial_values.is_none());
                assert!(encode_args.grain_fallback_value.is_none());
            } // _ => panic!("Expected Encode command"), // Removed as other commands don't exist yet
        }
    }

    #[test]
    fn test_parse_encode_with_log_dir() {
        let args = vec![
            "drapto-cli",
            "encode",
            "input.mkv",
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
            }
        }
    }

    // Add more tests here for edge cases, invalid inputs (if clap allows testing this easily),
    // or specific flag combinations.
}