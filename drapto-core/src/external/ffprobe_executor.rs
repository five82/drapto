// drapto-core/src/external/ffprobe_executor.rs

use crate::error::{CoreError, CoreResult};
use crate::processing::detection::properties::{get_video_properties_impl, VideoProperties}; // Update path
use std::path::Path;
use std::process::Command; // Removed unused Stdio
// Removed unused serde::Deserialize

// --- Ffprobe Execution Abstraction ---

/// Trait for executing ffprobe commands.
pub trait FfprobeExecutor {
    /// Gets audio channel counts for a given input file.
    fn get_audio_channels(&self, input_path: &Path) -> CoreResult<Vec<u32>>;
    /// Gets video properties (dimensions, duration, color info) for a given input file.
    fn get_video_properties(&self, input_path: &Path) -> CoreResult<VideoProperties>;
}

/// Concrete implementation using std::process::Command.
pub struct CommandFfprobeExecutor;

impl FfprobeExecutor for CommandFfprobeExecutor {
    fn get_audio_channels(&self, input_path: &Path) -> CoreResult<Vec<u32>> {
        get_audio_channels_impl(input_path)
    }
    fn get_video_properties(&self, input_path: &Path) -> CoreResult<VideoProperties> {
        get_video_properties_impl(input_path) // Call the implementation logic
    }
}

// Original logic moved into a private implementation function
fn get_audio_channels_impl(input_path: &Path) -> CoreResult<Vec<u32>> {
    // We assume ffprobe is found directly for now.
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

// Removed get_video_properties_impl as it belongs in detection.rs (or should be called by CommandFfprobeExecutor)
// For now, CommandFfprobeExecutor calls the pub(crate) fn in detection.rs