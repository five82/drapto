//! Audio stream analysis and bitrate calculation.
//!
//! This module handles the analysis of audio streams in video files, including
//! detecting the number of channels and calculating appropriate bitrates for
//! encoding.

use crate::external::get_audio_channels;

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
    let filename = crate::utils::get_filename_safe(input_path)
        .unwrap_or_else(|_| "unknown_file".to_string());

    let audio_channels = match get_audio_channels(input_path) {
        Ok(channels) => channels,
        Err(e) => {
            // Audio info is non-critical - warn and continue
            log::warn!("Error getting audio channels for {}: {}. Using empty list.", filename, e);
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

