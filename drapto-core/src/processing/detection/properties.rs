// drapto-core/src/processing/detection/properties.rs

use crate::error::{CoreError, CoreResult};
use serde::Deserialize;
use std::path::Path;
use std::process::{Command, Stdio};

// --- Structs for ffprobe JSON output ---

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct FfprobeFormat {
    duration: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct FfprobeStream {
    codec_type: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    color_space: Option<String>,
    color_transfer: Option<String>,
    color_primaries: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct FfprobeOutput {
    format: FfprobeFormat,
    streams: Vec<FfprobeStream>,
}

// --- Struct to hold extracted properties ---

#[derive(Debug, Clone, Default)]
pub struct VideoProperties { // Keep public as it's re-exported
    pub width: u32,
    pub height: u32,
    pub duration: f64,
    pub color_space: Option<String>,
    pub color_transfer: Option<String>,
    pub color_primaries: Option<String>,
}

// --- Implementation ---

/// Implementation logic for getting video properties using ffprobe command.
/// This is called by CommandFfprobeExecutor.
pub(crate) fn get_video_properties_impl(input_file: &Path) -> CoreResult<VideoProperties> {
     let cmd_ffprobe = "ffprobe";
    let args = [
        "-v", "quiet",
        "-print_format", "json",
        "-show_format",
        "-show_streams",
        &input_file.to_string_lossy(),
    ];

    log::debug!("Running ffprobe to get properties: {} {:?}", cmd_ffprobe, args);

    let output = Command::new(cmd_ffprobe)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| CoreError::CommandStart(cmd_ffprobe.to_string(), e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::error!("ffprobe failed for property check on {}: {}", input_file.display(), stderr.trim());
        return Err(CoreError::CommandFailed(
            cmd_ffprobe.to_string(),
            output.status,
            stderr.trim().to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    log::trace!("ffprobe properties output for {}: {}", input_file.display(), stdout);

    let ffprobe_data: FfprobeOutput = serde_json::from_str(&stdout)
        .map_err(|e| CoreError::JsonParseError(format!("ffprobe properties output: {}", e)))?;

    let duration = ffprobe_data.format.duration
        .as_deref()
        .and_then(|d_str| d_str.parse::<f64>().ok())
        .unwrap_or(0.0);

    let video_stream = ffprobe_data.streams.iter()
        .find(|s| s.codec_type.as_deref() == Some("video"))
        .ok_or_else(|| CoreError::VideoInfoError(format!("No video stream found in {}", input_file.display())))?;

    let width = video_stream.width.unwrap_or(0);
    let height = video_stream.height.unwrap_or(0);

    if width == 0 || height == 0 {
         return Err(CoreError::VideoInfoError(format!("Could not determine video dimensions for {}", input_file.display())));
    }

    Ok(VideoProperties {
        width,
        height,
        duration,
        color_space: video_stream.color_space.clone(),
        color_transfer: video_stream.color_transfer.clone(),
        color_primaries: video_stream.color_primaries.clone(),
    })
}