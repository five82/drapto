//! Structure for video metadata.
//!
//! This file defines the VideoProperties structure that holds metadata about
//! video files, such as resolution, duration, and color space. This structure
//! is used throughout the codebase to represent the properties of a video file.

/// Structure containing detected video properties.
///
/// This structure holds metadata about a video file, including its resolution,
/// duration, and color space. It is populated by the `FfprobeExecutor` trait
/// implementations and used throughout the codebase for encoding decisions.
///
/// # Fields
///
/// * `width` - Width of the video in pixels
/// * `height` - Height of the video in pixels
/// * `duration_secs` - Duration of the video in seconds
/// * `color_space` - Color space of the video (e.g., "bt709", "bt2020nc")
///
/// # Examples
///
/// ```rust
/// use drapto_core::processing::video_properties::VideoProperties;
///
/// // Create a new VideoProperties instance
/// let props = VideoProperties {
///     width: 1920,
///     height: 1080,
///     duration_secs: 3600.0, // 1 hour
///     color_space: Some("bt709".to_string()),
/// };
///
/// // Use the properties for encoding decisions
/// let is_hd = props.width >= 1920;
/// let is_hdr = props.color_space.as_deref() == Some("bt2020nc");
/// ```
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
