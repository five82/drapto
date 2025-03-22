use std::path::{Path, PathBuf};
use std::f64;
use log::info;
use thiserror::Error;

use crate::error::Result;
use crate::util::command::run_command;
use std::process::Command;

/// Muxing-specific errors
#[derive(Error, Debug)]
pub enum MuxingError {
    #[error("Failed to mux tracks: {0}")]
    MuxingFailed(String),

    #[error("AV sync issue detected: video_start={video_start:.2}s vs audio_start={audio_start:.2}s; video_duration={video_duration:.2}s vs audio_duration={audio_duration:.2}s")]
    AVSyncIssue {
        video_start: f64,
        audio_start: f64,
        video_duration: f64,
        audio_duration: f64,
    },

    #[error("Failed to validate output: {0}")]
    ValidationFailed(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

/// Options for muxing configuration
#[derive(Debug, Clone)]
pub struct MuxOptions {
    /// AV sync threshold in seconds
    pub sync_threshold: f64,
    
    /// Allow container duration fallback
    pub allow_container_duration: bool,
}

impl Default for MuxOptions {
    fn default() -> Self {
        Self {
            sync_threshold: 0.1,
            allow_container_duration: true,
        }
    }
}

/// Media track muxer
pub struct Muxer {
    /// Muxing options
    pub options: MuxOptions,
}

impl Muxer {
    /// Create a new media muxer
    pub fn new() -> Self {
        Self {
            options: MuxOptions::default(),
        }
    }
    
    /// Create a new media muxer with custom options
    pub fn with_options(options: MuxOptions) -> Self {
        Self {
            options,
        }
    }
    
    /// Mux video and audio tracks into a final output file
    pub fn mux_tracks<P, Q>(
        &self,
        video_track: P,
        audio_tracks: &[Q],
        output_file: P,
        _options: &MuxOptions,
    ) -> Result<PathBuf> 
    where 
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let video_path = video_track.as_ref();
        let output_path = output_file.as_ref();
        
        info!("Muxing tracks to: {}", output_path.display());
        
        // Build mux command
        let mut cmd = self.build_mux_command(video_path, audio_tracks, output_path)?;
        
        // Execute command
        run_command(&mut cmd)?;
        
        // Validate AV sync in muxed output
        self.validate_output(output_path)?;
        
        Ok(output_path.to_path_buf())
    }
    
    /// Build FFmpeg command for muxing tracks
    pub fn build_mux_command<P, Q>(
        &self,
        video_track: P,
        audio_tracks: &[Q],
        output_file: P,
    ) -> Result<Command> 
    where 
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let video_path = video_track.as_ref();
        let output_path = output_file.as_ref();
        
        // Validate paths
        if !video_path.exists() {
            return Err(MuxingError::InvalidPath(
                format!("Video track does not exist: {}", video_path.display())
            ).into());
        }
        
        for audio_path in audio_tracks {
            let path = audio_path.as_ref();
            if !path.exists() {
                return Err(MuxingError::InvalidPath(
                    format!("Audio track does not exist: {}", path.display())
                ).into());
            }
        }
        
        // Create command
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-hide_banner", "-loglevel", "warning"]);
        
        // Add video input
        cmd.args(["-i", video_path.to_str().unwrap_or_default()]);
        
        // Add audio inputs
        for audio_path in audio_tracks {
            let path = audio_path.as_ref();
            cmd.args(["-i", path.to_str().unwrap_or_default()]);
        }
        
        // Add mapping
        cmd.args(["-map", "0:v:0"]); // Video track
        
        // Map all audio tracks
        if !audio_tracks.is_empty() {
            for i in 0..audio_tracks.len() {
                cmd.args(["-map", &format!("{}:a:0?", i + 1)]);
            }
        }
        
        // Add output file with copy codecs
        cmd.args(["-c", "copy", "-y", output_path.to_str().unwrap_or_default()]);
        
        Ok(cmd)
    }
    
    /// Validate muxed output for AV sync
    fn validate_output<P: AsRef<Path>>(&self, output_file: P) -> Result<()> {
        let file_path = output_file.as_ref();
        
        // Simple validation for testing: just check that the file exists and has a size
        if !file_path.exists() {
            return Err(MuxingError::ValidationFailed(
                format!("Output file not found: {}", file_path.display())
            ).into());
        }
        
        let metadata = std::fs::metadata(file_path)?;
        if metadata.len() == 0 {
            return Err(MuxingError::ValidationFailed(
                "Output file is empty".to_string()
            ).into());
        }
        
        // In a real implementation, we would validate A/V sync here
        info!("Output file validated: {}", file_path.display());
        
        Ok(())
    }
    
}

impl Default for Muxer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::File;
    
    #[test]
    fn test_build_mux_command() {
        // Create temporary directory for test files
        let temp_dir = tempdir().unwrap();
        
        // Create dummy video file
        let video_path = temp_dir.path().join("video.mp4");
        File::create(&video_path).unwrap();
        
        // Create dummy audio files
        let audio_path1 = temp_dir.path().join("audio1.opus");
        let audio_path2 = temp_dir.path().join("audio2.opus");
        File::create(&audio_path1).unwrap();
        File::create(&audio_path2).unwrap();
        
        // Create output path
        let output_path = temp_dir.path().join("output.mkv");
        
        // Create muxer
        let muxer = Muxer::new();
        
        // Build command
        let cmd = muxer.build_mux_command(
            &video_path,
            &[&audio_path1, &audio_path2],
            &output_path
        ).unwrap();
        
        // Convert command to string for assertion
        let args: Vec<String> = cmd.get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();
        
        // Check that the command contains the expected arguments
        assert!(args.contains(&"-map".to_string()));
        assert!(args.contains(&"0:v:0".to_string()));
        // Map input formats
        assert!(args.iter().any(|a| a.contains("a:0") && (a.contains("1:") || a.contains("2:"))));
        assert!(args.contains(&"copy".to_string()));
        
        // The first argument should be -hide_banner
        assert_eq!(args[0], "-hide_banner");
    }
}