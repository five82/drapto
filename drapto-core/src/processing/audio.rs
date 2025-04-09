// drapto-core/src/processing/audio.rs

use crate::error::CoreResult; // Removed CoreError
use crate::external::get_audio_channels; // Assuming this stays in external
use std::path::Path; // Removed PathBuf

// Calculates audio bitrate based on channel count (private helper)
fn calculate_audio_bitrate(channels: u32) -> u32 {
    match channels {
        1 => 64,   // Mono
        2 => 128,  // Stereo
        6 => 256,  // 5.1
        8 => 384,  // 7.1
        _ => channels * 48, // Default fallback
    }
}

/// Prepares the HandBrakeCLI audio arguments based on detected channels.
///
/// # Arguments
///
/// * `input_path` - Path to the input video file.
/// * `log_callback` - A mutable closure for logging messages.
///
/// # Returns
///
/// A `CoreResult` containing a `Vec<String>` of HandBrakeCLI audio arguments
/// (e.g., ["--aencoder", "opus", "--ab", "128,256", ...]) on success,
/// or a `CoreError` on failure (though errors in getting channels result in default args).
pub fn prepare_audio_options<F>(
    input_path: &Path,
    mut log_callback: F, // Accept by mutable reference
) -> CoreResult<Vec<String>>
where
    F: FnMut(&str),
{
    let filename = input_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown_file".to_string()); // Fallback filename for logging

    let mut audio_args = Vec::new();

    // --- Get Audio Info ---
    let audio_channels = match get_audio_channels(input_path) {
        Ok(channels) => {
            log_callback(&format!("Detected audio channels: {:?}", channels));
            channels
        }
        Err(e) => {
            // Log warning but continue without specific bitrates if ffprobe fails
            log_callback(&format!(
                "Warning: Error getting audio channels for {}: {}. Skipping audio bitrate options.",
                filename, e
            ));
            // Return default audio args without specific bitrates
            audio_args.push("--aencoder".to_string());
            audio_args.push("opus".to_string());
            audio_args.push("--all-audio".to_string());
            audio_args.push("--mixdown".to_string());
            audio_args.push("none".to_string());
            return Ok(audio_args);
        }
    };

    // --- Build Dynamic Audio Bitrate Options ---
    let mut audio_bitrates = Vec::new();
    let mut audio_bitrate_log_parts = Vec::new(); // For logging individual bitrates

    for (index, &num_channels) in audio_channels.iter().enumerate() {
        let bitrate = calculate_audio_bitrate(num_channels); // Use local helper
        audio_bitrates.push(bitrate.to_string());
        let log_msg = format!(
            "Calculated bitrate for audio stream {} ({} channels): {}kbps",
            index, num_channels, bitrate
        );
        log_callback(&log_msg);
        audio_bitrate_log_parts.push(format!("Stream {}: {}kbps", index, bitrate)); // Store for summary log
    }

    // --- Add Static and Dynamic Audio Args ---
    audio_args.push("--aencoder".to_string());
    audio_args.push("opus".to_string());
    audio_args.push("--all-audio".to_string());
    audio_args.push("--mixdown".to_string());
    audio_args.push("none".to_string());

    if !audio_bitrates.is_empty() {
        let bitrate_string = audio_bitrates.join(",");
        audio_args.push("--ab".to_string());
        audio_args.push(bitrate_string.clone()); // Add the comma-separated string
        log_callback(&format!("Final audio bitrate option: --ab {}", bitrate_string));
        log_callback(&format!("  Breakdown: {}", audio_bitrate_log_parts.join(", "))); // Log the breakdown
    } else {
        // This case might happen if get_audio_channels returns Ok(vec![])
        log_callback("No audio channels detected or error occurred; using default audio settings without specific bitrates.");
    }

    Ok(audio_args)
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

    // Note: Testing prepare_audio_options fully requires mocking `get_audio_channels`.
    // This is a basic test structure demonstrating the function signature and basic logic.
    // A more robust test suite would use a mocking library or dependency injection
    // to control the behavior of `get_audio_channels`.
    #[test]
    fn test_prepare_audio_options_structure_example() {
        // This test primarily checks if the function compiles and runs without panicking.
        // It doesn't verify the output correctness without mocking get_audio_channels.
        let tmp_dir = tempdir().unwrap();
        let dummy_file = create_dummy_file(&tmp_dir, "test.mkv");

        // We expect this to likely hit the error path in `get_audio_channels`
        // if ffprobe isn't available or the dummy file isn't a valid video.
        let result = prepare_audio_options(&dummy_file, mock_log_callback);

        // Check that it returns Ok, even if it defaulted due to channel detection error
        assert!(result.is_ok());
        let args = result.unwrap();

        // Basic check for expected default args
        assert!(args.contains(&"--aencoder".to_string()));
        assert!(args.contains(&"opus".to_string()));
        assert!(args.contains(&"--all-audio".to_string()));
        assert!(args.contains(&"--mixdown".to_string()));
        assert!(args.contains(&"none".to_string()));

        // Depending on the mock/real result of get_audio_channels,
        // --ab might or might not be present.
        // If get_audio_channels failed or returned empty, --ab should NOT be present.
        // If get_audio_channels succeeded with channels, --ab SHOULD be present.
        // println!("Generated args: {:?}", args); // For debugging test runs
    }
}