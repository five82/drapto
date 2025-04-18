// drapto-core/src/processing/audio.rs

use crate::error::CoreResult; // Removed CoreError
use crate::external::get_audio_channels; // Assuming this stays in external
use std::path::Path; // Removed PathBuf

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
pub fn log_audio_info<F>(
    input_path: &Path,
    mut log_callback: F, // Accept by mutable reference
) -> CoreResult<()>
where
    F: FnMut(&str),
{
    let filename = input_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown_file".to_string()); // Fallback filename for logging

    // --- Get Audio Info ---
    let audio_channels = match get_audio_channels(input_path) {
        Ok(channels) => {
            log_callback(&format!("Detected audio channels: {:?}", channels));
            channels
        }
        Err(e) => {
            // Log warning and return Ok, as this function is just for logging.
            // The ffmpeg builder will handle missing channel info separately.
            log_callback(&format!(
                "Warning: Error getting audio channels for {}: {}. Cannot log bitrate info.",
                filename, e
            ));
            return Ok(()); // Return Ok, logging is best-effort
        }
    };

    // --- Log Calculated Bitrates ---
    if audio_channels.is_empty() {
        log_callback("No audio channels detected; cannot calculate specific bitrates.");
        return Ok(());
    }

    let mut audio_bitrate_log_parts = Vec::new();
    for (index, &num_channels) in audio_channels.iter().enumerate() {
        let bitrate = calculate_audio_bitrate(num_channels); // Use local helper
        let log_msg = format!(
            "Calculated bitrate for audio stream {} ({} channels): {}kbps",
            index, num_channels, bitrate
        );
        log_callback(&log_msg);
        audio_bitrate_log_parts.push(format!("Stream {}: {}kbps", index, bitrate));
    }
    log_callback(&format!("  Bitrate Breakdown: {}", audio_bitrate_log_parts.join(", ")));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    // PathBuf is needed for tests, but not the main code. Import it locally.
    use std::path::PathBuf;

    // Helper to create a dummy file
    fn create_dummy_file(dir: &tempfile::TempDir, name: &str) -> PathBuf {
        let file_path = dir.path().join(name);
        fs::write(&file_path, "dummy content").unwrap();
        file_path
    }

    // Mock log callback
    fn mock_log_callback(msg: &str) {
        // In a real test, you might collect messages in a Vec<&str>
        // For this example, we just print.
        println!("LOG: {}", msg);
    }

    #[test]
    fn test_calculate_audio_bitrate() {
        assert_eq!(calculate_audio_bitrate(1), 64);
        assert_eq!(calculate_audio_bitrate(2), 128);
        assert_eq!(calculate_audio_bitrate(6), 256);
        assert_eq!(calculate_audio_bitrate(8), 384);
        assert_eq!(calculate_audio_bitrate(3), 144); // 3 * 48
        assert_eq!(calculate_audio_bitrate(0), 0);   // 0 * 48
    }

    // Note: Testing log_audio_info fully requires mocking `get_audio_channels`.
    // This is a basic test structure demonstrating the function signature and basic logic.
    // A more robust test suite would use a mocking library or dependency injection
    // to control the behavior of `get_audio_channels`.
    #[test]
    fn test_log_audio_info_structure_example() { // Renamed test
        // This test primarily checks if the function compiles and runs without panicking.
        // It doesn't verify the output correctness without mocking get_audio_channels.
        let tmp_dir = tempdir().unwrap();
        let dummy_file = create_dummy_file(&tmp_dir, "test.mkv");

        // We expect this to likely hit the error path in `get_audio_channels`
        // if ffprobe isn't available or the dummy file isn't a valid video.
        let result = log_audio_info(&dummy_file, mock_log_callback); // Call renamed function

        // Check that it returns Ok, even if it logged a warning due to channel detection error
        assert!(result.is_ok());
        // No args vector to check anymore
    }
}