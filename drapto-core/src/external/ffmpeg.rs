// drapto-core/src/external/ffmpeg.rs
//
// This module will encapsulate the logic for building and potentially
// executing ffmpeg commands.

use crate::error::CoreResult;
use crate::processing::audio; // To access calculate_audio_bitrate (needs to be pub(crate))
use std::path::PathBuf;

/// Represents the type of hardware acceleration to use for decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareAccel { // Changed to pub
    None,
    Vaapi,        // Linux AMD/Intel
    VideoToolbox, // macOS
}

// Struct containing necessary info for building ffmpeg args
pub(crate) struct FfmpegCommandArgs {
    pub input_path: PathBuf,
    pub hw_accel: HardwareAccel, // Added hardware acceleration type
    pub output_path: PathBuf,
    pub quality: u32, // CRF value
    pub preset: u8,   // SVT-AV1 preset
    pub crop_filter: Option<String>, // Optional crop filter string "crop=..."
    pub audio_channels: Vec<u32>, // Detected audio channels for bitrate mapping
}

/// Builds the ffmpeg command arguments as a Vec<String>.
pub(crate) fn build_ffmpeg_args(
    cmd_args: &FfmpegCommandArgs,
) -> CoreResult<Vec<String>> {
    let mut ffmpeg_args: Vec<String> = Vec::new();

    ffmpeg_args.push("-hide_banner".to_string());

    // Hardware Acceleration (Input Option - must come before -i)
    match cmd_args.hw_accel {
        HardwareAccel::Vaapi => {
            ffmpeg_args.extend(vec![
                "-hwaccel".to_string(),
                "vaapi".to_string(),
                "-hwaccel_output_format".to_string(),
                "vaapi".to_string(), // VAAPI often requires specifying the output format
            ]);
        }
        HardwareAccel::VideoToolbox => {
            ffmpeg_args.extend(vec!["-hwaccel".to_string(), "videotoolbox".to_string()]);
        }
        HardwareAccel::None => {
            // No hwaccel args needed
        }
    }

    // Input
    ffmpeg_args.extend(vec!["-i".to_string(), cmd_args.input_path.to_string_lossy().to_string()]);

    // Video Filters (Crop)
    // Combine all video filters with comma if multiple exist in the future
    let mut vf_filters: Vec<String> = Vec::new();
    if let Some(crop) = &cmd_args.crop_filter {
        vf_filters.push(crop.clone());
    }
    // Add other video filters here if needed
    if !vf_filters.is_empty() {
        ffmpeg_args.extend(vec!["-vf".to_string(), vf_filters.join(",")]);
    }

    // Stream Mapping
    ffmpeg_args.extend(vec![
        "-map".to_string(), "0:v:0?".to_string(), // Map first video stream (optional)
        "-map".to_string(), "0:a?".to_string(), // Map all audio streams (optional)
        "-map".to_string(), "0:s?".to_string(), // Map all subtitle streams (optional)
        "-map_metadata".to_string(), "0".to_string(), // Copy global metadata
        "-map_chapters".to_string(), "0".to_string(), // Copy chapters
    ]);

    // Video Codec and Params
    ffmpeg_args.extend(vec![
        "-c:v".to_string(), "libsvtav1".to_string(),
        "-pix_fmt".to_string(), "yuv420p10le".to_string(), // Ensure 10-bit
        "-crf".to_string(), cmd_args.quality.to_string(),
        "-preset".to_string(), cmd_args.preset.to_string(),
        // TODO: Verify if tune=0 is needed or default for SVT-AV1
        // "-svtav1-params".to_string(), "tune=0".to_string(),
    ]);

    // Audio Codec and Params
    ffmpeg_args.extend(vec![
        "-c:a".to_string(), "libopus".to_string(),
    ]);
    for (i, &channels) in cmd_args.audio_channels.iter().enumerate() {
        // Use the helper from the audio module (needs to be pub(crate))
        let bitrate = audio::calculate_audio_bitrate(channels);
        ffmpeg_args.push(format!("-b:a:{}", i));
        ffmpeg_args.push(format!("{}k", bitrate));
    }
    // Add the channel layout workaround filter
    // Combine audio filters if more are added later
    let af_filters = vec!["aformat=channel_layouts=7.1|5.1|stereo|mono".to_string()];
    ffmpeg_args.extend(vec!["-af".to_string(), af_filters.join(",")]);

    // Subtitle Codec (copy)
    ffmpeg_args.extend(vec![
        "-c:s".to_string(), "copy".to_string(),
    ]);

    // Progress Reporting
    ffmpeg_args.extend(vec!["-progress".to_string(), "-".to_string()]); // Report progress to stderr

    // Output Path (must be the last argument)
    ffmpeg_args.push(cmd_args.output_path.to_string_lossy().to_string());

    Ok(ffmpeg_args)
}


#[cfg(test)]
mod tests {
    use super::*; // Import items from parent module (ffmpeg.rs)
    use std::path::PathBuf;

