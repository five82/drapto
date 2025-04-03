use clap::{Parser, Subcommand};
use drapto_core::{CoreConfig, CoreError, EncodeResult};
use serde::Deserialize; // Added for TOML parsing
use std::cell::Cell;
use std::fs::{self, File};
use std::io::{Write, BufWriter};
use std::path::PathBuf;
use std::process;
use std::time::Instant;
use toml; // Added for TOML parsing

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
}

// --- Helper Functions (Timestamp) ---
fn get_timestamp() -> String {
    chrono::Local::now().format("%Y%m%d_%H%M%S").to_string()
}

// --- Configuration Struct ---

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)] // Optional: Error if unknown fields are in TOML
struct HandbrakeDefaults {
    encoder_preset: Option<u8>,
    quality: Option<u8>,
    crop_mode: Option<String>,
}

// Provide default values (None means not set, core library should handle its own defaults)
impl Default for HandbrakeDefaults {
    fn default() -> Self {
        HandbrakeDefaults {
            encoder_preset: None,
            quality: None,
            crop_mode: None,
        }
    }
}


// --- Main Logic ---

// Renamed the main logic function to reflect the 'encode' action
fn run_encode(args: EncodeArgs) -> Result<(), Box<dyn std::error::Error>> {
    let total_start_time = Instant::now();

    // --- Load Handbrake Defaults ---
    // Assumes handbrake_defaults.toml is in the same directory as Cargo.toml for the CLI
    // A more robust solution might involve searching standard config locations.
    let defaults_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("handbrake_defaults.toml");
    let defaults: HandbrakeDefaults = match fs::read_to_string(&defaults_path) {
        Ok(content) => match toml::from_str(&content) {
            Ok(parsed_defaults) => {
                // Use println! for now, replace with proper logging later if needed
                println!("INFO: Loaded defaults from {}", defaults_path.display());
                parsed_defaults
            }
            Err(e) => {
                eprintln!(
                    "WARN: Failed to parse {}: {}. Using built-in defaults.",
                    defaults_path.display(),
                    e
                );
                HandbrakeDefaults::default()
            }
        },
        Err(e) => {
            // Only warn if it's not a 'NotFound' error, otherwise silently use defaults
            if e.kind() != std::io::ErrorKind::NotFound {
                 eprintln!(
                    "WARN: Could not read {}: {}. Using built-in defaults.",
                    defaults_path.display(),
                    e
                 );
            } else {
                 println!("INFO: Defaults file {} not found. Using built-in defaults.", defaults_path.display());
            }
            HandbrakeDefaults::default()
        }
    };
    // Temporary: Print loaded defaults for verification
    println!("DEBUG: Using Handbrake defaults: {:?}", defaults);

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

    let mut log_callback = |msg: &str| {
        let is_progress = msg.contains('\r');

        if is_progress {
            // Print progress update using print! and flush
            print!("{}", msg);
            std::io::stdout().flush().ok();
            last_was_progress.set(true);
        } else {
            // If the last message was progress, print a newline first
            // to move off the progress line before printing the current message.
            if last_was_progress.get() {
                println!(); // Move to the next line
            }
            // Print the normal log message
            println!("{}", msg);
            last_was_progress.set(false);
        }

        // Write the raw message to the log file regardless
        writeln!(logger, "{}", msg).ok();
        // Flush the file logger
        logger.flush().ok();
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
        // Pass the loaded defaults (or None if not set in TOML/file not found)
        default_encoder_preset: defaults.encoder_preset,
        default_quality: defaults.quality,
        default_crop_mode: defaults.crop_mode,
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


fn main() {
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
            process::exit(0);
        }
        Err(e) => {
            // Print errors directly to stderr for CLI tools
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}