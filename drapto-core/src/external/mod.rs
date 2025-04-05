// drapto-core/src/external/mod.rs
//
// This module encapsulates all interactions with external command-line interface (CLI)
// tools that `drapto-core` relies on, such as `ffprobe` (for media analysis) and
// `HandBrakeCLI` (for encoding).
//
// Its primary responsibilities include:
// - Providing functions to check for the presence and executability of required
//   external tools (`check_dependency`).
// - Abstracting the execution of these tools and parsing their output.
// - Defining helper functions that utilize these tools to gather information
//   (e.g., `get_audio_channels` using `ffprobe`).
// - (Future) Containing the logic for constructing and executing HandBrakeCLI commands.
//
// Functions within this module are typically marked `pub(crate)` as they represent
// internal implementation details of the core library, not intended for direct
// external consumption, but used by other modules within `drapto-core` (like `processing`).
//
// Consider creating sub-modules like `external::ffprobe` and `external::handbrake`
// as the complexity grows.

use crate::error::{CoreError, CoreResult}; // Use crate:: prefix
use std::io;
use std::path::Path;
use serde::Deserialize; // Added for JSON parsing
use std::process::{Command, Stdio};

// TODO: Move get_video_duration_secs here.
// TODO: Extract HandBrakeCLI command-building logic here.
// Consider creating sub-modules like external/ffprobe.rs and external/handbrake.rs later.

/// Checks if a required external command is available and executable.
pub(crate) fn check_dependency(cmd_name: &str) -> CoreResult<()> {
    // Use a version flag that typically exits quickly if the command exists
    let version_arg = if cmd_name == "ffprobe" { "-version" } else { "--version" };
    Command::new(cmd_name)
        .arg(version_arg)
        .stdout(Stdio::null()) // Don't capture stdout
        .stderr(Stdio::null()) // Don't capture stderr
        .status() // Use status() to wait and get only the exit status
        .map_err(|e| {
            // Specifically check if the error is because the command wasn't found
            if e.kind() == io::ErrorKind::NotFound {
                CoreError::DependencyNotFound(cmd_name.to_string())
            } else {
                // Other errors during spawn (e.g., permissions)
                CoreError::CommandStart(cmd_name.to_string(), e)
            }
        })?;

    // Check if the command executed successfully (exit code 0)
    // Some tools might return non-zero even for --version if other args are needed,
    // but this is a common pattern. If output() succeeded without an IoError::NotFound,
    // it implies the command was found and started. A non-zero exit might indicate
    // an issue, but for this check, we primarily care that it *can* be executed.
    // Re-evaluating if `!output.status.success()` should also be DependencyNotFound.
    // For now, if `output()` succeeds, we assume the dependency is met.
    // Let's refine: if `output()` works but status is non-zero, it *was* found.
    // The original `map_err` handles the "not found" case correctly.
    // So if we reach here, it was found.
    Ok(())
}


// Gets audio channel counts using ffprobe
pub(crate) fn get_audio_channels(input_path: &Path) -> CoreResult<Vec<u32>> {
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

// TODO: Move get_video_duration_secs from processing/film_grain.rs here


// --- ffprobe JSON Structures ---

#[derive(Deserialize, Debug)]
struct FfprobeOutput {
    streams: Vec<StreamInfo>,
}

#[derive(Deserialize, Debug)]
struct StreamInfo {
    // codec_type: String, // Removed as it's unused
    width: Option<u32>,
    // Add other fields if needed later, e.g., height, duration
}


/// Gets the width of the first video stream in the file using ffprobe.
///
/// # Arguments
///
/// * `file_path` - Path to the video file.
///
/// # Returns
///
/// * `CoreResult<u32>` - The width of the video stream, or an error.
pub fn get_video_width(file_path: &Path) -> CoreResult<u32> {
    let cmd_ffprobe = "ffprobe";
    let args = [
        "-v", "error", // Only show errors
        "-select_streams", "v:0", // Select the first video stream
        "-show_entries", "stream=width", // Show only the width entry
        "-of", "json", // Output in JSON format
        &file_path.to_string_lossy(),
    ];

    // Using log crate assuming it's configured elsewhere (e.g., in drapto-cli)
    // If not, replace log::debug!/error!/trace! with println! or similar for now.
    log::debug!("Running ffprobe to get width: {} {:?}", cmd_ffprobe, args);

    let output = Command::new(cmd_ffprobe)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped()) // Capture stderr as well
        .output()
        .map_err(|e| CoreError::CommandStart(cmd_ffprobe.to_string(), e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::error!("ffprobe failed for width check on {}: {}", file_path.display(), stderr.trim());
        // Use the existing CommandFailed variant, assuming it takes status code and stderr string
        return Err(CoreError::CommandFailed(
            cmd_ffprobe.to_string(),
            output.status, // Pass the whole status
            stderr.trim().to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    log::trace!("ffprobe width output for {}: {}", file_path.display(), stdout);

    let ffprobe_data: FfprobeOutput = serde_json::from_str(&stdout)
        .map_err(|e| CoreError::JsonParseError(format!("ffprobe width output: {}", e)))?;

    // Find the first stream (should be the only one due to -select_streams v:0)
    // and extract its width.
    ffprobe_data.streams
        .first()
        .and_then(|stream| stream.width) // Get the width if the stream and width exist
        .ok_or_else(|| CoreError::VideoInfoError(format!("Could not find video width for {}", file_path.display())))
}