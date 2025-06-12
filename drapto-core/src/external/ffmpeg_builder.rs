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
            add_hardware_decoding_to_command(&mut self.cmd, true);
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
            params: vec![],
        }
    }

    /// Sets the tune parameter
    #[must_use] pub fn with_tune(mut self, tune: u8) -> Self {
        self.params.push(("tune".to_string(), tune.to_string()));
        self
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_filter_chain_empty() {
        let chain = VideoFilterChain::new();
        assert_eq!(chain.build(), None);
    }

    #[test]
    fn test_video_filter_chain_single_filter() {
        // Test crop filter
        let chain = VideoFilterChain::new()
            .add_crop("crop=1920:800:0:140");
        assert_eq!(chain.build(), Some("crop=1920:800:0:140".to_string()));
        
        // Test denoise filter with full format
        let chain = VideoFilterChain::new()
            .add_denoise("hqdn3d=0.5:0.4:2:2");
        assert_eq!(chain.build(), Some("hqdn3d=0.5:0.4:2:2".to_string()));
        
        // Test denoise filter without prefix
        let chain = VideoFilterChain::new()
            .add_denoise("0.5:0.4:2:2");
        assert_eq!(chain.build(), Some("hqdn3d=0.5:0.4:2:2".to_string()));
        
        // Test custom filter
        let chain = VideoFilterChain::new()
            .add_filter("scale=1920:1080".to_string());
        assert_eq!(chain.build(), Some("scale=1920:1080".to_string()));
    }

    #[test]
    fn test_video_filter_chain_multiple_filters() {
        let chain = VideoFilterChain::new()
            .add_crop("crop=1920:800:0:140")
            .add_denoise("0.5:0.4:2:2")
            .add_filter("scale=1920:1080".to_string());
        
        assert_eq!(
            chain.build(),
            Some("crop=1920:800:0:140,hqdn3d=0.5:0.4:2:2,scale=1920:1080".to_string())
        );
    }

    #[test]
    fn test_video_filter_chain_empty_filters_ignored() {
        let chain = VideoFilterChain::new()
            .add_crop("")
            .add_denoise("")
            .add_filter("".to_string())
            .add_crop("crop=1920:1080:0:0");
        
        assert_eq!(chain.build(), Some("crop=1920:1080:0:0".to_string()));
    }

    #[test]
    fn test_svtav1_params_builder_default() {
        let builder = SvtAv1ParamsBuilder::new();
        assert_eq!(builder.build(), "");
    }

    #[test]
    fn test_svtav1_params_builder_with_tune() {
        let builder = SvtAv1ParamsBuilder::new()
            .with_tune(3);
        assert_eq!(builder.build(), "tune=3");
        
        let builder = SvtAv1ParamsBuilder::new()
            .with_tune(0);
        assert_eq!(builder.build(), "tune=0");
    }

    #[test]
    fn test_svtav1_params_builder_with_film_grain() {
        // Test with film grain
        let builder = SvtAv1ParamsBuilder::new()
            .with_tune(3)
            .with_film_grain(4);
        assert_eq!(builder.build(), "tune=3:film-grain=4:film-grain-denoise=0");
        
        // Test with no film grain (0)
        let builder = SvtAv1ParamsBuilder::new()
            .with_tune(3)
            .with_film_grain(0);
        assert_eq!(builder.build(), "tune=3");
        
        // Test with max film grain
        let builder = SvtAv1ParamsBuilder::new()
            .with_tune(3)
            .with_film_grain(50);
        assert_eq!(builder.build(), "tune=3:film-grain=50:film-grain-denoise=0");
    }

    #[test]
    fn test_svtav1_params_builder_custom_params() {
        let builder = SvtAv1ParamsBuilder::new()
            .with_tune(3)
            .add_param("preset", "6")
            .add_param("crf", "27")
            .with_film_grain(4);
        
        assert_eq!(
            builder.build(),
            "tune=3:preset=6:crf=27:film-grain=4:film-grain-denoise=0"
        );
    }

    #[test]
    fn test_svtav1_params_builder_order() {
        // Verify parameters maintain order
        let builder = SvtAv1ParamsBuilder::new()
            .with_tune(3)
            .add_param("a", "1")
            .add_param("b", "2")
            .add_param("c", "3");
        
        assert_eq!(builder.build(), "tune=3:a=1:b=2:c=3");
    }

    #[test]
    fn test_ffmpeg_command_builder_defaults() {
        let builder = FfmpegCommandBuilder::new();
        assert_eq!(builder.hide_banner, true);
        assert_eq!(builder.use_hw_decode, false);
    }

    #[test]
    fn test_ffmpeg_command_builder_with_options() {
        // Test with hardware acceleration
        let builder = FfmpegCommandBuilder::new()
            .with_hardware_accel(true);
        assert_eq!(builder.use_hw_decode, true);
        
        // Test without hide banner
        let builder = FfmpegCommandBuilder::new()
            .with_hide_banner(false);
        assert_eq!(builder.hide_banner, false);
        
        // Test chaining
        let builder = FfmpegCommandBuilder::new()
            .with_hardware_accel(true)
            .with_hide_banner(false);
        assert_eq!(builder.use_hw_decode, true);
        assert_eq!(builder.hide_banner, false);
    }
}