    // Helper to create default args for tests
    fn default_test_args() -> FfmpegCommandArgs {
        FfmpegCommandArgs {
            input_path: PathBuf::from("input.mkv"),
            hw_accel: HardwareAccel::None, // Default to None, override in tests
            output_path: PathBuf::from("output.mkv"),
            quality: 25,
            preset: 6,
            crop_filter: None,
            audio_channels: vec![2], // Example: stereo
        }
    }

    #[test]
    fn test_build_ffmpeg_args_no_hwaccel() {
        let args = default_test_args(); // hw_accel is None by default
        let ffmpeg_args = build_ffmpeg_args(&args).unwrap();

        // Check that no hwaccel flags are present
        assert!(!ffmpeg_args.contains(&"-hwaccel".to_string()));
        assert!(!ffmpeg_args.contains(&"vaapi".to_string()));
        assert!(!ffmpeg_args.contains(&"-hwaccel_output_format".to_string()));
        assert!(!ffmpeg_args.contains(&"videotoolbox".to_string()));

        // Basic check that input/output are present
        assert!(ffmpeg_args.contains(&"-i".to_string()));
        assert!(ffmpeg_args.contains(&"input.mkv".to_string()));
        assert_eq!(ffmpeg_args.last().unwrap(), "output.mkv");
    }

    #[test]
    fn test_build_ffmpeg_args_vaapi() {
        let mut args = default_test_args();
        args.hw_accel = HardwareAccel::Vaapi;
        let ffmpeg_args = build_ffmpeg_args(&args).unwrap();

        // Check that VAAPI flags are present and in the correct order (before -i)
        let hwaccel_pos = ffmpeg_args.iter().position(|r| r == "-hwaccel").unwrap();
        let vaapi_pos1 = ffmpeg_args.iter().position(|r| r == "vaapi").unwrap(); // First vaapi
        let hwaccel_out_pos = ffmpeg_args.iter().position(|r| r == "-hwaccel_output_format").unwrap();
        // Find the *second* occurrence of "vaapi"
        let vaapi_pos2 = ffmpeg_args.iter().enumerate()
            .filter(|&(_, r)| r == "vaapi")
            .nth(1) // Get the second one
            .map(|(i, _)| i)
            .unwrap();
        let input_pos = ffmpeg_args.iter().position(|r| r == "-i").unwrap();


        assert_eq!(hwaccel_pos, 1, "Expected -hwaccel at index 1"); // After -hide_banner
        assert_eq!(vaapi_pos1, 2, "Expected first vaapi at index 2");
        assert_eq!(hwaccel_out_pos, 3, "Expected -hwaccel_output_format at index 3");
        assert_eq!(vaapi_pos2, 4, "Expected second vaapi at index 4");
        assert_eq!(input_pos, 5, "Expected -i at index 5"); // Ensure -i comes after hwaccel flags

        assert!(ffmpeg_args.contains(&"-hwaccel".to_string()));
        assert!(ffmpeg_args.contains(&"vaapi".to_string()));
        assert!(ffmpeg_args.contains(&"-hwaccel_output_format".to_string()));
        // Check that videotoolbox is NOT present
        assert!(!ffmpeg_args.contains(&"videotoolbox".to_string()));
    }

     #[test]
    fn test_build_ffmpeg_args_videotoolbox() {
        let mut args = default_test_args();
        args.hw_accel = HardwareAccel::VideoToolbox;
        let ffmpeg_args = build_ffmpeg_args(&args).unwrap();

        // Check that VideoToolbox flags are present and in the correct order (before -i)
        let hwaccel_pos = ffmpeg_args.iter().position(|r| r == "-hwaccel").unwrap();
        let videotoolbox_pos = ffmpeg_args.iter().position(|r| r == "videotoolbox").unwrap();
        let input_pos = ffmpeg_args.iter().position(|r| r == "-i").unwrap();

        assert_eq!(hwaccel_pos, 1, "Expected -hwaccel at index 1"); // After -hide_banner
        assert_eq!(videotoolbox_pos, 2, "Expected videotoolbox at index 2");
        assert_eq!(input_pos, 3, "Expected -i at index 3"); // Ensure -i comes after hwaccel flags

        assert!(ffmpeg_args.contains(&"-hwaccel".to_string()));
        assert!(ffmpeg_args.contains(&"videotoolbox".to_string()));
        // Check that VAAPI flags are NOT present
        assert!(!ffmpeg_args.contains(&"vaapi".to_string()));
        assert!(!ffmpeg_args.contains(&"-hwaccel_output_format".to_string()));
    }

    // Add more tests as needed, e.g., with crop filters, multiple audio tracks etc.
    #[test]
    fn test_build_ffmpeg_args_with_crop() {
        let mut args = default_test_args();
        args.crop_filter = Some("crop=1920:800:0:140".to_string());
        let ffmpeg_args = build_ffmpeg_args(&args).unwrap();

        assert!(ffmpeg_args.contains(&"-vf".to_string()));
        assert!(ffmpeg_args.contains(&"crop=1920:800:0:140".to_string()));
    }
}