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

/// Information about an audio stream
#[derive(Debug, Clone)]
pub struct AudioStreamInfo {
    /// Number of channels
    pub channels: u32,
    /// Codec name (e.g., "truehd", "eac3", "dts")
    pub codec_name: String,
    /// Codec profile (e.g., "Atmos" for TrueHD/E-AC-3)
    pub profile: Option<String>,
    /// Stream index
    pub index: usize,
    /// Spatial audio preservation is no longer supported; this flag is always false.
    pub is_spatial: bool,
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
                "ffprobe failed for audio channels on {}: {:?}",
                input_path.display(),
                err
            );
            Err(map_ffprobe_error(err, "audio channels"))
        }
    }
}

/// Gets detailed audio stream information (spatial preservation removed)
pub fn get_audio_stream_info(input_path: &Path) -> CoreResult<Vec<AudioStreamInfo>> {
    log::debug!(
        "Running ffprobe for detailed audio stream info on: {}",
        input_path.display()
    );
    match ffprobe(input_path) {
        Ok(metadata) => {
            let mut audio_streams = Vec::new();
            let mut audio_index = 0;

            for (stream_index, stream) in metadata.streams.iter().enumerate() {
                if stream.codec_type.as_deref() != Some("audio") {
                    continue;
                }

                let channels = stream.channels.unwrap_or(0);
                if channels <= 0 {
                    log::warn!(
                        "Skipping audio stream {} with invalid channel count: {}",
                        stream_index,
                        channels
                    );
                    continue;
                }

                let codec_name = stream.codec_name.clone().unwrap_or_default();
                let profile = stream.profile.clone();
                let is_spatial = false; // Spatial audio preservation removed; always transcode to Opus

                audio_streams.push(AudioStreamInfo {
                    channels: channels as u32,
                    codec_name: codec_name.clone(),
                    profile,
                    index: audio_index,
                    is_spatial,
                });

                audio_index += 1;
            }

            if audio_streams.is_empty() {
                log::warn!(
                    "No audio streams found by ffprobe for {}",
                    input_path.display()
                );
            }

            Ok(audio_streams)
        }
        Err(err) => {
            log::error!(
                "ffprobe failed for audio stream info on {}: {:?}",
                input_path.display(),
                err
            );
            Err(map_ffprobe_error(err, "audio stream info"))
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

            // Get HDR information using MediaInfo
            let hdr_info = match crate::external::mediainfo_executor::get_media_info(input_path) {
                Ok(media_info) => {
                    crate::external::mediainfo_executor::detect_hdr_from_mediainfo(&media_info)
                }
                Err(e) => {
                    log::warn!(
                        "Failed to get MediaInfo for HDR detection: {}, defaulting to SDR",
                        e
                    );
                    crate::external::HdrInfo {
                        is_hdr: false,
                        colour_primaries: None,
                        transfer_characteristics: None,
                        matrix_coefficients: None,
                        bit_depth: None,
                    }
                }
            };

            Ok(VideoProperties {
                width: width as u32,
                height: height as u32,
                duration_secs,
                hdr_info,
            })
        }
        Err(err) => {
            log::error!(
                "ffprobe failed for video properties on {}: {:?}",
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

                // Get total frames from nb_frames field if available
                info.total_frames = video_stream
                    .nb_frames
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
        FfProbeError::Io(io_err) => command_start_error(format!("ffprobe ({context})"), io_err),
        FfProbeError::Status(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            command_failed_error(format!("ffprobe ({context})"), output.status, stderr)
        }
        FfProbeError::Deserialize(err) => {
            CoreError::JsonParseError(format!("ffprobe {context} output deserialization: {err}"))
        }
        _ => CoreError::FfprobeParse(format!("Unknown ffprobe error during {context}: {err:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test audio stream indexing
    #[test]
    fn test_audio_stream_indexing() {
        // Create mock streams with different indices
        let streams = vec![
            AudioStreamInfo {
                channels: 2,
                codec_name: "aac".to_string(),
                profile: None,
                index: 0, // First audio stream
                is_spatial: false,
            },
            AudioStreamInfo {
                channels: 6,
                codec_name: "eac3".to_string(),
                profile: Some("Dolby Digital Plus".to_string()),
                index: 1, // Second audio stream
                is_spatial: false,
            },
        ];

        assert_eq!(
            streams[0].index, 0,
            "First audio stream should have index 0"
        );
        assert_eq!(
            streams[1].index, 1,
            "Second audio stream should have index 1"
        );
        assert!(
            !streams[0].is_spatial && !streams[1].is_spatial,
            "Spatial flags should be false after removal of spatial support"
        );
    }
}
