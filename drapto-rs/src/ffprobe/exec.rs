use std::path::Path;
use std::process::Command;
use serde_json::Value;

use crate::error::{DraptoError, Result};
use crate::logging;

/// FFprobe command executor
pub struct FFprobe;

impl FFprobe {
    /// Execute ffprobe and return JSON output
    pub fn execute<P: AsRef<Path>>(input_path: P) -> Result<Value> {
        let path = input_path.as_ref();
        
        if !path.exists() {
            return Err(DraptoError::MediaFile(
                format!("File not found: {:?}", path)
            ));
        }
        
        let mut cmd = Command::new("ffprobe");
        cmd.args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            "-show_streams",
            "-show_chapters",
            path.to_str().unwrap_or_default(),
        ]);
        
        logging::log_command(&cmd);
        
        let output = cmd
            .output()
            .map_err(|e| DraptoError::ExternalTool(format!("Failed to execute ffprobe: {}", e)))?;
        
        if !output.status.success() {
            return Err(DraptoError::ExternalTool(
                format!("ffprobe exited with error: {}", 
                    String::from_utf8_lossy(&output.stderr))
            ));
        }
        
        serde_json::from_slice(&output.stdout)
            .map_err(|e| DraptoError::ExternalTool(format!("Failed to parse ffprobe output: {}", e)))
    }
    
    /// Check if ffprobe is available on the system
    pub fn is_available() -> bool {
        Command::new("ffprobe")
            .arg("-version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    /// Get ffprobe version string
    pub fn version() -> Result<String> {
        let output = Command::new("ffprobe")
            .arg("-version")
            .output()
            .map_err(|e| DraptoError::ExternalTool(format!("Failed to execute ffprobe: {}", e)))?;
            
        if !output.status.success() {
            return Err(DraptoError::ExternalTool(
                "ffprobe failed to return version".to_string()
            ));
        }
        
        let version_str = String::from_utf8_lossy(&output.stdout);
        let first_line = version_str.lines().next().unwrap_or_default();
        
        Ok(first_line.to_string())
    }
}