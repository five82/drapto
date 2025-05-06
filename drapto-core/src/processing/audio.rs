// drapto-core/src/processing/audio.rs
use colored::*; // Import colored for formatting

use crate::error::CoreResult;
use crate::external::FfprobeExecutor; // Import trait
use std::path::Path;
use log::{info, warn}; // Import log macros

// Calculates audio bitrate based on channel count (private helper)
pub(crate) fn calculate_audio_bitrate(channels: u32) -> u32 {
    match channels {
        1 => 64,   // Mono
        2 => 128,  // Stereo
        6 => 256,  // 5.1
        8 => 384,  // 7.1
        _ => channels * 48, // Default fallback
    }
}

/// Logs detected audio channels and calculated bitrates.
///
/// # Arguments
///
/// * `input_path` - Path to the input video file.
/// * `log_callback` - A mutable closure for logging messages.
///
/// # Returns
///
/// A `CoreResult<()>` indicating success or failure in getting channel info.
pub fn log_audio_info<P: FfprobeExecutor>( // Remove F generic
    ffprobe_executor: &P, // Add executor argument
    input_path: &Path,
    // Removed log_callback parameter
) -> CoreResult<()>
// Removed where clause
{
    let filename = input_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown_file".to_string()); // Fallback filename for logging

    // --- Get Audio Info ---
    let audio_channels = match ffprobe_executor.get_audio_channels(input_path) { // Use executor
        Ok(channels) => {
            info!("Detected audio channels: {}", format!("{:?}", channels).green()); // Use info level
            channels
        }
        Err(e) => {
            // Log warning and return Ok, as this function is just for logging.
            // The ffmpeg builder will handle missing channel info separately.
            warn!( // Use warn level
                "Error getting audio channels for {}: {}. Cannot log bitrate info.",
                filename, e
            );
            return Ok(()); // Return Ok, logging is best-effort
        }
    };

    // --- Log Calculated Bitrates ---
    if audio_channels.is_empty() {
        info!("No audio channels detected; cannot calculate specific bitrates."); // Use info level
        return Ok(());
    }

    let mut audio_bitrate_log_parts = Vec::new();
    for (index, &num_channels) in audio_channels.iter().enumerate() {
        let bitrate = calculate_audio_bitrate(num_channels); // Use local helper
        let log_msg = format!(
            "Calculated bitrate for audio stream {} ({} channels): {}",
            index,
            num_channels.to_string().green(),
            format!("{}kbps", bitrate).green().bold()
        );
        info!("{}", log_msg); // Use info level
        audio_bitrate_log_parts.push(format!("Stream {}: {}", index, format!("{}kbps", bitrate).green().bold()));
    }
    info!("  Bitrate Breakdown: {}", audio_bitrate_log_parts.join(", ")); // Use info level, parts already colored

    Ok(())
}
