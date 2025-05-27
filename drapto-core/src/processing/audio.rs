//! Audio stream analysis and bitrate calculation.
//!
//! This module handles the analysis of audio streams in video files, including
//! detecting the number of channels and calculating appropriate bitrates for
//! encoding.

use crate::error::CoreResult;
use crate::external::get_audio_channels;

use std::path::Path;

/// Calculates the appropriate audio bitrate based on the number of channels.
///
/// This function determines the optimal audio bitrate for encoding based on
/// the number of audio channels in the stream. It uses common bitrate values
/// for standard channel configurations (mono, stereo, 5.1, 7.1) and falls back
/// to a formula for non-standard configurations.
///
/// # Arguments
///
/// * `channels` - The number of audio channels
///
/// # Returns
///
/// * The recommended audio bitrate in kbps (kilobits per second)
///
/// # Examples
///
/// ```
/// // This function is internal to the crate, so we can't call it directly in doctests
/// // Example usage within the crate:
/// // assert_eq!(calculate_audio_bitrate(1), 64);  // Mono
/// // assert_eq!(calculate_audio_bitrate(2), 128); // Stereo
/// // assert_eq!(calculate_audio_bitrate(6), 256); // 5.1 surround
/// ```
pub(crate) fn calculate_audio_bitrate(channels: u32) -> u32 {
    match channels {
        1 => 64,            // Mono
        2 => 128,           // Stereo
        6 => 256,           // 5.1 surround
        8 => 384,           // 7.1 surround
        _ => channels * 48, // ~48 kbps per channel for non-standard configs
    }
}


/// Analyzes and logs information about audio streams in a video file.
///
/// This function detects the number of audio channels in each stream of the
/// video file and calculates appropriate bitrates for encoding. It logs this
/// information for user feedback and debugging purposes.
///
/// The function is designed to be non-critical - if it fails to get audio
/// information, it logs a warning but doesn't prevent the encoding process
/// from continuing.
///
/// # Arguments
///
/// * `ffprobe_executor` - Implementation of `FfprobeExecutor` for analyzing the video
/// * `input_path` - Path to the input video file
///
/// # Returns
///
/// * `Ok(())` - If the analysis completes (even if no audio streams are found)
/// * `Err(CoreError)` - This function generally handles errors internally and
///   returns Ok, but may propagate critical errors from the ffprobe executor
///
/// # Examples
///
/// ```rust,no_run
/// use drapto_core::processing::audio::log_audio_info;
/// use std::path::Path;
///
/// let input_path = Path::new("/path/to/video.mkv");
///
/// log_audio_info(input_path).unwrap();
/// ```
pub fn log_audio_info(input_path: &Path) -> CoreResult<()> {
    // Extract filename for logging purposes
    let filename = input_path
        .file_name().map_or_else(|| "unknown_file".to_string(), |s| s.to_string_lossy().to_string());

    let audio_channels = match get_audio_channels(input_path) {
        Ok(channels) => channels,
        Err(e) => {
            // Audio info is non-critical - warn and continue
            crate::progress_reporting::warning(&format!(
                "Error getting audio channels for {filename}: {e}. Cannot log bitrate info."
            ));

            return Ok(());
        }
    };
    if audio_channels.is_empty() {
        crate::progress_reporting::status("Audio streams", "None detected", false);
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
    crate::progress_reporting::status("Audio", &channel_summary, false);

    let mut bitrate_parts = Vec::new();
    for (index, &num_channels) in audio_channels.iter().enumerate() {
        let bitrate = calculate_audio_bitrate(num_channels);
        if audio_channels.len() == 1 {
            crate::progress_reporting::status("Bitrate", &format!("{bitrate}kbps"), false);
        } else {
            bitrate_parts.push(format!("Stream {index}: {bitrate}kbps"));
        }
    }

    if audio_channels.len() > 1 {
        crate::progress_reporting::status("Bitrates", &bitrate_parts.join(", "), false);
    }

    Ok(())
}
