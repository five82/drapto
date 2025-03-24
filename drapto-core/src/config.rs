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
    
    /// Resource management settings
    #[serde(default)]
    pub resources: ResourceConfig,
    
    /// Logging settings
    #[serde(default)]
    pub logging: LoggingConfig,
}

/// Directory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryConfig {
    /// Temporary directory for intermediate files
    pub temp_dir: PathBuf,
    
    /// Keep temporary files after encoding
    pub keep_temp_files: bool,
    
    /// Directory for segmented files
    pub segments_dir: Option<PathBuf>,
    
    /// Directory for encoded segments
    pub encoded_segments_dir: Option<PathBuf>,
    
    /// Working directory for temporary processing
    pub working_dir: Option<PathBuf>,
}

/// Video encoding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoEncodingConfig {
    /// Enable hardware acceleration for decoding if available
    pub hardware_acceleration: bool,
    
    /// Hardware acceleration options for FFmpeg
    pub hw_accel_option: String,
    
    /// Target quality (0-100) for VMAF
    pub target_quality: Option<f32>,
    
    /// Target quality for HDR content
    pub target_quality_hdr: Option<f32>,
    
    /// Encoder preset (0-13, lower = slower/better quality)
    pub preset: u8,
    
    /// SVT-AV1 encoder parameters
    pub svt_params: String,
    
    /// Pixel format for encoding
    pub pix_fmt: String,
    
    /// Disable automatic crop detection
    pub disable_crop: bool,
    
    /// Use scene-based segmentation and parallel encoding
    pub use_segmentation: bool,
    
    /// VMAF sampling count
    pub vmaf_sample_count: u8,
    
    /// VMAF sample length in seconds
    pub vmaf_sample_length: f32,
}

/// Audio encoding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioEncodingConfig {
    /// Audio codec to use
    pub codec: String,
    
    /// Audio bitrate in kbps
    pub bitrate: Option<u32>,
    
    /// Enable normalization
    pub normalize: bool,
    
    /// Target loudness level in LUFS
    pub target_loudness: f32,
}

/// Scene detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDetectionConfig {
    /// Scene detection threshold for SDR content (0-100)
    pub scene_threshold: f32,
    
    /// Scene detection threshold for HDR content (0-100)
    pub hdr_scene_threshold: f32,
    
    /// Minimum segment length in seconds
    pub min_segment_length: f32,
    
    /// Maximum segment length in seconds
    pub max_segment_length: f32,
}

/// Resource management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    /// Number of parallel encoding jobs
    pub parallel_jobs: usize,
    
    /// Memory threshold as a fraction of total system memory
    pub memory_threshold: f32,
    
    /// Maximum memory tokens for concurrent operations
    pub max_memory_tokens: usize,
    
    /// Task stagger delay in seconds
    pub task_stagger_delay: f32,
    
    /// Memory limit per encoding job in MB (0 = auto)
    pub memory_per_job: usize,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Enable verbose logging
    pub verbose: bool,
    
    /// Log level (DEBUG, INFO, WARNING, ERROR)
    pub log_level: String,
    
    /// Log directory
    pub log_dir: PathBuf,
}

// Default implementations

impl Default for DirectoryConfig {
    fn default() -> Self {
        let temp_dir = std::env::temp_dir().join("drapto");
        Self {
            temp_dir: get_env_path("DRAPTO_WORKDIR", temp_dir.clone()),
            keep_temp_files: false,
            segments_dir: Some(get_env_path("DRAPTO_SEGMENTS_DIR", temp_dir.join("segments"))),
            encoded_segments_dir: Some(get_env_path("DRAPTO_ENCODED_SEGMENTS_DIR", temp_dir.join("encoded_segments"))),
            working_dir: Some(get_env_path("DRAPTO_WORKING_DIR", temp_dir.join("working"))),
        }
    }
}

impl Default for VideoEncodingConfig {
    fn default() -> Self {
        Self {
            hardware_acceleration: true,
            hw_accel_option: get_env_string("DRAPTO_HW_ACCEL_OPTION", String::new()),
            target_quality: Some(get_env_f32("DRAPTO_TARGET_VMAF", 93.0)),
            target_quality_hdr: Some(get_env_f32("DRAPTO_TARGET_VMAF_HDR", 95.0)),
            preset: get_env_u8("DRAPTO_PRESET", 6),
            svt_params: get_env_string("DRAPTO_SVT_PARAMS", "tune=0:film-grain=0:film-grain-denoise=0".to_string()),
            pix_fmt: get_env_string("DRAPTO_PIX_FMT", "yuv420p10le".to_string()),
            disable_crop: get_env_bool("DRAPTO_DISABLE_CROP", false),
            use_segmentation: true,
            vmaf_sample_count: get_env_u8("DRAPTO_VMAF_SAMPLE_COUNT", 3),
            vmaf_sample_length: get_env_f32("DRAPTO_VMAF_SAMPLE_LENGTH", 1.0),
        }
    }
}

