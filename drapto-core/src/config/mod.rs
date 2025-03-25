//! Configuration module
//!
//! Responsibilities:
//! - Define configuration structures for all system components
//! - Parse configuration files (TOML format)
//! - Apply environment variable overrides
//! - Validate configuration values
//! - Provide centralized configuration management
//!
//! This module provides a unified approach to configuration handling
//! throughout the application, with file parsing and validation.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{DraptoError, Result};

// Module imports
mod directory;
mod encoding;
mod validation;
mod resource;
mod logging;
mod detection;
mod utils;

// Public exports
pub use directory::DirectoryConfig;
pub use encoding::{VideoEncodingConfig, AudioEncodingConfig, AudioBitrates};
pub use validation::{ValidationConfig, AudioValidationConfig, VideoValidationConfig};
pub use resource::ResourceConfig;
pub use logging::LoggingConfig;
pub use detection::{SceneDetectionConfig, CropDetectionConfig};
pub use utils::*;

/// Main configuration for drapto
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Input file path
    pub input: PathBuf,

    /// Output file path
    pub output: PathBuf,

    /// Working directory settings
    #[serde(default)]
    pub directories: DirectoryConfig,

    /// Video encoding settings
    #[serde(default)]
    pub video: VideoEncodingConfig,

    /// Audio encoding settings
    #[serde(default)]
    pub audio: AudioEncodingConfig,

    /// Scene detection settings
    #[serde(default)]
    pub scene_detection: SceneDetectionConfig,

    /// Crop detection settings
    #[serde(default)]
    pub crop_detection: CropDetectionConfig,

    /// Validation settings
    #[serde(default)]
    pub validation: ValidationConfig,

    /// Resource management settings
    #[serde(default)]
    pub resources: ResourceConfig,

    /// Logging settings
    #[serde(default)]
    pub logging: LoggingConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input: PathBuf::new(),
            output: PathBuf::new(),
            directories: DirectoryConfig::default(),
            video: VideoEncodingConfig::default(),
            audio: AudioEncodingConfig::default(),
            scene_detection: SceneDetectionConfig::default(),
            crop_detection: CropDetectionConfig::default(),
            validation: ValidationConfig::default(),
            resources: ResourceConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Config {
    /// Create a new default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| DraptoError::Config(format!("Failed to read config file: {}", e)))?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| DraptoError::Config(format!("Failed to parse config file: {}", e)))?;

        // Apply environment variable overrides
        // Default impl already handles env vars, so no need to override here

        Ok(config)
    }

    /// Set the input file path
    pub fn with_input<P: Into<PathBuf>>(mut self, input: P) -> Self {
        self.input = input.into();
        self
    }

    /// Set the output file path
    pub fn with_output<P: Into<PathBuf>>(mut self, output: P) -> Self {
        self.output = output.into();
        self
    }

    /// Validate configuration parameters
    pub fn validate(&self) -> Result<()> {
        if !self.input.exists() {
            return Err(DraptoError::Config(format!(
                "Input file not found: {:?}",
                self.input
            )));
        }

        // Validate VMAF values
        if !(0.0..=100.0).contains(&self.video.target_vmaf) {
            return Err(DraptoError::Config(format!(
                "Target VMAF must be between 0 and 100, got {}",
                self.video.target_vmaf
            )));
        }

        if !(0.0..=100.0).contains(&self.video.target_vmaf_hdr) {
            return Err(DraptoError::Config(format!(
                "Target VMAF HDR must be between 0 and 100, got {}",
                self.video.target_vmaf_hdr
            )));
        }

        if self.resources.parallel_jobs == 0 {
            return Err(DraptoError::Config(
                "Parallel jobs must be at least 1".to_string(),
            ));
        }

        // Ensure the output directory exists or can be created
        if let Some(parent) = self.output.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    DraptoError::Config(format!("Failed to create output directory: {}", e))
                })?;
            }
        }

        // Create necessary directories
        for dir in [
            &self.directories.temp_dir,
            self.directories
                .segments_dir
                .as_ref()
                .unwrap_or(&self.directories.temp_dir.join("segments")),
            self.directories
                .encoded_segments_dir
                .as_ref()
                .unwrap_or(&self.directories.temp_dir.join("encoded_segments")),
            self.directories
                .working_dir
                .as_ref()
                .unwrap_or(&self.directories.temp_dir.join("working")),
            &self.logging.log_dir,
        ] {
            std::fs::create_dir_all(dir).map_err(|e| {
                DraptoError::Config(format!(
                    "Failed to create directory {}: {}",
                    dir.display(),
                    e
                ))
            })?;
        }

        Ok(())
    }

    /// Save configuration to a TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| DraptoError::Config(format!("Failed to serialize config: {}", e)))?;

        fs::write(path, content)
            .map_err(|e| DraptoError::Config(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }
}