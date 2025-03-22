use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::{DraptoError, Result};

/// Configuration for drapto
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Input file path
    pub input: PathBuf,
    
    /// Output file path
    pub output: PathBuf,
    
    /// Enable hardware acceleration for decoding if available
    pub hardware_acceleration: bool,
    
    /// Hardware acceleration options for FFmpeg
    #[serde(default)]
    pub hw_accel_option: String,
    
    /// Target quality (0-100) - Currently not used as VMAF is disabled
    pub target_quality: Option<f32>,
    
    /// Number of parallel encoding jobs
    #[serde(default = "default_parallel_jobs")]
    pub parallel_jobs: usize,
    
    /// Enable verbose logging
    #[serde(default)]
    pub verbose: bool,
    
    /// Keep temporary files after encoding
    #[serde(default)]
    pub keep_temp_files: bool,
    
    /// Temporary directory for intermediate files
    #[serde(default = "default_temp_dir")]
    pub temp_dir: PathBuf,
    
    /// Scene detection threshold for SDR content (0-100)
    #[serde(default = "default_scene_threshold")]
    pub scene_threshold: f32,
    
    /// Scene detection threshold for HDR content (0-100)
    #[serde(default = "default_hdr_scene_threshold")]
    pub hdr_scene_threshold: f32,
    
    /// Minimum segment length in seconds
    #[serde(default = "default_min_segment_length")]
    pub min_segment_length: f32,
    
    /// Maximum segment length in seconds
    #[serde(default = "default_max_segment_length")]
    pub max_segment_length: f32,
    
    /// Disable automatic crop detection
    #[serde(default)]
    pub disable_crop: bool,
    
    /// Use scene-based segmentation and parallel encoding
    #[serde(default)]
    pub use_segmentation: bool,
    
    /// Memory limit per encoding job in MB (0 = auto)
    #[serde(default = "default_memory_per_job")]
    pub memory_per_job: usize,
}

fn default_parallel_jobs() -> usize {
    num_cpus::get()
}

fn default_temp_dir() -> PathBuf {
    std::env::temp_dir()
}

fn default_scene_threshold() -> f32 {
    40.0
}

fn default_hdr_scene_threshold() -> f32 {
    30.0
}

fn default_min_segment_length() -> f32 {
    5.0
}

fn default_max_segment_length() -> f32 {
    15.0
}

fn default_memory_per_job() -> usize {
    // Default to 2GB per job, will be adjusted based on resolution and encoder
    2048
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input: PathBuf::new(),
            output: PathBuf::new(),
            hardware_acceleration: true,
            hw_accel_option: String::new(),
            target_quality: Some(95.0),
            parallel_jobs: default_parallel_jobs(),
            verbose: false,
            keep_temp_files: false,
            temp_dir: default_temp_dir(),
            scene_threshold: default_scene_threshold(),
            hdr_scene_threshold: default_hdr_scene_threshold(),
            min_segment_length: default_min_segment_length(),
            max_segment_length: default_max_segment_length(),
            disable_crop: false,
            use_segmentation: true,
            memory_per_job: default_memory_per_job(),
        }
    }
}

impl Config {
    /// Create a new default configuration
    pub fn new() -> Self {
        Self::default()
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
    
    /// Set the hardware acceleration flag
    pub fn with_hardware_acceleration(mut self, enable: bool) -> Self {
        self.hardware_acceleration = enable;
        self
    }
    
    /// Set hardware acceleration options
    pub fn with_hw_accel_option<S: Into<String>>(mut self, options: S) -> Self {
        self.hw_accel_option = options.into();
        self
    }
    
    /// Set the scene threshold
    pub fn with_scene_threshold(mut self, threshold: f32) -> Self {
        self.scene_threshold = threshold;
        self
    }
    
    /// Set the HDR scene threshold
    pub fn with_hdr_scene_threshold(mut self, threshold: f32) -> Self {
        self.hdr_scene_threshold = threshold;
        self
    }
    
    /// Set the minimum segment length
    pub fn with_min_segment_length(mut self, length: f32) -> Self {
        self.min_segment_length = length;
        self
    }
    
    /// Set the maximum segment length
    pub fn with_max_segment_length(mut self, length: f32) -> Self {
        self.max_segment_length = length;
        self
    }
    
    /// Set whether to disable crop detection
    pub fn with_disable_crop(mut self, disable: bool) -> Self {
        self.disable_crop = disable;
        self
    }
    
    /// Set whether to use segmentation
    pub fn with_segmentation(mut self, enable: bool) -> Self {
        self.use_segmentation = enable;
        self
    }
    
    /// Set memory limit per encoding job
    pub fn with_memory_per_job(mut self, memory_mb: usize) -> Self {
        self.memory_per_job = memory_mb;
        self
    }
    
    
    /// Validate configuration parameters
    pub fn validate(&self) -> Result<()> {
        if !self.input.exists() {
            return Err(DraptoError::Config(
                format!("Input file not found: {:?}", self.input)
            ));
        }
        
        if let Some(quality) = self.target_quality {
            if !(0.0..=100.0).contains(&quality) {
                return Err(DraptoError::Config(
                    format!("Target quality must be between 0 and 100, got {}", quality)
                ));
            }
        }
        
        if self.parallel_jobs == 0 {
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
        
        Ok(())
    }
}