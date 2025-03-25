//! Directory configuration module
//!
//! Defines the configuration structure for file and directory paths
//! used throughout the application.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use super::utils::*;

/// Directory configuration for file management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryConfig {
    //
    // Primary directories
    //
    
    /// Temporary directory for intermediate files
    pub temp_dir: PathBuf,
    
    /// Working directory for temporary processing
    pub working_dir: Option<PathBuf>,
    
    //
    // Segmentation directories
    //

    /// Directory for segmented files
    pub segments_dir: Option<PathBuf>,

    /// Directory for encoded segments
    pub encoded_segments_dir: Option<PathBuf>,
    
    //
    // Cleanup options
    //

    /// Keep temporary files after encoding
    pub keep_temp_files: bool,
}

impl Default for DirectoryConfig {
    fn default() -> Self {
        let temp_dir = std::env::temp_dir().join("drapto");
        Self {
            // Primary directories
            
            // Temporary directory for intermediate files
            // Base directory for all temporary files
            // Default: system temp directory + /drapto
            temp_dir: get_env_path("DRAPTO_WORKDIR", temp_dir.clone()),
            
            // Working directory for temporary processing
            // Used for in-progress encoding operations
            // Default: system temp directory + /drapto/working
            working_dir: Some(get_env_path("DRAPTO_WORKING_DIR", temp_dir.join("working"))),
            
            // Segmentation directories
            
            // Directory for segmented files
            // Stores split video segments before encoding
            // Default: system temp directory + /drapto/segments
            segments_dir: Some(get_env_path(
                "DRAPTO_SEGMENTS_DIR",
                temp_dir.join("segments"),
            )),
            
            // Directory for encoded segments
            // Stores encoded video segments before merging
            // Default: system temp directory + /drapto/encoded_segments
            encoded_segments_dir: Some(get_env_path(
                "DRAPTO_ENCODED_SEGMENTS_DIR",
                temp_dir.join("encoded_segments"),
            )),
            
            // Cleanup options
            
            // Keep temporary files after encoding
            // When true, doesn't delete temporary files for debugging
            // Default: false - Clean up temp files when done
            keep_temp_files: false,
        }
    }
}