// drapto-core/src/processing/film_grain/sampling.rs
//
// This module is responsible for the operational aspects of film grain optimization,
// specifically interacting with external tools to gather necessary information and
// perform the sample encoding tests.
//
// Functions:
// - `get_video_duration_secs`: A function (`pub(crate)`) designed to retrieve the
//   total duration of a video file in seconds. It uses `ffprobe` for this purpose.
//   Includes conditional compilation (`#[cfg(test)]`) to allow mocking its return
//   value during tests using a `thread_local` variable. (Note: There's a TODO
//   to potentially move this function to the `external` module for better separation
//   of concerns).
// - `extract_and_test_sample`: The core function (`pub(crate)`) for testing a specific
//   film grain value. It takes the input video path, a start time, a duration, the
//   grain value to test, and the core configuration. It performs the following steps:
//     1. Creates a temporary directory using the `tempfile` crate.
//     2. Constructs the necessary `HandBrakeCLI` arguments to encode only the
//        specified segment (`--start-at`, `--stop-at`) of the input video.
//     3. Applies the provided `grain_value` and other relevant encoding settings
//        from the `CoreConfig` (preset, quality, crop mode).
//     4. Disables audio (`-a none`) and subtitles (`-s none`) to speed up the
//        sample encoding process, as only the video size impact is relevant here.
//     5. Suppresses `HandBrakeCLI`'s standard output and error streams (`--verbose=0`,
//        `Stdio::null()`) to keep the main process log clean.
//     6. Executes `HandBrakeCLI` in the temporary directory.
//     7. Checks the exit status of `HandBrakeCLI`. If it fails, returns a
//        `CoreError::FilmGrainEncodingFailed`.
//     8. If successful, retrieves the file size of the generated sample `.mkv` file.
//     9. Returns the file size as a `CoreResult<u64>`.
//    The temporary directory and its contents are automatically cleaned up when the
//    function scope ends.
//
// This module acts as the bridge between the abstract analysis logic in `analysis.rs`
// and the concrete execution of external tools needed to generate the data for that analysis.

use crate::config::CoreConfig;
use crate::error::{CoreError, CoreResult};
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
// Removed unused import: use tempfile::tempdir;

// --- Constants ---
pub(crate) const DEFAULT_SAMPLE_DURATION_SECS: u32 = 10; // Made pub(crate)

// --- Mocking Helpers (Only compiled for tests) ---

#[cfg(test)]
thread_local! {
    // Mock for get_video_duration_secs
    pub(crate) static MOCK_DURATION_SECS: std::cell::Cell<Option<f64>> = std::cell::Cell::new(None);
}

// --- Functions ---

/// Gets video duration in seconds using ffprobe (or mock for tests).
/// TODO: Move this to external/mod.rs as per the refactoring plan.
pub(crate) fn get_video_duration_secs(_input_path: &Path) -> CoreResult<f64> {
    #[cfg(test)]
    {
        // Test implementation using thread_local mock
        MOCK_DURATION_SECS.with(|cell| {
            if let Some(duration) = cell.get() {
                Ok(duration)
            } else {
                Ok(300.0) // Default mock duration if not set
            }
        })
        // To test failure:
        // Err(CoreError::FfprobeParse("Mock duration failure".to_string()))
    }
    #[cfg(not(test))]
    {
        // Real implementation using ffprobe
        let cmd_name = "ffprobe";
        let output = Command::new(cmd_name)
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1", // Output only the duration value
            ])
            .arg(_input_path) // Use the parameter name
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
            .trim()
            .parse::<f64>()
            .map_err(|e| CoreError::FfprobeParse(format!("Failed to parse duration '{}': {}", stdout.trim(), e)))
    }
}


