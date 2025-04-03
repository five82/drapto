use std::fs;
use std::io::{self}; // Removed BufRead, BufReader
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use std::collections::VecDeque;
use thiserror::Error; // Import the macro

// --- Public Structs ---

#[derive(Debug, Clone)] // Clone might be useful for the CLI
pub struct EncodeResult {
    pub filename: String,
    pub duration: Duration,
    pub input_size: u64,
    pub output_size: u64,
}

#[derive(Debug, Clone)] // Configuration for the core processing
pub struct CoreConfig {
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    pub log_dir: PathBuf,
    // --- Optional Handbrake Defaults ---
    pub default_encoder_preset: Option<u8>,
    pub default_quality: Option<u8>,
    pub default_crop_mode: Option<String>,
}

// --- Custom Error Type ---

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error), // Auto-implements From<io::Error>

    #[error("Directory traversal error: {0}")]
    Walkdir(#[from] walkdir::Error),

    #[error("Path error: {0}")]
    PathError(String),

    #[error("Failed to execute {0}: {1}")]
    CommandStart(String, io::Error), // e.g., "ffprobe", source error

    #[error("Failed to wait for {0}: {1}")]
    CommandWait(String, io::Error), // e.g., "HandBrakeCLI", source error

    #[error("Command {0} failed with status {1}. Stderr: {2}")]
    CommandFailed(String, std::process::ExitStatus, String), // e.g., "ffprobe", status, stderr

    #[error("ffprobe output parsing error: {0}")]
    FfprobeParse(String),

    #[error("No suitable video files found in input directory")]
    NoFilesFound,
}

// Type alias for Result using our custom error
pub type CoreResult<T> = Result<T, CoreError>;

// --- Helper Functions (Internal - not pub) ---

fn get_file_size(path: &Path) -> CoreResult<u64> {
    Ok(fs::metadata(path)?.len())
}

