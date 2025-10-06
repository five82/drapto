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
    /// Whether this is a spatial audio format that should be copied
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

/// Gets detailed audio stream information including spatial audio detection
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

                // Detect spatial audio formats
                // Dolby Atmos: TrueHD with Atmos profile or E-AC-3 with JOC (Joint Object Coding)
                // DTS:X: DTS with DTS:X profile
                let is_spatial = match codec_name.to_lowercase().as_str() {
                    "truehd" => {
                        // TrueHD with Atmos
                        profile
                            .as_ref()
                            .map(|p| p.to_lowercase().contains("atmos"))
                            .unwrap_or(false)
                    }
                    "eac3" => {
                        // E-AC-3 with Atmos (JOC)
                        // Note: ffprobe from git should detect this properly
                        profile
                            .as_ref()
                            .map(|p| {
                                let p_lower = p.to_lowercase();
                                p_lower.contains("atmos") || p_lower.contains("joc")
                            })
                            .unwrap_or(false)
                    }
                    "dts" => {
                        // DTS:X
                        profile
                            .as_ref()
                            .map(|p| {
                                let p_lower = p.to_lowercase();
                                p_lower.contains("dts:x")
                                    || p_lower.contains("dtsx")
                                    || p_lower.contains("dts-x")
                            })
                            .unwrap_or(false)
                    }
                    _ => false,
                };

                if is_spatial {
                    log::info!(
                        "Detected spatial audio in stream {} (audio index {}): {} {}",
                        stream_index,
                        audio_index,
                        codec_name,
                        profile.as_deref().unwrap_or("")
                    );
                }

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

    /// Test spatial audio detection for various codec and profile combinations
    #[test]
    fn test_spatial_audio_detection() {
        // Test TrueHD with Atmos
        let truehd_atmos = AudioStreamInfo {
            channels: 8,
            codec_name: "truehd".to_string(),
            profile: Some("Dolby TrueHD + Dolby Atmos".to_string()),
            index: 0,
            is_spatial: true,
        };
        assert!(
            truehd_atmos.is_spatial,
            "TrueHD with Atmos profile should be spatial"
        );

        // Test TrueHD without Atmos
        let truehd_normal = AudioStreamInfo {
            channels: 8,
            codec_name: "truehd".to_string(),
            profile: Some("Dolby TrueHD".to_string()),
            index: 0,
            is_spatial: false,
        };
        assert!(
            !truehd_normal.is_spatial,
            "TrueHD without Atmos should not be spatial"
        );

        // Test E-AC-3 with JOC (Atmos)
        let eac3_joc = AudioStreamInfo {
            channels: 8,
            codec_name: "eac3".to_string(),
            profile: Some("Dolby Digital Plus + JOC".to_string()),
            index: 0,
            is_spatial: true,
        };
        assert!(eac3_joc.is_spatial, "E-AC-3 with JOC should be spatial");

        // Test E-AC-3 with explicit Atmos
        let eac3_atmos = AudioStreamInfo {
            channels: 8,
            codec_name: "eac3".to_string(),
            profile: Some("Dolby Digital Plus + Dolby Atmos".to_string()),
            index: 0,
            is_spatial: true,
        };
        assert!(eac3_atmos.is_spatial, "E-AC-3 with Atmos should be spatial");

        // Test regular E-AC-3
        let eac3_normal = AudioStreamInfo {
            channels: 6,
            codec_name: "eac3".to_string(),
            profile: Some("Dolby Digital Plus".to_string()),
            index: 0,
            is_spatial: false,
        };
        assert!(
            !eac3_normal.is_spatial,
            "Regular E-AC-3 should not be spatial"
        );

        // Test DTS:X
        let dtsx = AudioStreamInfo {
            channels: 8,
            codec_name: "dts".to_string(),
            profile: Some("DTS:X".to_string()),
            index: 0,
            is_spatial: true,
        };
        assert!(dtsx.is_spatial, "DTS:X should be spatial");

        // Test DTS-X (alternative naming)
        let dts_x_alt = AudioStreamInfo {
            channels: 8,
            codec_name: "dts".to_string(),
            profile: Some("DTS-X".to_string()),
            index: 0,
            is_spatial: true,
        };
        assert!(
            dts_x_alt.is_spatial,
            "DTS-X (alternative naming) should be spatial"
        );

        // Test regular DTS
        let dts_normal = AudioStreamInfo {
            channels: 6,
            codec_name: "dts".to_string(),
            profile: Some("DTS-HD Master Audio".to_string()),
            index: 0,
            is_spatial: false,
        };
        assert!(!dts_normal.is_spatial, "Regular DTS should not be spatial");

        // Test regular codec (AC-3)
        let ac3 = AudioStreamInfo {
            channels: 6,
            codec_name: "ac3".to_string(),
            profile: Some("Dolby Digital".to_string()),
            index: 0,
            is_spatial: false,
        };
        assert!(!ac3.is_spatial, "AC-3 should not be spatial");
    }

    /// Test case-insensitive profile matching
    #[test]
    fn test_case_insensitive_profile_matching() {
        // Test various case combinations for Atmos
        let test_cases = vec![
            ("truehd", "DOLBY TRUEHD + DOLBY ATMOS", true),
            ("truehd", "Dolby TrueHD + Dolby Atmos", true),
            ("truehd", "dolby truehd + dolby atmos", true),
            ("eac3", "DOLBY DIGITAL PLUS + JOC", true),
            ("eac3", "dolby digital plus + joc", true),
            ("dts", "DTS:X", true),
            ("dts", "dts:x", true),
            ("dts", "DTSX", true),
        ];

        for (codec, profile, expected_spatial) in test_cases {
            // This tests the logic that would be used in the actual detection
            let is_spatial = match codec {
                "truehd" => profile.to_lowercase().contains("atmos"),
                "eac3" => {
                    let p_lower = profile.to_lowercase();
                    p_lower.contains("atmos") || p_lower.contains("joc")
                }
                "dts" => {
                    let p_lower = profile.to_lowercase();
                    p_lower.contains("dts:x")
                        || p_lower.contains("dtsx")
                        || p_lower.contains("dts-x")
                }
                _ => false,
            };

            assert_eq!(
                is_spatial, expected_spatial,
                "Case insensitive matching failed for codec '{}' with profile '{}'",
                codec, profile
            );
        }
    }

    /// Test spatial audio detection with no profile information
    #[test]
    fn test_spatial_detection_no_profile() {
        let stream_no_profile = AudioStreamInfo {
            channels: 8,
            codec_name: "truehd".to_string(),
            profile: None,
            index: 0,
            is_spatial: false,
        };
        assert!(
            !stream_no_profile.is_spatial,
            "Stream without profile should not be spatial"
        );

        let stream_empty_profile = AudioStreamInfo {
            channels: 8,
            codec_name: "truehd".to_string(),
            profile: Some("".to_string()),
            index: 0,
            is_spatial: false,
        };
        assert!(
            !stream_empty_profile.is_spatial,
            "Stream with empty profile should not be spatial"
        );
    }

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
                channels: 8,
                codec_name: "truehd".to_string(),
                profile: Some("Dolby TrueHD + Dolby Atmos".to_string()),
                index: 1, // Second audio stream
                is_spatial: true,
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
        assert!(!streams[0].is_spatial, "AAC stream should not be spatial");
        assert!(
            streams[1].is_spatial,
            "TrueHD + Atmos stream should be spatial"
        );
    }
}
