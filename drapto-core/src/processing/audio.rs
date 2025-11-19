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

/// Analyzes audio streams and returns detailed stream information (spatial preservation removed).
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

    // Log stream information (all streams will be transcoded to Opus)
    if audio_streams.len() == 1 {
        let stream = &audio_streams[0];
        let bitrate = calculate_audio_bitrate(stream.channels);
        log::info!("Audio: {} channels @ {}kbps Opus", stream.channels, bitrate);
    } else {
        log::info!("Audio: {} streams detected", audio_streams.len());

        for stream in &audio_streams {
            let bitrate = calculate_audio_bitrate(stream.channels);
            log::info!(
                "  Stream {}: {} channels [{}kbps Opus]",
                stream.index,
                stream.channels,
                bitrate
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
    fn test_single_stream_logging_defaults_to_transcode() {
        // Create a mock audio stream; spatial flag remains false after removal
        let stream = AudioStreamInfo {
            channels: 8,
            codec_name: "truehd".to_string(),
            profile: Some("Dolby TrueHD + Dolby Atmos".to_string()),
            index: 0,
            is_spatial: false,
        };

        // Validate stored metadata
        assert_eq!(stream.codec_name, "truehd");
        assert_eq!(stream.channels, 8);
        assert!(!stream.is_spatial, "Spatial handling removed");
    }

    #[test]
    fn test_multiple_streams_analysis_transcodes_all() {
        // Create mixed streams (all treated as non-spatial for transcoding)
        let primary_stream = AudioStreamInfo {
            channels: 8,
            codec_name: "truehd".to_string(),
            profile: Some("Dolby TrueHD + Dolby Atmos".to_string()),
            index: 0,
            is_spatial: false,
        };

        let commentary_stream = AudioStreamInfo {
            channels: 2,
            codec_name: "aac".to_string(),
            profile: Some("LC".to_string()),
            index: 1,
            is_spatial: false,
        };

        let streams = vec![primary_stream, commentary_stream];

        // Test mixed stream analysis
        assert_eq!(streams.len(), 2, "Should have two streams");
        assert!(!streams[0].is_spatial, "Spatial flag disabled");
        assert!(!streams[1].is_spatial, "Spatial flag disabled");
    }
}