impl Default for AudioEncodingConfig {
    fn default() -> Self {
        Self {
            codec: get_env_string("DRAPTO_AUDIO_CODEC", "aac".to_string()),
            bitrate: Some(get_env_u32("DRAPTO_AUDIO_BITRATE", 128)),
            normalize: get_env_bool("DRAPTO_AUDIO_NORMALIZE", true),
            target_loudness: get_env_f32("DRAPTO_TARGET_LOUDNESS", -23.0),
        }
    }
}

impl Default for SceneDetectionConfig {
    fn default() -> Self {
        Self {
            scene_threshold: get_env_f32("DRAPTO_SCENE_THRESHOLD", 40.0),
            hdr_scene_threshold: get_env_f32("DRAPTO_HDR_SCENE_THRESHOLD", 30.0),
            min_segment_length: get_env_f32("DRAPTO_MIN_SEGMENT_LENGTH", 5.0),
            max_segment_length: get_env_f32("DRAPTO_MAX_SEGMENT_LENGTH", 15.0),
        }
    }
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            parallel_jobs: get_env_usize("DRAPTO_PARALLEL_JOBS", num_cpus::get()),
            memory_threshold: get_env_f32("DRAPTO_MEMORY_THRESHOLD", 0.7),
            max_memory_tokens: get_env_usize("DRAPTO_MAX_MEMORY_TOKENS", 8),
            task_stagger_delay: get_env_f32("DRAPTO_TASK_STAGGER_DELAY", 0.2),
            memory_per_job: get_env_usize("DRAPTO_MEMORY_PER_JOB", 2048),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            verbose: get_env_bool("DRAPTO_VERBOSE", false),
            log_level: get_env_string("DRAPTO_LOG_LEVEL", "INFO".to_string()),
            log_dir: get_env_path("DRAPTO_LOG_DIR", home.join("drapto_logs")),
        }
    }
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
            return Err(DraptoError::Config(
                format!("Input file not found: {:?}", self.input)
            ));
        }
        
        if let Some(quality) = self.video.target_quality {
            if !(0.0..=100.0).contains(&quality) {
                return Err(DraptoError::Config(
                    format!("Target quality must be between 0 and 100, got {}", quality)
                ));
            }
        }
        
        if self.resources.parallel_jobs == 0 {
            return Err(DraptoError::Config(
                "Parallel jobs must be at least 1".to_string()
            ));
        }
        
        // Ensure the output directory exists or can be created
        if let Some(parent) = self.output.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    DraptoError::Config(
                        format!("Failed to create output directory: {}", e)
                    )
                })?;
            }
        }
        
        // Create necessary directories
        for dir in [&self.directories.temp_dir, 
                   self.directories.segments_dir.as_ref().unwrap_or(&self.directories.temp_dir.join("segments")),
                   self.directories.encoded_segments_dir.as_ref().unwrap_or(&self.directories.temp_dir.join("encoded_segments")),
                   self.directories.working_dir.as_ref().unwrap_or(&self.directories.temp_dir.join("working")),
                   &self.logging.log_dir] {
            std::fs::create_dir_all(dir).map_err(|e| {
                DraptoError::Config(
                    format!("Failed to create directory {}: {}", dir.display(), e)
                )
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

// Helper functions for environment variable handling

fn get_env_string(key: &str, default: String) -> String {
    std::env::var(key).unwrap_or(default)
}

fn get_env_path(key: &str, default: PathBuf) -> PathBuf {
    std::env::var(key).map(PathBuf::from).unwrap_or(default)
}

fn get_env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(val) => val.to_lowercase() == "true" || val == "1",
        Err(_) => default,
    }
}

fn get_env_f32(key: &str, default: f32) -> f32 {
    match std::env::var(key) {
        Ok(val) => val.parse().unwrap_or(default),
        Err(_) => default,
    }
}

fn get_env_u8(key: &str, default: u8) -> u8 {
    match std::env::var(key) {
        Ok(val) => val.parse().unwrap_or(default),
        Err(_) => default,
    }
}

fn get_env_u32(key: &str, default: u32) -> u32 {
    match std::env::var(key) {
        Ok(val) => val.parse().unwrap_or(default),
        Err(_) => default,
    }
}

fn get_env_usize(key: &str, default: usize) -> usize {
    match std::env::var(key) {
        Ok(val) => val.parse().unwrap_or(default),
        Err(_) => default,
    }
}