/// Encodes a sample using HandBrakeCLI with specific settings and returns the output file size.
/// Suppresses HandBrakeCLI output. Made pub(crate) for DI.
pub(crate) fn extract_and_test_sample(
    input_path: &Path,
    start_secs: f64,
    duration_secs: u32,
    grain_value: u8,
    config: &CoreConfig,
    handbrake_cmd_parts: &[String], // <-- Add this parameter
    // log_callback: &mut dyn FnMut(&str), // Not needed here, logging is done by caller
) -> CoreResult<u64> {
    // Create a dedicated subdirectory for temporary samples within the main output dir
    let samples_tmp_base_dir = config.output_dir.join("grain_samples_tmp");
    fs::create_dir_all(&samples_tmp_base_dir).map_err(CoreError::Io)?;

    // Create the temporary directory *inside* the dedicated subdirectory
    let temp_dir = tempfile::Builder::new()
        .prefix("sample_") // Optional: add a prefix
        .tempdir_in(&samples_tmp_base_dir)
        .map_err(CoreError::Io)?;

    let output_filename = format!(
        "start{}_dur{}_grain{}.mkv", // Simplified filename slightly
        start_secs.round(), duration_secs, grain_value
    );
    let output_path = temp_dir.path().join(output_filename);

    let mut handbrake_args: Vec<String> = Vec::new();

    // --- Base Parameters (mirroring lib.rs, but without audio/subs for speed) ---
    handbrake_args.push("--encoder".to_string());
    handbrake_args.push("svt_av1_10bit".to_string());
    handbrake_args.push("--encoder-tune".to_string());
    handbrake_args.push("0".to_string());

    // Dynamic film grain setting
    let encopts = format!("film-grain={}:film-grain-denoise=1", grain_value);
    handbrake_args.push("--encopts".to_string());
    handbrake_args.push(encopts);

    // Encoder Preset
    let encoder_preset = config.default_encoder_preset.unwrap_or(6);
    handbrake_args.push("--encoder-preset".to_string());
    handbrake_args.push(encoder_preset.to_string());

    // Quality
    // Use a fixed quality for sample encoding, as resolution-specific quality isn't needed here.
    // Using 27 as a reasonable default (matches previous HD default).
    let quality = 27;
    handbrake_args.push("--quality".to_string());
    handbrake_args.push(quality.to_string());

    // Crop Mode for Sampling (Use the specific config setting)
    // The CLI ensures film_grain_sample_crop_mode is always Some, defaulting to "auto".
    // We still provide a fallback here for robustness.
    let sample_crop_mode = config.film_grain_sample_crop_mode
        .as_deref() // Get &str from Option<String>
        .unwrap_or("auto"); // Default to "auto" if None (shouldn't happen)
    handbrake_args.push("--crop-mode".to_string());
    handbrake_args.push(sample_crop_mode.to_string()); // Add the argument

    // --- Sample Specific Parameters ---
    handbrake_args.push("--start-at".to_string());
    handbrake_args.push(format!("duration:{}", start_secs));
    handbrake_args.push("--stop-at".to_string());
    handbrake_args.push(format!("duration:{}", duration_secs));

    // Disable audio and subtitles for faster sample encodes
    handbrake_args.push("-a".to_string());
    handbrake_args.push("none".to_string());
    handbrake_args.push("-s".to_string());
    handbrake_args.push("none".to_string());

    // Input and Output
    handbrake_args.push("-i".to_string());
    handbrake_args.push(input_path.to_string_lossy().to_string());
    handbrake_args.push("-o".to_string());
    handbrake_args.push(output_path.to_string_lossy().to_string());

    // --- Output Suppression ---
    // Use verbose level 0 for minimal console output during sample encodes
    handbrake_args.push("--verbose=0".to_string());

    // --- Execute ---
    // Use the provided command parts
    if handbrake_cmd_parts.is_empty() {
        // Should not happen if check_dependency worked, but good practice.
        // Use FilmGrainEncodingFailed as this prevents the encoding step.
        return Err(CoreError::FilmGrainEncodingFailed("Internal error: HandBrakeCLI command parts are unexpectedly empty.".to_string()));
    }
    let handbrake_executable = &handbrake_cmd_parts[0];
    let base_args = &handbrake_cmd_parts[1..]; // e.g., ["run", "fr.handbrake..."] or empty

    // Capture output to get stderr on failure
    let output = Command::new(handbrake_executable)
        .args(base_args) // Add base args first (like "run", "fr.handbrake...")
        .args(&handbrake_args) // Then add the specific sample encode args
        .stdout(Stdio::null()) // Still ignore stdout
        .stderr(Stdio::piped()) // Capture stderr
        .output() // Use output() to get status and stderr
        .map_err(|e| CoreError::CommandStart(handbrake_executable.to_string(), e))?; // Use correct executable in error

    if !output.status.success() {
        let stderr_output = String::from_utf8_lossy(&output.stderr);
        return Err(CoreError::FilmGrainEncodingFailed(format!(
            "HandBrakeCLI failed for sample (start: {}, grain: {}) with status {}. Stderr: {}",
            start_secs, grain_value, output.status, stderr_output.trim()
        )));
    }

    // Get file size
    let metadata = fs::metadata(&output_path).map_err(CoreError::Io)?;
    Ok(metadata.len())

    // temp_dir and its contents are automatically cleaned up when `temp_dir` goes out of scope
}