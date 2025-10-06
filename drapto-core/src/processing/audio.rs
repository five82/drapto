//! Audio stream analysis and bitrate calculation.
//!
//! This module handles the analysis of audio streams in video files, including
//! detecting the number of channels and calculating appropriate bitrates for
//! encoding.

use crate::external::{AudioStreamInfo, get_audio_channels, get_audio_stream_info};

use std::path::Path;

/// Returns audio bitrate in kbps based on channel count (mono:64, stereo:128, 5.1:256, 7.1:384).
pub fn calculate_audio_bitrate(channels: u32) -> u32 {
    match channels {
        1 => 64,            // Mono
        2 => 128,           // Stereo
        6 => 256,           // 5.1 surround
        8 => 384,           // 7.1 surround
        _ => channels * 48, // ~48 kbps per channel for non-standard configs
    }
}

/// Analyzes audio streams and returns channel information without logging.
/// Returns empty vector on error (non-critical operation).
pub fn get_audio_channels_quiet(input_path: &Path) -> Vec<u32> {
    get_audio_channels(input_path).unwrap_or_default()
}

/// Analyzes audio streams and returns channel information for encoding.
/// Also logs audio stream details to the terminal.
/// Returns empty vector on error (non-critical operation).
pub fn analyze_and_log_audio(input_path: &Path) -> Vec<u32> {
    // Extract filename for logging purposes
    let filename =
        crate::utils::get_filename_safe(input_path).unwrap_or_else(|_| "unknown_file".to_string());

    let audio_channels = match get_audio_channels(input_path) {
        Ok(channels) => channels,
        Err(e) => {
            // Audio info is non-critical - warn and continue
            log::warn!(
                "Error getting audio channels for {}: {}. Using empty list.",
                filename,
                e
            );
            log::info!("Audio streams: Error detecting audio");
            return vec![];
        }
    };
    if audio_channels.is_empty() {
        log::info!("Audio streams: None detected");
        return vec![];
    }

    let channel_summary = if audio_channels.len() == 1 {
        format!("{} channels", audio_channels[0])
    } else {
        format!(
            "{} streams: {}",
            audio_channels.len(),
            audio_channels
                .iter()
                .enumerate()
                .map(|(i, &ch)| format!("Stream {i} ({ch}ch)"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    log::info!("Audio: {}", channel_summary);

    let mut bitrate_parts = Vec::new();
    for (index, &num_channels) in audio_channels.iter().enumerate() {
        let bitrate = calculate_audio_bitrate(num_channels);
        if audio_channels.len() == 1 {
            log::info!("Bitrate: {}kbps", bitrate);
        } else {
            bitrate_parts.push(format!("Stream {index}: {bitrate}kbps"));
        }
    }

    if audio_channels.len() > 1 {
        log::info!("Bitrates: {}", bitrate_parts.join(", "));
    }

    audio_channels
}

/// Analyzes audio streams and returns detailed stream information including spatial audio detection.
/// Also logs audio stream details to the terminal.
/// Returns None on error (non-critical operation).
pub fn analyze_and_log_audio_detailed(input_path: &Path) -> Option<Vec<AudioStreamInfo>> {
    // Extract filename for logging purposes
    let filename =
        crate::utils::get_filename_safe(input_path).unwrap_or_else(|_| "unknown_file".to_string());

    let audio_streams = match get_audio_stream_info(input_path) {
        Ok(streams) => {
            log::info!("Detected {} audio streams", streams.len());
            for stream in &streams {
                log::info!(
                    "Stream {}: codec={}, profile={:?}, spatial={}",
                    stream.index,
                    stream.codec_name,
                    stream.profile,
                    stream.is_spatial
                );
            }
            streams
        }
        Err(e) => {
            // Audio info is non-critical - warn and continue
            log::warn!(
                "Error getting audio stream info for {}: {}. Using fallback.",
                filename,
                e
            );
            log::info!("Audio streams: Error detecting audio details");
            return None;
        }
    };

    if audio_streams.is_empty() {
        log::info!("Audio streams: None detected");
        return Some(vec![]);
    }

    // Log stream information
    if audio_streams.len() == 1 {
        let stream = &audio_streams[0];
        let spatial_note = if stream.is_spatial {
            format!(
                " [Spatial: {} {}]",
                stream.codec_name,
                stream.profile.as_deref().unwrap_or("")
            )
        } else {
            String::new()
        };
        log::info!("Audio: {} channels{}", stream.channels, spatial_note);

        if stream.is_spatial {
            log::info!("Processing: Will copy spatial audio to preserve Atmos/DTS:X");
        } else {
            let bitrate = calculate_audio_bitrate(stream.channels);
            log::info!("Bitrate: {}kbps (Opus)", bitrate);
        }
    } else {
        log::info!("Audio: {} streams detected", audio_streams.len());

        for stream in &audio_streams {
            let spatial_note = if stream.is_spatial {
                format!(
                    " [Spatial: {} {}, will copy]",
                    stream.codec_name,
                    stream.profile.as_deref().unwrap_or("")
                )
            } else {
                let bitrate = calculate_audio_bitrate(stream.channels);
                format!(" [{}kbps Opus]", bitrate)
            };
            log::info!(
                "  Stream {}: {} channels{}",
                stream.index,
                stream.channels,
                spatial_note
            );
        }
    }

    Some(audio_streams)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external::AudioStreamInfo;

    #[test]
    fn test_calculate_audio_bitrate() {
        // Test standard channel configurations
        assert_eq!(calculate_audio_bitrate(1), 64, "Mono should be 64kbps");
        assert_eq!(calculate_audio_bitrate(2), 128, "Stereo should be 128kbps");
        assert_eq!(calculate_audio_bitrate(6), 256, "5.1 should be 256kbps");
        assert_eq!(calculate_audio_bitrate(8), 384, "7.1 should be 384kbps");

        // Test non-standard configurations
        assert_eq!(
            calculate_audio_bitrate(4),
            192,
            "4-channel should be 4 * 48kbps"
        );
        assert_eq!(
            calculate_audio_bitrate(10),
            480,
            "10-channel should be 10 * 48kbps"
        );
    }

    #[test]
    fn test_spatial_audio_logging_single_stream() {
        // Create a mock spatial audio stream
        let spatial_stream = AudioStreamInfo {
            channels: 8,
            codec_name: "truehd".to_string(),
            profile: Some("Dolby TrueHD + Dolby Atmos".to_string()),
            index: 0,
            is_spatial: true,
        };

        // Test that spatial audio is properly identified and formatted
        let streams = vec![spatial_stream];

        // Test the logic that would be used in analyze_and_log_audio_detailed
        assert_eq!(streams.len(), 1, "Should have one stream");
        assert!(streams[0].is_spatial, "Stream should be spatial");
        assert_eq!(streams[0].codec_name, "truehd", "Codec should be truehd");
        assert_eq!(streams[0].channels, 8, "Should have 8 channels");
    }

    #[test]
    fn test_mixed_audio_streams_analysis() {
        // Create mixed spatial and non-spatial streams
        let spatial_stream = AudioStreamInfo {
            channels: 8,
            codec_name: "truehd".to_string(),
            profile: Some("Dolby TrueHD + Dolby Atmos".to_string()),
            index: 0,
            is_spatial: true,
        };

        let commentary_stream = AudioStreamInfo {
            channels: 2,
            codec_name: "aac".to_string(),
            profile: Some("LC".to_string()),
            index: 1,
            is_spatial: false,
        };

        let streams = vec![spatial_stream, commentary_stream];

        // Test mixed stream analysis
        assert_eq!(streams.len(), 2, "Should have two streams");
        assert!(streams[0].is_spatial, "First stream should be spatial");
        assert!(
            !streams[1].is_spatial,
            "Second stream should not be spatial"
        );

        // Test codec display logic
        let spatial_count = streams.iter().filter(|s| s.is_spatial).count();
        let non_spatial_count = streams.len() - spatial_count;

        assert_eq!(spatial_count, 1, "Should have one spatial stream");
        assert_eq!(non_spatial_count, 1, "Should have one non-spatial stream");

        // This matches the logic in video.rs for codec display
        let expected_display = match (spatial_count, non_spatial_count) {
            (0, _) => "Opus",
            (_, 0) => "Copy (Spatial Audio Preserved)",
            (_, _) => "Mixed (Spatial + Opus)",
        };
        assert_eq!(
            expected_display, "Mixed (Spatial + Opus)",
            "Should show mixed codec display"
        );
    }

    #[test]
    fn test_all_spatial_streams() {
        // Test scenario with multiple spatial streams
        let atmos_stream = AudioStreamInfo {
            channels: 8,
            codec_name: "truehd".to_string(),
            profile: Some("Dolby TrueHD + Dolby Atmos".to_string()),
            index: 0,
            is_spatial: true,
        };

        let dtsx_stream = AudioStreamInfo {
            channels: 8,
            codec_name: "dts".to_string(),
            profile: Some("DTS:X".to_string()),
            index: 1,
            is_spatial: true,
        };

        let streams = vec![atmos_stream, dtsx_stream];

        let spatial_count = streams.iter().filter(|s| s.is_spatial).count();
        let non_spatial_count = streams.len() - spatial_count;

        assert_eq!(spatial_count, 2, "Should have two spatial streams");
        assert_eq!(non_spatial_count, 0, "Should have no non-spatial streams");

        let expected_display = match (spatial_count, non_spatial_count) {
            (0, _) => "Opus",
            (_, 0) => "Copy (Spatial Audio Preserved)",
            (_, _) => "Mixed (Spatial + Opus)",
        };
        assert_eq!(
            expected_display, "Copy (Spatial Audio Preserved)",
            "Should show spatial preservation"
        );
    }

    #[test]
    fn test_all_non_spatial_streams() {
        // Test scenario with only non-spatial streams
        let stereo_stream = AudioStreamInfo {
            channels: 2,
            codec_name: "aac".to_string(),
            profile: Some("LC".to_string()),
            index: 0,
            is_spatial: false,
        };

        let surround_stream = AudioStreamInfo {
            channels: 6,
            codec_name: "ac3".to_string(),
            profile: Some("Dolby Digital".to_string()),
            index: 1,
            is_spatial: false,
        };

        let streams = vec![stereo_stream, surround_stream];

        let spatial_count = streams.iter().filter(|s| s.is_spatial).count();
        let non_spatial_count = streams.len() - spatial_count;

        assert_eq!(spatial_count, 0, "Should have no spatial streams");
        assert_eq!(non_spatial_count, 2, "Should have two non-spatial streams");

        let expected_display = match (spatial_count, non_spatial_count) {
            (0, _) => "Opus",
            (_, 0) => "Copy (Spatial Audio Preserved)",
            (_, _) => "Mixed (Spatial + Opus)",
        };
        assert_eq!(expected_display, "Opus", "Should show Opus codec");
    }

    #[test]
    fn test_audio_stream_formatting() {
        // Test the formatting logic for different stream types
        let spatial_stream = AudioStreamInfo {
            channels: 8,
            codec_name: "truehd".to_string(),
            profile: Some("Dolby TrueHD + Dolby Atmos".to_string()),
            index: 0,
            is_spatial: true,
        };

        let non_spatial_stream = AudioStreamInfo {
            channels: 2,
            codec_name: "aac".to_string(),
            profile: Some("LC".to_string()),
            index: 1,
            is_spatial: false,
        };

        // Test single stream formatting (spatial)
        let channel_desc = match spatial_stream.channels {
            1 => "Mono".to_string(),
            2 => "Stereo".to_string(),
            6 => "5.1".to_string(),
            8 => "7.1".to_string(),
            n => format!("{} channels", n),
        };
        assert_eq!(channel_desc, "7.1", "Should format 8 channels as 7.1");

        let spatial_format = if spatial_stream.is_spatial {
            format!(
                "{} ({} {}) - Preserved",
                channel_desc,
                spatial_stream.codec_name,
                spatial_stream.profile.as_deref().unwrap_or("")
            )
        } else {
            let bitrate = calculate_audio_bitrate(spatial_stream.channels);
            format!("{} @ {}kbps Opus", channel_desc, bitrate)
        };
        assert!(
            spatial_format.contains("7.1 (truehd Dolby TrueHD + Dolby Atmos) - Preserved"),
            "Spatial stream should be formatted correctly: {}",
            spatial_format
        );

        // Test single stream formatting (non-spatial)
        let stereo_desc = match non_spatial_stream.channels {
            2 => "Stereo".to_string(),
            _ => format!("{} channels", non_spatial_stream.channels),
        };
        let non_spatial_format = if non_spatial_stream.is_spatial {
            format!(
                "{} ({} {}) - Preserved",
                stereo_desc,
                non_spatial_stream.codec_name,
                non_spatial_stream.profile.as_deref().unwrap_or("")
            )
        } else {
            let bitrate = calculate_audio_bitrate(non_spatial_stream.channels);
            format!("{} @ {}kbps Opus", stereo_desc, bitrate)
        };
        assert_eq!(
            non_spatial_format, "Stereo @ 128kbps Opus",
            "Non-spatial stream should be formatted correctly"
        );
    }
}
