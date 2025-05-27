//! FFprobe integration for media analysis and information extraction
//!
//! This module provides functions for executing ffprobe commands to analyze
//! media files and extract properties such as dimensions, duration, audio channels,
//! and bitplane noise for grain analysis.
use crate::error::{CoreError, CoreResult, command_failed_error, command_start_error};
use crate::processing::video_properties::VideoProperties;
use ffprobe::{FfProbeError, ffprobe};
use std::path::Path;
use std::process::Command;

/// Struct containing media information.
#[derive(Debug, Default, Clone)]
pub struct MediaInfo {
    /// Duration of the media in seconds
    pub duration: Option<f64>,
    /// Width of the video stream
    pub width: Option<i64>,
    /// Height of the video stream
    pub height: Option<i64>,
}


/// Runs ffprobe with bitplanenoise filter to analyze grain.
pub fn run_ffprobe_bitplanenoise(input_path: &Path, duration_secs: f64) -> CoreResult<Vec<f32>> {
    let cmd_name = "ffprobe";
    const TARGET_SAMPLES: f64 = 10.0;
    let sample_interval = if duration_secs > 0.0 {
        (duration_secs / TARGET_SAMPLES).max(0.1)
    } else {
        1.0
    };

    let input_path_str = input_path.to_str().ok_or_else(|| {
        CoreError::PathError(format!(
            "Input path is not valid UTF-8: {}",
            input_path.display()
        ))
    })?;

    // Escape filename for filter graph
    let escaped_input_path = input_path_str.replace('\'', "'\\''");
    let filter_graph = format!(
        "movie='{escaped_input_path}',select='isnan(prev_selected_t)+gte(t-prev_selected_t\\,{sample_interval:.3})',bitplanenoise,metadata=print"
    );

    log::debug!(
        "Running {} for bitplanenoise on: {}",
        cmd_name,
        input_path.display()
    );
    log::trace!("Filter graph: {filter_graph}");

    let output = Command::new(cmd_name)
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            &filter_graph,
            "-show_entries",
            "frame_tags=lavfi.bitplanenoise.0.1",
            "-of",
            "csv=p=0",
        ])
        .output()
        .map_err(|e| command_start_error(cmd_name, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        log::error!(
            "{} bitplanenoise failed for {}. Status: {}. Stderr: {}",
            cmd_name,
            input_path.display(),
            output.status,
            stderr
        );
        return Err(command_failed_error(
            format!("{cmd_name} bitplanenoise"),
            output.status,
            stderr,
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    log::trace!(
        "{} bitplanenoise stdout for {}:\n{}",
        cmd_name,
        input_path.display(),
        stdout
    );

    let mut results: Vec<f32> = Vec::new();
    for line in stdout.lines() {
        let trimmed_line = line.trim();
        if trimmed_line.is_empty() {
            continue;
        }
        let value_str = trimmed_line.strip_suffix(',').unwrap_or(trimmed_line);
        match value_str.parse::<f32>() {
            Ok(n1) => results.push(n1),
            Err(_) => log::warn!(
                "Failed to parse bitplanenoise value as f32: '{}' (original line: '{}') for {}",
                value_str,
                trimmed_line,
                input_path.display()
            ),
        }
    }

    if results.is_empty() {
        log::trace!(
            "{} bitplanenoise analysis produced no valid results for {}. Stdout content was:\n---\n{}\n---",
            cmd_name,
            input_path.display(),
            stdout
        );
    }
    Ok(results)
}

/// Gets audio channel counts for a given input file.
pub fn get_audio_channels(input_path: &Path) -> CoreResult<Vec<u32>> {
    log::debug!(
        "Running ffprobe (via crate) for audio channels on: {}",
        input_path.display()
    );
    match ffprobe(input_path) {
        Ok(metadata) => {
            let channels: Vec<u32> = metadata
                .streams
                .iter()
                .filter(|s| s.codec_type.as_deref() == Some("audio"))
                .filter_map(|s| s.channels)
                .map(|c| {
                    if c < 0 {
                        log::warn!(
                            "Negative channel count ({}) found for {}, treating as 0",
                            c,
                            input_path.display()
                        );
                        0u32
                    } else {
                        c as u32
                    }
                })
                .collect();
            if channels.is_empty() {
                log::warn!(
                    "No audio streams found by ffprobe for {}",
                    input_path.display()
                );
            }
            Ok(channels)
        }
        Err(err) => {
            log::error!(
                "ffprobe (crate) failed for audio channels on {}: {:?}",
                input_path.display(),
                err
            );
            Err(map_ffprobe_error(err, "audio channels"))
        }
    }
}

/// Gets video properties for a given input file.
pub fn get_video_properties(input_path: &Path) -> CoreResult<VideoProperties> {
    log::debug!(
        "Running ffprobe (via crate) for video properties on: {}",
        input_path.display()
    );
    match ffprobe(input_path) {
        Ok(metadata) => {
            let duration_secs = metadata
                .format
                .duration
                .as_deref()
                .and_then(|d| d.parse::<f64>().ok())
                .ok_or_else(|| {
                    CoreError::FfprobeParse(format!(
                        "Failed to parse duration from format for {}",
                        input_path.display()
                    ))
                })?;

            let video_stream = metadata
                .streams
                .iter()
                .find(|s| s.codec_type.as_deref() == Some("video"))
                .ok_or_else(|| {
                    CoreError::VideoInfoError(format!(
                        "No video stream found in {}",
                        input_path.display()
                    ))
                })?;
            let width = video_stream.width.ok_or_else(|| {
                CoreError::VideoInfoError(format!(
                    "Video stream missing width in {}",
                    input_path.display()
                ))
            })?;
            let height = video_stream.height.ok_or_else(|| {
                CoreError::VideoInfoError(format!(
                    "Video stream missing height in {}",
                    input_path.display()
                ))
            })?;

            if width < 0 || height < 0 {
                return Err(CoreError::VideoInfoError(format!(
                    "Invalid dimensions (negative) found in {}: width={}, height={}",
                    input_path.display(),
                    width,
                    height
                )));
            }

            Ok(VideoProperties {
                width: width as u32,
                height: height as u32,
                duration_secs,
                color_space: video_stream.color_space.clone(),
            })
        }
        Err(err) => {
            log::error!(
                "ffprobe (crate) failed for video properties on {}: {:?}",
                input_path.display(),
                err
            );
            Err(map_ffprobe_error(err, "video properties"))
        }
    }
}

/// Gets media information for a given input file.
pub fn get_media_info(input_path: &Path) -> CoreResult<MediaInfo> {
    log::debug!(
        "Running ffprobe (via crate) for media info on: {}",
        input_path.display()
    );
    match ffprobe(input_path) {
        Ok(metadata) => {
            let duration = metadata
                .format
                .duration
                .as_deref()
                .and_then(|d| d.parse::<f64>().ok());

            let mut info = MediaInfo {
                duration,
                ..Default::default()
            };

            if let Some(video_stream) = metadata
                .streams
                .iter()
                .find(|s| s.codec_type.as_deref() == Some("video"))
            {
                info.width = video_stream.width;
                info.height = video_stream.height;
            }

            Ok(info)
        }
        Err(err) => {
            log::warn!("Failed to get media info: {err:?}");
            Err(map_ffprobe_error(err, "media info"))
        }
    }
}

fn map_ffprobe_error(err: FfProbeError, context: &str) -> CoreError {
    match err {
        FfProbeError::Io(io_err) => {
            command_start_error(format!("ffprobe ({context})"), io_err)
        }
        FfProbeError::Status(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            command_failed_error(format!("ffprobe ({context})"), output.status, stderr)
        }
        FfProbeError::Deserialize(err) => CoreError::JsonParseError(format!(
            "ffprobe {context} output deserialization: {err}"
        )),
        _ => CoreError::FfprobeParse(format!(
            "Unknown ffprobe error during {context}: {err:?}"
        )),
    }
}
