use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for drapto
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Input file path
    pub input: PathBuf,
    
    /// Output file path
    pub output: PathBuf,
    
    /// Enable hardware acceleration if available
    pub hardware_acceleration: bool,
    
    /// Target VMAF quality (0-100)
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
}

fn default_parallel_jobs() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(2)
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

impl Default for Config {
    fn default() -> Self {
        Self {
            input: PathBuf::new(),
            output: PathBuf::new(),
            hardware_acceleration: true,
            target_quality: Some(95.0),
            parallel_jobs: default_parallel_jobs(),
            verbose: false,
            keep_temp_files: false,
            temp_dir: default_temp_dir(),
            scene_threshold: default_scene_threshold(),
            hdr_scene_threshold: default_hdr_scene_threshold(),
            min_segment_length: default_min_segment_length(),
            max_segment_length: default_max_segment_length(),
        }
    }
}

impl Config {
    /// Create a new default configuration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Validate configuration parameters
    pub fn validate(&self) -> crate::error::Result<()> {
        if !self.input.exists() {
            return Err(crate::error::DraptoError::Config(
                format!("Input file not found: {:?}", self.input)
            ));
        }
        
        if let Some(quality) = self.target_quality {
            if !(0.0..=100.0).contains(&quality) {
                return Err(crate::error::DraptoError::Config(
                    format!("Target quality must be between 0 and 100, got {}", quality)
                ));
            }
        }
        
        if self.parallel_jobs == 0 {
            return Err(crate::error::DraptoError::Config(
                "Parallel jobs must be at least 1".to_string()
            ));
        }
        
        // Ensure the output directory exists or can be created
        if let Some(parent) = self.output.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    crate::error::DraptoError::Config(
                        format!("Failed to create output directory: {}", e)
                    )
                })?;
            }
        }
        
        Ok(())
    }
}