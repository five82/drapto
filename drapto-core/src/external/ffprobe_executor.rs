// drapto-core/src/external/ffprobe_executor.rs
use crate::error::{CoreError, CoreResult};
use crate::processing::detection::properties::VideoProperties; // Keep this import
use ffprobe::{ffprobe, FfProbeError}; // Import from the new crate
use std::path::Path;
// Remove unused std::process::Command

// --- Ffprobe Execution Abstraction ---

/// Trait for executing ffprobe commands.
pub trait FfprobeExecutor {
    /// Gets audio channel counts for a given input file.
    fn get_audio_channels(&self, input_path: &Path) -> CoreResult<Vec<u32>>;
    /// Gets video properties (dimensions, duration, color info) for a given input file.
    fn get_video_properties(&self, input_path: &Path) -> CoreResult<VideoProperties>;
}

// --- New Implementation using `ffprobe` crate ---

/// Concrete implementation using the `ffprobe` crate.
#[derive(Debug, Clone, Default)] // Add derive for potential future use and consistency
pub struct CrateFfprobeExecutor;

impl CrateFfprobeExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl FfprobeExecutor for CrateFfprobeExecutor {
    fn get_audio_channels(&self, input_path: &Path) -> CoreResult<Vec<u32>> {
        log::debug!("Running ffprobe (via crate) for audio channels on: {}", input_path.display());
        match ffprobe(input_path) {
            Ok(metadata) => {
                let channels: Vec<u32> = metadata
                    .streams
                    .iter()
                    .filter(|s| s.codec_type.as_deref() == Some("audio"))
                    .filter_map(|s| s.channels) // filter_map handles Option<i64> in ffprobe v0.3.3
                    .map(|c| { // Cast i64 to u32
                        if c < 0 {
                            log::warn!("Negative channel count ({}) found for {}, treating as 0", c, input_path.display());
                            0u32 // Or handle as error? Unlikely scenario.
                        } else {
                            c as u32
                        }
                    })
                    .collect();
                if channels.is_empty() {
                     log::warn!("No audio streams found by ffprobe for {}", input_path.display());
                }
                Ok(channels)
            }
            Err(err) => {
                log::error!("ffprobe (crate) failed for audio channels on {}: {:?}", input_path.display(), err);
                Err(map_ffprobe_error(err, "audio channels"))
            }
        }
    }

    fn get_video_properties(&self, input_path: &Path) -> CoreResult<VideoProperties> {
        log::debug!("Running ffprobe (via crate) for video properties on: {}", input_path.display());
        match ffprobe(input_path) {
            Ok(metadata) => {
                let duration_secs = metadata
                    .format
                    .duration
                    .as_deref()
                    .and_then(|d| d.parse::<f64>().ok())
                    .ok_or_else(|| CoreError::FfprobeParse(format!("Failed to parse duration from format for {}", input_path.display())))?;

                let video_stream = metadata
                    .streams
                    .iter()
                    .find(|s| s.codec_type.as_deref() == Some("video"))
                    .ok_or_else(|| CoreError::VideoInfoError(format!("No video stream found in {}", input_path.display())))?;

                // Use i64 from ffprobe crate and cast carefully
                let width = video_stream.width.ok_or_else(|| {
                    CoreError::VideoInfoError(format!("Video stream missing width in {}", input_path.display()))
                })?;
                 let height = video_stream.height.ok_or_else(|| {
                    CoreError::VideoInfoError(format!("Video stream missing height in {}", input_path.display()))
                })?;

                // Ensure non-negative before casting
                if width < 0 || height < 0 {
                    return Err(CoreError::VideoInfoError(format!(
                        "Invalid dimensions (negative) found in {}: width={}, height={}",
                        input_path.display(), width, height
                    )));
                }


                Ok(VideoProperties {
                    width: width as u32, // Cast after check
                    height: height as u32, // Cast after check
                    duration_secs,
                    color_space: video_stream.color_space.clone(),
                    // color_primaries and color_transfer removed as they are not in ffprobe v0.3.3 Stream struct
                })
            }
            Err(err) => {
                 log::error!("ffprobe (crate) failed for video properties on {}: {:?}", input_path.display(), err);
                Err(map_ffprobe_error(err, "video properties"))
            }
        }
    }
}

// Helper function to map ffprobe crate errors to CoreError
fn map_ffprobe_error(err: FfProbeError, context: &str) -> CoreError {
    match err {
        FfProbeError::Io(io_err) => {
             // Ambiguous if it's starting the command or reading output, lean towards CommandStart
             CoreError::CommandStart(format!("ffprobe ({})", context), io_err)
        }
        // Adjusted for ffprobe v0.3.3 FfProbeError::Status variant
        FfProbeError::Status(output) => {
            // Pass the actual ExitStatus (output.status) instead of a string representation
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            CoreError::CommandFailed(format!("ffprobe ({})", context), output.status, stderr)
        }
        // Adjusted for ffprobe v0.3.3 FfProbeError::Deserialize variant (assuming name)
        FfProbeError::Deserialize(err) => {
            CoreError::JsonParseError(format!("ffprobe {} output deserialization: {}", context, err))
        }
        // Add wildcard arm for non-exhaustive enum
        _ => CoreError::FfprobeParse(format!("Unknown ffprobe error during {}: {:?}", context, err)),
    }
}


// --- Old Implementation (Removed) ---
/*
/// Concrete implementation using std::process::Command.
pub struct CommandFfprobeExecutor;

impl FfprobeExecutor for CommandFfprobeExecutor {
    fn get_audio_channels(&self, input_path: &Path) -> CoreResult<Vec<u32>> {
        get_audio_channels_impl(input_path)
    }
    fn get_video_properties(&self, input_path: &Path) -> CoreResult<VideoProperties> {
        // Note: This relies on the function being pub(crate) in properties.rs
        crate::processing::detection::properties::get_video_properties_impl(input_path)
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
            output.status, // Keep original ExitStatus here
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
*/