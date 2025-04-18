// drapto-core/src/external/ffmpeg.rs
//
// This module will encapsulate the logic for building and potentially
// executing ffmpeg commands.

use crate::error::CoreResult;
use crate::processing::audio; // To access calculate_audio_bitrate (needs to be pub(crate))
use std::path::PathBuf;

// Struct containing necessary info for building ffmpeg args
pub(crate) struct FfmpegCommandArgs {
    pub input_path: PathBuf,
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