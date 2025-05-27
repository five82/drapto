//! FFmpeg command builder utilities
//!
//! This module provides a builder pattern for constructing FFmpeg commands
//! with common options and configurations used throughout the application.
//!
//! IMPORTANT: Hardware acceleration in this module refers ONLY to hardware
//! DECODING. We exclusively use software encoding (libsvtav1) for output.

use crate::hardware_decode::add_hardware_decoding_to_command;
use ffmpeg_sidecar::command::FfmpegCommand;

/// Builder for creating `FFmpeg` commands with common configurations
pub struct FfmpegCommandBuilder {
    cmd: FfmpegCommand,
    use_hw_decode: bool,
    hide_banner: bool,
}

impl Default for FfmpegCommandBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FfmpegCommandBuilder {
    /// Creates a new `FFmpeg` command builder with sensible defaults
    #[must_use] pub fn new() -> Self {
        Self {
            cmd: FfmpegCommand::new(),
            use_hw_decode: false,
            hide_banner: true,
        }
    }

    /// Enables hardware decoding (`VideoToolbox` on macOS)
    #[must_use] pub fn with_hardware_accel(mut self, enabled: bool) -> Self {
        self.use_hw_decode = enabled;
        self
    }

    /// Sets whether to hide the `FFmpeg` banner
    #[must_use] pub fn with_hide_banner(mut self, hide: bool) -> Self {
        self.hide_banner = hide;
        self
    }

    /// Builds the `FFmpeg` command with all configured options
    #[must_use] pub fn build(mut self) -> FfmpegCommand {
        if self.hide_banner {
            self.cmd.arg("-hide_banner");
        }

        if self.use_hw_decode {
            add_hardware_decoding_to_command(&mut self.cmd, true, false);
        }

        self.cmd
    }
}

/// Builder for constructing video filter chains
#[derive(Default)]
pub struct VideoFilterChain {
    filters: Vec<String>,
}

impl VideoFilterChain {
    /// Creates a new empty filter chain
    #[must_use] pub fn new() -> Self {
        Self::default()
    }

    /// Adds a denoising filter to the chain
    #[must_use] pub fn add_denoise(mut self, params: &str) -> Self {
        if !params.is_empty() {
            if params.starts_with("hqdn3d=") {
                self.filters.push(params.to_string());
            } else {
                self.filters.push(format!("hqdn3d={params}"));
            }
        }
        self
    }

    /// Adds a crop filter to the chain
    #[must_use] pub fn add_crop(mut self, crop: &str) -> Self {
        if !crop.is_empty() {
            self.filters.push(crop.to_string());
        }
        self
    }

    /// Adds a custom filter to the chain
    #[must_use] pub fn add_filter(mut self, filter: String) -> Self {
        if !filter.is_empty() {
            self.filters.push(filter);
        }
        self
    }

    /// Builds the filter chain into a single filter string
    #[must_use] pub fn build(self) -> Option<String> {
        if self.filters.is_empty() {
            None
        } else {
            Some(self.filters.join(","))
        }
    }
}

/// Helper to build SVT-AV1 parameters
pub struct SvtAv1ParamsBuilder {
    params: Vec<(String, String)>,
}

impl SvtAv1ParamsBuilder {
    /// Creates a new SVT-AV1 parameters builder
    #[must_use] pub fn new() -> Self {
        Self {
            params: vec![("tune".to_string(), "3".to_string())],
        }
    }

    /// Sets the film grain synthesis level
    #[must_use] pub fn with_film_grain(mut self, level: u8) -> Self {
        if level > 0 {
            self.params
                .push(("film-grain".to_string(), level.to_string()));
            self.params
                .push(("film-grain-denoise".to_string(), "0".to_string()));
        }
        self
    }

    /// Adds a custom parameter
    #[must_use] pub fn add_param(mut self, key: &str, value: &str) -> Self {
        self.params.push((key.to_string(), value.to_string()));
        self
    }

    /// Builds the parameters into a colon-separated string
    #[must_use] pub fn build(self) -> String {
        self.params
            .into_iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(":")
    }
}

impl Default for SvtAv1ParamsBuilder {
    fn default() -> Self {
        Self::new()
    }
}
