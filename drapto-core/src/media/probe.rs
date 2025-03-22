use std::path::Path;
use std::process::Command;
use serde_json::Value;

use crate::error::{DraptoError, Result};
use crate::logging;
use crate::util::command;

/// Information about a codec
#[derive(Debug, Clone)]
pub struct CodecInfo {
    pub name: String,
    pub type_name: String,
    pub description: String,
}

/// FFprobe command executor
pub struct FFprobe;

impl FFprobe {
    /// Create a new FFprobe instance
    pub fn new() -> Self {
        Self
    }
    
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
    
    /// Get FFmpeg version
    pub fn get_version(&self) -> Result<String> {
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-version");
        
        let output = command::run_command(&mut cmd)?;
        
        let version_str = String::from_utf8_lossy(&output.stdout);
        let first_line = version_str.lines().next().unwrap_or_default();
        
        Ok(first_line.to_string())
    }
    
    /// Get available decoders
    pub fn get_decoders(&self) -> Result<Vec<CodecInfo>> {
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-hide_banner", "-decoders"]);
        
        let output = command::run_command(&mut cmd)?;
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        
        // Parse the output
        let mut decoders = Vec::new();
        let mut parsing = false;
        
        for line in output_str.lines() {
            if line.contains("------") {
                parsing = true;
                continue;
            }
            
            if parsing && !line.trim().is_empty() {
                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if parts.len() >= 3 {
                    let codec_type = match parts[0].chars().nth(1) {
                        Some('V') => "video",
                        Some('A') => "audio",
                        Some('S') => "subtitle",
                        _ => "other",
                    };
                    
                    let name = parts[1].to_string();
                    let description = parts[2..].join(" ");
                    
                    decoders.push(CodecInfo {
                        name,
                        type_name: codec_type.to_string(),
                        description,
                    });
                }
            }
        }
        
        Ok(decoders)
    }
    
    /// Get available encoders
    pub fn get_encoders(&self) -> Result<Vec<CodecInfo>> {
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-hide_banner", "-encoders"]);
        
        let output = command::run_command(&mut cmd)?;
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        
        // Parse the output
        let mut encoders = Vec::new();
        let mut parsing = false;
        
        for line in output_str.lines() {
            if line.contains("------") {
                parsing = true;
                continue;
            }
            
            if parsing && !line.trim().is_empty() {
                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if parts.len() >= 3 {
                    let codec_type = match parts[0].chars().nth(1) {
                        Some('V') => "video",
                        Some('A') => "audio",
                        Some('S') => "subtitle",
                        _ => "other",
                    };
                    
                    let name = parts[1].to_string();
                    let description = parts[2..].join(" ");
                    
                    encoders.push(CodecInfo {
                        name,
                        type_name: codec_type.to_string(),
                        description,
                    });
                }
            }
        }
        
        Ok(encoders)
    }
    
    /// Get available hardware accelerators
    pub fn get_hwaccels(&self) -> Result<Vec<String>> {
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-hide_banner", "-hwaccels"]);
        
        let output = command::run_command(&mut cmd)?;
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        
        // Parse the output, skipping the first line which is just "Hardware acceleration methods:"
        let hwaccels: Vec<String> = output_str
            .lines()
            .skip(1)
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();
        
        Ok(hwaccels)
    }
    
    /// Check if hardware decoding is supported on this platform
    /// Returns the appropriate hardware acceleration option for FFmpeg
    pub fn check_hardware_decoding(&self) -> Result<Option<String>> {
        // On MacOS, check for VideoToolbox
        if cfg!(target_os = "macos") {
            match self.get_hwaccels()? {
                hwaccels if hwaccels.iter().any(|h| h == "videotoolbox") => {
                    log::info!("Found VideoToolbox hardware decoding on macOS");
                    return Ok(Some("-hwaccel videotoolbox".to_string()));
                }
                _ => {
                    log::info!("VideoToolbox not available on macOS");
                    return Ok(None);
                }
            }
        }
        
        // On Linux, check for VAAPI
        if cfg!(target_os = "linux") {
            match self.get_hwaccels()? {
                hwaccels if hwaccels.iter().any(|h| h == "vaapi") => {
                    log::info!("Found VAAPI hardware decoding on Linux");
                    return Ok(Some("-hwaccel vaapi".to_string()));
                }
                _ => {
                    log::info!("VAAPI not available on Linux");
                    return Ok(None);
                }
            }
        }
        
        // No supported hardware decoding for other platforms
        log::info!("No supported hardware decoding on this platform");
        Ok(None)
    }
}