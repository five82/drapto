//! Audio stream analysis and bitrate calculation.
//!
//! This module handles the analysis of audio streams in video files, including
//! detecting the number of channels and calculating appropriate bitrates for
//! encoding.

use crate::error::CoreResult;
use crate::external::get_audio_channels;

use log::warn;
use std::path::Path;

/// Returns audio bitrate in kbps based on channel count (mono:64, stereo:128, 5.1:256, 7.1:384).
pub(crate) fn calculate_audio_bitrate(channels: u32) -> u32 {
    match channels {
        1 => 64,            // Mono
        2 => 128,           // Stereo
        6 => 256,           // 5.1 surround
        8 => 384,           // 7.1 surround
        _ => channels * 48, // ~48 kbps per channel for non-standard configs
    }
}


/// Logs audio channel info and bitrates. Non-critical - continues on error.
pub fn log_audio_info(input_path: &Path) -> CoreResult<()> {
    // Extract filename for logging purposes
    let filename = input_path
        .file_name().map_or_else(|| "unknown_file".to_string(), |s| s.to_string_lossy().to_string());

    let audio_channels = match get_audio_channels(input_path) {
        Ok(channels) => channels,
        Err(e) => {
            // Audio info is non-critical - warn and continue
            warn!("Error getting audio channels for {}: {}. Cannot log bitrate info.", filename, e);

            return Ok(());
        }
    };
    if audio_channels.is_empty() {
        crate::terminal_output::print_status("Audio streams", "None detected", false);
        return Ok(());
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
    crate::terminal_output::print_status("Audio", &channel_summary, false);

    let mut bitrate_parts = Vec::new();
    for (index, &num_channels) in audio_channels.iter().enumerate() {
        let bitrate = calculate_audio_bitrate(num_channels);
        if audio_channels.len() == 1 {
            crate::terminal_output::print_status("Bitrate", &format!("{}kbps", bitrate), false);
        } else {
            bitrate_parts.push(format!("Stream {index}: {bitrate}kbps"));
        }
    }

    if audio_channels.len() > 1 {
        crate::terminal_output::print_status("Bitrates", &bitrate_parts.join(", "), false);
    }

    Ok(())
}
