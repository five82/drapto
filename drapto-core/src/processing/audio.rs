// ============================================================================
// drapto-core/src/processing/audio.rs
// ============================================================================
//
// AUDIO PROCESSING: Audio Stream Analysis and Bitrate Calculation
//
// This module handles the analysis of audio streams in video files, including
// detecting the number of channels and calculating appropriate bitrates for
// encoding. It provides functions for logging audio information and determining
// optimal encoding parameters based on the audio characteristics.
//
// KEY COMPONENTS:
// - Audio channel detection using ffprobe
// - Bitrate calculation based on channel count
// - Logging of audio stream information
//
// AI-ASSISTANT-INFO: Audio stream analysis and bitrate calculation

// ---- External crate imports ----
use colored::*;
use log::{info, warn};

// ---- Internal crate imports ----
use crate::error::CoreResult;
use crate::external::FfprobeExecutor;

// ---- Standard library imports ----
use std::path::Path;

// ============================================================================
// BITRATE CALCULATION
// ============================================================================

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
/// use drapto_core::processing::audio::calculate_audio_bitrate;
///
/// assert_eq!(calculate_audio_bitrate(1), 64);  // Mono
/// assert_eq!(calculate_audio_bitrate(2), 128); // Stereo
/// assert_eq!(calculate_audio_bitrate(6), 256); // 5.1 surround
/// ```
pub(crate) fn calculate_audio_bitrate(channels: u32) -> u32 {
    match channels {
        1 => 64,   // Mono: 64 kbps is sufficient for voice/simple audio
        2 => 128,  // Stereo: 128 kbps provides good quality for most content
        6 => 256,  // 5.1 surround: 256 kbps balances quality and size
        8 => 384,  // 7.1 surround: 384 kbps for high-quality surround
        _ => channels * 48, // For non-standard configurations: ~48 kbps per channel
    }
}

// ============================================================================
// AUDIO INFORMATION LOGGING
// ============================================================================

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
/// * `ffprobe_executor` - Implementation of FfprobeExecutor for analyzing the video
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
/// use drapto_core::external::CrateFfprobeExecutor;
/// use std::path::Path;
///
/// let ffprobe_executor = CrateFfprobeExecutor::new();
/// let input_path = Path::new("/path/to/video.mkv");
///
/// log_audio_info(&ffprobe_executor, input_path).unwrap();
/// ```
pub fn log_audio_info<P: FfprobeExecutor>(
    ffprobe_executor: &P,
    input_path: &Path,
) -> CoreResult<()> {
    // Extract filename for logging purposes
    let filename = input_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown_file".to_string());

    // STEP 1: Get audio channel information using ffprobe
    let audio_channels = match ffprobe_executor.get_audio_channels(input_path) {
        Ok(channels) => {
            info!("Detected audio channels: {}", format!("{:?}", channels).green());
            channels
        }
        Err(e) => {
            // Log warning but don't fail the process - audio info is non-critical
            // The ffmpeg builder will handle missing channel info separately
            warn!(
                "Error getting audio channels for {}: {}. Cannot log bitrate info.",
                filename, e
            );
            return Ok(());
        }
    };

    // STEP 2: Log calculated bitrates for each audio stream
    if audio_channels.is_empty() {
        info!("No audio channels detected; cannot calculate specific bitrates.");
        return Ok(());
    }

    // Calculate and log bitrate for each audio stream
    let mut audio_bitrate_log_parts = Vec::new();
    for (index, &num_channels) in audio_channels.iter().enumerate() {
        // Calculate appropriate bitrate based on channel count
        let bitrate = calculate_audio_bitrate(num_channels);

        // Log detailed information for each stream
        let log_msg = format!(
            "Calculated bitrate for audio stream {} ({} channels): {}",
            index,
            num_channels.to_string().green(),
            format!("{}kbps", bitrate).green().bold()
        );
        info!("{}", log_msg);

        // Add to summary for combined log message
        audio_bitrate_log_parts.push(format!(
            "Stream {}: {}",
            index,
            format!("{}kbps", bitrate).green().bold()
        ));
    }

    // Log summary of all streams
    info!("  Bitrate Breakdown: {}", audio_bitrate_log_parts.join(", "));

    Ok(())
}
