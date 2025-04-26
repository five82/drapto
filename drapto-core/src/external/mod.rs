// drapto-core/src/external/mod.rs
//
// Encapsulates interactions with external CLI tools like ffmpeg and ffprobe.
// Provides functions for dependency checking and abstracting tool execution.

use crate::error::{CoreError, CoreResult};
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};
pub mod ffmpeg;

// TODO: Move get_video_duration_secs here.
// TODO: Extract ffmpeg command-building logic to ffmpeg.rs.
// Consider creating external/ffprobe.rs later.

/// Checks if a required external command is available and executable.
/// Returns the command parts (e.g., `["ffmpeg"]`) if found,
/// otherwise returns an error.
pub(crate) fn check_dependency(cmd_name: &str) -> CoreResult<Vec<String>> {
    // Use a version flag that typically exits quickly if the command exists
    // Both ffmpeg and ffprobe use "-version"
    let version_arg = "-version";

    // --- First attempt: Direct command ---
    let direct_cmd_parts = vec![cmd_name.to_string()];
    let direct_result = Command::new(&direct_cmd_parts[0])
        .arg(version_arg)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match direct_result {
        Ok(_) => {
            log::debug!("Found dependency directly: {}", cmd_name);
            return Ok(direct_cmd_parts);
        }
        Err(e) => {
            // If the direct command failed, return the appropriate error
            if e.kind() == io::ErrorKind::NotFound {
                log::warn!("Dependency '{}' not found.", cmd_name);
                Err(CoreError::DependencyNotFound(cmd_name.to_string()))
            } else {
                log::error!("Failed to start dependency check command '{}': {}", cmd_name, e);
                // Use CommandStart for errors other than NotFound when trying to run the check.
                Err(CoreError::CommandStart(cmd_name.to_string(), e))
            }
        }
    }
}


// Gets audio channel counts using ffprobe
#[cfg(not(feature = "test-mock-ffprobe"))] // Original implementation
pub(crate) fn get_audio_channels(input_path: &Path) -> CoreResult<Vec<u32>> {
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

// Mock implementation for get_audio_channels
#[cfg(feature = "test-mock-ffprobe")]
pub(crate) fn get_audio_channels(input_path: &Path) -> CoreResult<Vec<u32>> {
    log::warn!("Using MOCKED get_audio_channels for path: {}", input_path.display());
    // Return some default valid data for testing purposes
    Ok(vec![2]) // e.g., Stereo
}


// Removed get_video_width function and related structs (FfprobeOutput, StreamInfo)
// as width is now obtained via detection::get_video_properties.