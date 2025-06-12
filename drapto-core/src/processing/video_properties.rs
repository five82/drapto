//! Structure for video metadata.
//!
//! This file defines the VideoProperties structure that holds metadata about
//! video files, such as resolution, duration, and color space. This structure
//! is used throughout the codebase to represent the properties of a video file.

/// Video metadata including resolution, duration, and color space.
#[derive(Debug, Clone, Default)]
pub struct VideoProperties {
    /// Width of the video in pixels
    pub width: u32,

    /// Height of the video in pixels
    pub height: u32,

    /// Duration of the video in seconds
    pub duration_secs: f64,

    /// Color space of the video (e.g., "bt709", "bt2020nc")
    pub color_space: Option<String>,
}
