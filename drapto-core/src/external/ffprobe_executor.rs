//! FFprobe integration for media analysis and information extraction
//!
//! This module provides functions for executing ffprobe commands to analyze
//! media files and extract properties such as dimensions, duration, audio channels,
//! for video analysis.
use crate::error::{CoreError, CoreResult, command_failed_error, command_start_error};
use crate::processing::video_properties::VideoProperties;
use ffprobe::{FfProbeError, ffprobe};
use std::path::Path;

/// Struct containing media information.
#[derive(Debug, Default, Clone)]
pub struct MediaInfo {
    /// Duration of the media in seconds
    pub duration: Option<f64>,
    /// Width of the video stream
    pub width: Option<i64>,
    /// Height of the video stream
    pub height: Option<i64>,
    /// Total number of frames in the video
    pub total_frames: Option<u64>,
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
            log::error!("ffprobe failed for audio channels on {}: {:?}", input_path.display(), err);
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
            log::error!("ffprobe failed for video properties on {}: {:?}", input_path.display(), err);
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
                
                // Get total frames from nb_frames field if available
                info.total_frames = video_stream.nb_frames
                    .as_deref()
                    .and_then(|f| f.parse::<u64>().ok());
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
