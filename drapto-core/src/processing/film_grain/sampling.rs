// drapto-core/src/processing/film_grain/sampling.rs
// Responsibility: Handle the extraction and testing of video samples.

use crate::config::CoreConfig;
use crate::error::{CoreError, CoreResult};
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::tempdir;

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
    // log_callback: &mut dyn FnMut(&str), // Not needed here, logging is done by caller
) -> CoreResult<u64> {
    let temp_dir = tempdir().map_err(CoreError::Io)?;
    let output_filename = format!(
        "sample_start{}_dur{}_grain{}.mkv",
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
    let quality = config.default_quality.unwrap_or(28);
    handbrake_args.push("--quality".to_string());
    handbrake_args.push(quality.to_string());

    // Crop Mode (use config or default 'off' for samples?) - Let's use config for consistency
    if let Some(crop_mode) = &config.default_crop_mode {
        handbrake_args.push("--crop-mode".to_string());
        handbrake_args.push(crop_mode.clone());
    } else {
         // Default to 'off' if not specified, to avoid auto-crop variance affecting size
         handbrake_args.push("--crop-mode".to_string());
         handbrake_args.push("off".to_string());
    }

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
    let cmd_handbrake = "HandBrakeCLI";
    let status = Command::new(cmd_handbrake)
        .args(&handbrake_args)
        .stdout(Stdio::null()) // Ensure stdout is ignored
        .stderr(Stdio::null()) // Ensure stderr is ignored
        .status()
        .map_err(|e| CoreError::CommandStart(cmd_handbrake.to_string(), e))?;

    if !status.success() {
        // We don't have stderr here, so provide a generic error message
        return Err(CoreError::FilmGrainEncodingFailed(format!(
            "HandBrakeCLI failed for sample (start: {}, grain: {}) with status {}",
            start_secs, grain_value, status
        )));
    }

    // Get file size
    let metadata = fs::metadata(&output_path).map_err(CoreError::Io)?;
    Ok(metadata.len())

    // temp_dir and its contents are automatically cleaned up when `temp_dir` goes out of scope
}