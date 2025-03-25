//! Logging configuration module
//!
//! Defines the configuration structure for logging settings
//! including log level, verbosity, and file paths.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use super::utils::*;

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    //
    // Log detail settings
    //
    
    /// Enable verbose logging
    pub verbose: bool,

    /// Log level (DEBUG, INFO, WARNING, ERROR)
    pub log_level: String,
    
    //
    // Log destination
    //

    /// Log directory
    pub log_dir: PathBuf,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            // Log detail settings
            
            // Enable verbose logging
            // When true, more detailed logs will be output
            // Default: false - Only show essential information
            verbose: get_env_bool("DRAPTO_VERBOSE", false),
            
            // Log level (DEBUG, INFO, WARNING, ERROR)
            // Controls the minimum severity level of messages to output
            // Default: "INFO" - Show informational messages and errors
            log_level: get_env_string("DRAPTO_LOG_LEVEL", "INFO".to_string()),
            
            // Log destination
            
            // Log directory
            // Where log files will be written
            // Default: "~/drapto_logs" in user's home directory
            log_dir: get_env_path("DRAPTO_LOG_DIR", home.join("drapto_logs")),
        }
    }
}