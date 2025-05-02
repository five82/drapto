// drapto-core/src/processing/audio.rs

use crate::error::CoreResult;
use crate::external::FfprobeExecutor; // Import trait
use std::path::Path;

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
pub fn log_audio_info<P: FfprobeExecutor, F>( // Add generic
    ffprobe_executor: &P, // Add executor argument
    input_path: &Path,
    mut log_callback: F,
) -> CoreResult<()>
where
    F: FnMut(&str),
{
    let filename = input_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown_file".to_string()); // Fallback filename for logging

    // --- Get Audio Info ---
    let audio_channels = match ffprobe_executor.get_audio_channels(input_path) { // Use executor
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
    use crate::error::CoreError; // Import CoreError
    use crate::external::mocks::MockFfprobeExecutor; // Import mock
    use std::fs;
    use std::sync::{Arc, Mutex}; // For capturing logs
    use tempfile::{tempdir, TempDir}; // Import tempdir function and TempDir struct
    use std::path::PathBuf; // Only import PathBuf

    // Helper to create a dummy file
    fn create_dummy_file(dir: &TempDir, name: &str) -> PathBuf { // Accept TempDir
        let file_path = dir.path().join(name); // Call path() on TempDir
        fs::write(&file_path, "dummy content").unwrap();
        file_path
    }

    // Note: Removed old mock_log_callback, using Arc<Mutex<Vec>> instead

    #[test]
    fn test_calculate_audio_bitrate() {
        assert_eq!(calculate_audio_bitrate(1), 64);
        assert_eq!(calculate_audio_bitrate(2), 128);
        assert_eq!(calculate_audio_bitrate(6), 256);
        assert_eq!(calculate_audio_bitrate(8), 384);
        assert_eq!(calculate_audio_bitrate(3), 144); // 3 * 48
        assert_eq!(calculate_audio_bitrate(0), 0);   // 0 * 48
    }

    #[test]
    fn test_log_audio_info_with_mock() { // Renamed test
        let tmp_dir = tempdir().unwrap();
        let dummy_file = create_dummy_file(&tmp_dir, "test.mkv"); // Pass &tmp_dir

        // --- Mock Ffprobe Setup ---
        let mock_ffprobe = MockFfprobeExecutor::new();
        // Simulate stereo and 5.1 tracks
        mock_ffprobe.expect_audio_channels(&dummy_file, Ok(vec![2, 6]));

        // --- Log Capture Setup ---
        let log_messages = Arc::new(Mutex::new(Vec::new()));
        let log_messages_clone = log_messages.clone();
        let log_callback = move |msg: &str| {
            log_messages_clone.lock().unwrap().push(msg.to_string());
        };

        // --- Execute ---
        let result = log_audio_info(&mock_ffprobe, &dummy_file, log_callback);

        // --- Assertions ---
        assert!(result.is_ok(), "log_audio_info should succeed");

        let logs = log_messages.lock().unwrap();
        // Check for specific log messages based on mock data
        assert!(logs.iter().any(|m| m == "Detected audio channels: [2, 6]"));
        assert!(logs.iter().any(|m| m == "Calculated bitrate for audio stream 0 (2 channels): 128kbps"));
        assert!(logs.iter().any(|m| m == "Calculated bitrate for audio stream 1 (6 channels): 256kbps"));
        assert!(logs.iter().any(|m| m == "  Bitrate Breakdown: Stream 0: 128kbps, Stream 1: 256kbps"));
    }

    #[test]
    fn test_log_audio_info_ffprobe_error() {
        let tmp_dir = tempdir().unwrap();
        let dummy_file = create_dummy_file(&tmp_dir, "test_err.mkv"); // Pass &tmp_dir

        // --- Mock Ffprobe Setup (Error) ---
        let mock_ffprobe = MockFfprobeExecutor::new();
        let ffprobe_error = CoreError::FfprobeParse("Simulated ffprobe error".to_string());
        mock_ffprobe.expect_audio_channels(&dummy_file, Err(ffprobe_error));

        // --- Log Capture Setup ---
        let log_messages = Arc::new(Mutex::new(Vec::new()));
        let log_messages_clone = log_messages.clone();
        let log_callback = move |msg: &str| {
            log_messages_clone.lock().unwrap().push(msg.to_string());
        };

        // --- Execute ---
        let result = log_audio_info(&mock_ffprobe, &dummy_file, log_callback);

        // --- Assertions ---
        // Should still return Ok, but log a warning
        assert!(result.is_ok(), "log_audio_info should return Ok even on ffprobe error");

        let logs = log_messages.lock().unwrap();
        assert!(logs.iter().any(|m| m.contains("Warning: Error getting audio channels for test_err.mkv")));
        assert!(logs.iter().any(|m| m.contains("Simulated ffprobe error")));
        // Ensure no bitrate logs were generated
        assert!(!logs.iter().any(|m| m.contains("Calculated bitrate")));
        assert!(!logs.iter().any(|m| m.contains("Bitrate Breakdown")));
    }
}