// Gets audio channel counts using ffprobe
fn get_audio_channels(input_path: &Path) -> CoreResult<Vec<u32>> {
    let cmd_name = "ffprobe";
    let output = Command::new(cmd_name)
        .args([
            "-v",
            "error",
            "-select_streams",
            "a",
            "-show_entries",
            "stream=channels",
            "-of",
            "csv=p=0",
        ])
        .arg(input_path)
        .output()
        .map_err(|e| CoreError::CommandStart(cmd_name.to_string(), e))?;

    if !output.status.success() {
        return Err(CoreError::CommandFailed(
            cmd_name.to_string(),
            output.status,
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .map(|line| {
            line.trim()
                .parse::<u32>()
                .map_err(|e| CoreError::FfprobeParse(format!("Failed to parse channel count '{}': {}", line, e)))
        })
        .collect()
}

// Calculates audio bitrate based on channel count
fn calculate_audio_bitrate(channels: u32) -> u32 {
    match channels {
        1 => 64,   // Mono
        2 => 128,  // Stereo
        6 => 256,  // 5.1
        8 => 384,  // 7.1
        _ => channels * 48, // Default fallback
    }
}

// --- Public API ---

/// Finds processable video files (currently hardcoded to .mkv).
pub fn find_processable_files(input_dir: &Path) -> CoreResult<Vec<PathBuf>> {
    // Use collect to handle potential WalkDir errors first
    let entries: Vec<walkdir::DirEntry> = walkdir::WalkDir::new(input_dir)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .collect::<Result<Vec<_>, _>>() // Collect results, propagating the first error
        .map_err(CoreError::Walkdir)?; // Map walkdir::Error to CoreError::Walkdir

    let files: Vec<PathBuf> = entries
        .into_iter()
        .filter(|e| e.file_type().is_file())
        .filter_map(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str()) // Ensure extension is valid UTF-8
                .filter(|ext_str| ext_str.eq_ignore_ascii_case("mkv"))
                .map(|_| entry.path().to_path_buf()) // If it's an mkv, keep the path
        })
        .collect();

    if files.is_empty() {
        // If entries were successfully collected but no MKV files were found
        Err(CoreError::NoFilesFound)
    } else {
        Ok(files)
    }
}

/// Processes a list of video files based on the configuration.
/// Calls the `log_callback` for logging messages.
/// Returns a list of results for successfully processed files.
pub fn process_videos<F>(
    config: &CoreConfig,
    files_to_process: &[PathBuf],
    mut log_callback: F,
) -> CoreResult<Vec<EncodeResult>>
where
    F: FnMut(&str), // Closure to handle logging
{
    let mut results: Vec<EncodeResult> = Vec::new();
    let cmd_handbrake = "HandBrakeCLI"; // Define command name

    for input_path in files_to_process {
        let file_start_time = Instant::now();
        let filename = input_path
            .file_name()
            .ok_or_else(|| CoreError::PathError(format!("Failed to get filename for {}", input_path.display())))?
            .to_string_lossy()
            .to_string();
        let _filename_noext = input_path
            .file_stem()
            .ok_or_else(|| CoreError::PathError(format!("Failed to get filename stem for {}", input_path.display())))?
            .to_string_lossy()
            .to_string();

        let output_path = config.output_dir.join(&filename);

        log_callback(&format!("Processing: {}", filename));

        // --- Get Audio Info ---
        let audio_channels = match get_audio_channels(input_path) {
            Ok(channels) => {
                log_callback(&format!("Detected audio channels: {:?}", channels));
                channels
            }
            Err(e) => {
                log_callback(&format!("Warning: Error getting audio channels for {}: {}. Skipping audio bitrate options.", filename, e));
                // Continue without specific bitrates if ffprobe fails for this file
                vec![]
            }
        };

        // --- Build HandBrakeCLI Command ---
        let mut handbrake_args: VecDeque<String> = VecDeque::new();

        // --- Build HandBrakeCLI Command using Config Defaults ---

        // Fixed options (Encoder, Tune, etc.)
        handbrake_args.push_back("--encoder".to_string());
        handbrake_args.push_back("svt_av1_10bit".to_string());
        handbrake_args.push_back("--encoder-tune".to_string());
        handbrake_args.push_back("0".to_string()); // Assuming tune 0 is always desired
        handbrake_args.push_back("--encopts".to_string());
        handbrake_args.push_back("film-grain=8:film-grain-denoise=1".to_string()); // Assuming film grain is always desired

        // Encoder Preset (Use default from config or fallback)
        let encoder_preset = config.default_encoder_preset.unwrap_or(6); // Fallback to 6
        handbrake_args.push_back("--encoder-preset".to_string());
        handbrake_args.push_back(encoder_preset.to_string());
        log_callback(&format!("Using encoder preset: {}", encoder_preset));

        // Quality (Use default from config or fallback)
        let quality = config.default_quality.unwrap_or(28); // Fallback to 28
        handbrake_args.push_back("--quality".to_string());
        handbrake_args.push_back(quality.to_string());
        log_callback(&format!("Using quality: {}", quality));

        // Crop Mode (Only add if specified in config)
        if let Some(crop_mode) = &config.default_crop_mode {
            handbrake_args.push_back("--crop-mode".to_string());
            handbrake_args.push_back(crop_mode.clone()); // Clone the string
             log_callback(&format!("Using crop mode: {}", crop_mode));
        } else {
             log_callback("Using Handbrake's default crop mode (likely 'off')");
        }


        // Other fixed options
         handbrake_args.push_back("--auto-anamorphic".to_string());
         handbrake_args.push_back("--all-subtitles".to_string());
         handbrake_args.push_back("--aencoder".to_string());
         handbrake_args.push_back("opus".to_string());
         handbrake_args.push_back("--all-audio".to_string());
         handbrake_args.push_back("--mixdown".to_string());
         handbrake_args.push_back("none".to_string());
         handbrake_args.push_back("--enable-hw-decoding".to_string());
         handbrake_args.push_back("--no-comb-detect".to_string());
         handbrake_args.push_back("--no-deinterlace".to_string());
         handbrake_args.push_back("--no-bwdif".to_string());
         handbrake_args.push_back("--no-decomb".to_string());
         handbrake_args.push_back("--no-detelecine".to_string());
         handbrake_args.push_back("--no-hqdn3d".to_string());
         handbrake_args.push_back("--no-nlmeans".to_string());
         handbrake_args.push_back("--no-chroma-smooth".to_string());
         handbrake_args.push_back("--no-unsharp".to_string());
         handbrake_args.push_back("--no-lapsharp".to_string());
         handbrake_args.push_back("--no-deblock".to_string());

         // Dynamic audio bitrate options
        let mut audio_bitrate_opts_log = String::new();
        for (index, &num_channels) in audio_channels.iter().enumerate() {
            let bitrate = calculate_audio_bitrate(num_channels);
            handbrake_args.push_back("--ab".to_string());
            handbrake_args.push_back(bitrate.to_string());
            let log_msg = format!(
                "Added bitrate for audio stream {} ({} channels): {}kbps",
                index, num_channels, bitrate
            );
            log_callback(&log_msg);
            audio_bitrate_opts_log.push_str(&format!(" --ab {}", bitrate));
        }
        if !audio_bitrate_opts_log.is_empty() {
            log_callback(&format!("Final audio bitrate options:{}", audio_bitrate_opts_log));
        }

         // Input and Output files
         handbrake_args.push_back("-i".to_string());
         handbrake_args.push_back(input_path.to_string_lossy().to_string());
         handbrake_args.push_back("-o".to_string());
         handbrake_args.push_back(output_path.to_string_lossy().to_string());


        log_callback(&format!("Starting HandBrakeCLI for {}...", filename));
        log_callback(&format!("Command: {} {}", cmd_handbrake, handbrake_args.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(" ")));


        // --- Execute HandBrakeCLI ---
        let mut child = Command::new(cmd_handbrake)
            .args(Vec::from(handbrake_args))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()) // Capture stderr
            .spawn()
            .map_err(|e| CoreError::CommandStart(cmd_handbrake.to_string(), e))?;

         // --- Stream Output to Log Callback ---
         // Combine stdout and stderr readers
         // Use raw readers, BufReader might add unwanted buffering here
         let mut stdout_reader = child.stdout.take().unwrap();
         let mut stderr_reader = child.stderr.take().unwrap();
         // Removed initial declaration of stderr_output

         // --- Process stdout/stderr concurrently (more robust) ---
         use std::io::Read;
         use std::thread;
         use std::sync::mpsc; // For sending data back from threads

         let (tx, rx) = mpsc::channel(); // Channel to send output lines/chunks

         let stdout_tx = tx.clone();
         let stdout_thread = thread::spawn(move || {
             let mut buffer = [0; 1024]; // Read in chunks
             loop {
                 match stdout_reader.read(&mut buffer) {
                     Ok(0) => break, // EOF
                     Ok(n) => {
                         // Send the chunk as lossy UTF-8
                         let chunk = String::from_utf8_lossy(&buffer[..n]);
                         if stdout_tx.send(chunk.to_string()).is_err() {
                             break; // Receiver disconnected
                         }
                     }
                     Err(_) => break, // Error reading
                 }
             }
         });

         let stderr_tx = tx; // No need to clone tx again
         let stderr_thread = thread::spawn(move || {
             let mut buffer = [0; 1024]; // Read in chunks
             let mut captured_stderr = String::new(); // Local stderr capture
             loop {
                 match stderr_reader.read(&mut buffer) {
                     Ok(0) => break, // EOF
                     Ok(n) => {
                         let chunk = String::from_utf8_lossy(&buffer[..n]);
                         captured_stderr.push_str(&chunk); // Capture locally
                         // Send the chunk for logging
                         if stderr_tx.send(chunk.to_string()).is_err() {
                             break; // Receiver disconnected
                         }
                     }
                     Err(_) => break, // Error reading
                 }
             }
             captured_stderr // Return captured stderr at the end
         });

         // --- Receive and log output from both threads ---
         // Drop the original tx so the loop terminates when threads finish
         // drop(tx); // This was incorrect, tx is moved into stderr_thread

         // Receive messages until the channel is closed (both threads exit)
         for received_chunk in rx {
             // Log chunks as they arrive. Note: This might interleave stdout/stderr
             // and split lines, but ensures real-time display.
             log_callback(&received_chunk);
             // We need to reconstruct stderr_output here if needed for CommandFailed error
             // For simplicity now, we'll get it from the thread join result.
         }

         // --- Wait for threads and get captured stderr ---
         stdout_thread.join().expect("Stdout reading thread panicked");
         // Join stderr thread and get the captured output
         let stderr_output = stderr_thread.join().expect("Stderr reading thread panicked"); // Declare and assign here


         let status = child.wait().map_err(|e| CoreError::CommandWait(cmd_handbrake.to_string(), e))?;


        // --- Handle Result ---
        if status.success() {
            let file_elapsed_time = file_start_time.elapsed();
            // Use ?. below, as they now return CoreResult
            let input_size = get_file_size(input_path)?;
            let output_size = get_file_size(&output_path)?;

            results.push(EncodeResult {
                filename: filename.clone(),
                duration: file_elapsed_time,
                input_size,
                output_size,
            });

            log_callback(&format!("Completed: {} in {:?}", filename, file_elapsed_time));

        } else {
            // Log error including captured stderr, then continue processing other files
             log_callback(&format!(
                "ERROR: HandBrakeCLI failed for {} with status {}. Stderr:\n{}",
                 filename, status, stderr_output.trim() // Use the captured stderr
             ));
            // Continue processing other files without adding this one to results.
            // Log error but continue processing other files
             log_callback(&format!(
                "ERROR: HandBrakeCLI failed for {} with status {}. Check log for details.",
                 filename, status
             ));
            // Consider returning a partial success / error report instead of just Vec<EncodeResult>
            // Or just log it and don't add to results, as done here.
        }
         log_callback("----------------------------------------");

    } // End loop through files

    Ok(results)
}


// --- Public Helper Functions (moved from main previously) ---

/// Formats duration into Hh Mm Ss format
pub fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{}h {}m {}s", hours, minutes, seconds)
}

/// Formats bytes into human-readable format (KiB, MiB, GiB)
pub fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;

    if bytes as f64 >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB)
    } else if bytes as f64 >= MIB {
        format!("{:.2} MiB", bytes as f64 / MIB)
    } else if bytes as f64 >= KIB {
        format!("{:.2} KiB", bytes as f64 / KIB)
    } else {
        format!("{} B", bytes)
    }
}

// Unit tests have been moved to the `tests/` directory.
