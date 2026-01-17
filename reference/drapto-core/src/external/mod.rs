//! External tool integrations for `FFmpeg` and `FFprobe`
//!
//! This module provides direct interactions with external command-line tools
//! like ffmpeg and ffprobe, as well as file system operations.

use crate::error::CoreResult;
use std::path::Path;

/// Contains ffmpeg argument building logic and encoding parameter structures
pub mod ffmpeg;

/// Contains builder utilities for FFmpeg commands
pub mod ffmpeg_builder;

/// Contains traits and implementations for executing ffprobe commands
pub mod ffprobe_executor;

/// Contains MediaInfo integration for comprehensive media analysis
pub mod mediainfo_executor;

// Re-exports for convenience
pub use ffmpeg_builder::{FfmpegCommandBuilder, SvtAv1ParamsBuilder, VideoFilterChain};
pub use ffprobe_executor::{
    AudioStreamInfo, MediaInfo, get_audio_channels, get_audio_stream_info, get_media_info,
    get_video_properties,
};
pub use mediainfo_executor::{
    HdrInfo, MediaInfoResponse, detect_hdr_from_mediainfo, get_audio_channels_from_mediainfo,
    get_media_info as get_mediainfo_data, is_mediainfo_available,
};

/// Returns file size in bytes.
pub fn get_file_size(path: &Path) -> CoreResult<u64> {
    Ok(std::fs::metadata(path)?.len())
}

/// List of FFmpeg error messages that should be treated as non-critical.
/// These messages appear in stderr but don't indicate actual problems.
pub const NON_CRITICAL_FFMPEG_MESSAGES: &[&str] = &[
    "deprecated pixel format",
    "No accelerated colorspace conversion",
    "Stream map",
    "automatically inserted filter",
    "Timestamps are unset",
    "does not match the corresponding codec",
    "Queue input is backward",
    "No streams found",
    "first frame is no keyframe",
    "Skipping NAL unit",
];

/// Checks if FFmpeg error message is non-critical.
pub fn is_non_critical_ffmpeg_message(message: &str) -> bool {
    NON_CRITICAL_FFMPEG_MESSAGES
        .iter()
        .any(|pattern| message.contains(pattern))
}
