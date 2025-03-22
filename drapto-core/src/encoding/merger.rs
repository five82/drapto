//! Segment merger functionality for drapto
//!
//! This module handles the merging of video segments using FFmpeg's concat demuxer.
//! It provides tools to concatenate multiple video segments into a single file,
//! validate the merged output, and manage temporary files used during the process.

use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{self, Write};
use std::process::Command;
use log::{debug, info, error};
use thiserror::Error;

use crate::error::Result;
use crate::util::command;
use crate::media::MediaInfo;

/// Errors related to segment merging operations
#[derive(Error, Debug)]
pub enum SegmentMergerError {
    #[error("Failed to create merged output: {0}")]
    CreationFailed(String),
    
    #[error("Invalid segment file: {0}")]
    InvalidSegment(String),
    
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    
    #[error("Merged output validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Duration mismatch - expected: {expected:.2}s, actual: {actual:.2}s")]
    DurationMismatch { expected: f64, actual: f64 },
    
    #[error("Wrong codec in merged output - expected: {expected}, actual: {actual}")]
    WrongCodec { expected: String, actual: String },
    
    #[error("Timing anomaly detected in merged output: {0}")]
    TimingAnomaly(String),
}

/// Options for segment merging
#[derive(Clone, Debug)]
pub struct MergeOptions {
    /// Copy all streams without re-encoding
    pub copy_streams: bool,
    /// Add faststart flag for web optimization (MP4)
    pub faststart: bool,
    /// Generate missing presentation timestamps
    pub generate_pts: bool,
    /// Copy metadata from first input
    pub copy_metadata: bool,
    /// Expected video codec in output (for validation)
    pub expected_codec: Option<String>,
    /// How much difference in duration is acceptable (seconds)
    pub duration_tolerance: f64,
    /// Maximum acceptable start time offset (seconds)
    pub start_time_tolerance: f64,
}

impl Default for MergeOptions {
    fn default() -> Self {
        Self {
            copy_streams: true,
            faststart: true,
            generate_pts: true,
            copy_metadata: true,
            expected_codec: Some("av1".to_string()),
            duration_tolerance: 1.0,
            start_time_tolerance: 0.2,
        }
    }
}

/// Main functionality for merging video segments
pub struct SegmentMerger {
    pub options: MergeOptions,
}

impl SegmentMerger {
    /// Create a new segment merger with default options
    pub fn new() -> Self {
        Self {
            options: MergeOptions::default(),
        }
    }
    
    /// Create a new segment merger with custom options
    pub fn with_options(options: MergeOptions) -> Self {
        Self { options }
    }
    
    /// Merge video segments into a single output file
    ///
    /// # Arguments
    ///
    /// * `segments` - List of segment file paths to merge
    /// * `output` - Output file path for the merged video
    ///
    /// # Returns
    ///
    /// * `Result<PathBuf>` - Path to the merged output if successful
    pub fn merge_segments<P: AsRef<Path>>(
        &self,
        segments: &[PathBuf],
        output: P,
    ) -> Result<PathBuf> {
        let output_path = output.as_ref().to_path_buf();
        
        if segments.is_empty() {
            return Err(SegmentMergerError::CreationFailed("No segments to merge".to_string()).into());
        }
        
        // Ensure output directory exists
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        
        // Calculate total duration for validation
        let mut total_segment_duration = 0.0;
        for segment in segments {
            if !segment.exists() {
                return Err(SegmentMergerError::InvalidSegment(
                    format!("Segment file does not exist: {:?}", segment)
                ).into());
            }
            
            match MediaInfo::from_path(segment) {
                Ok(info) => {
                    if let Some(duration) = info.duration() {
                        total_segment_duration += duration;
                        debug!("Segment {:?}: duration={:.2}s", segment.file_name().unwrap_or_default(), duration);
                    } else {
                        return Err(SegmentMergerError::InvalidSegment(
                            format!("Could not determine duration for segment: {:?}", segment)
                        ).into());
                    }
                },
                Err(e) => {
                    return Err(SegmentMergerError::InvalidSegment(
                        format!("Failed to get info for segment {:?}: {}", segment, e)
                    ).into());
                }
            }
        }
        
        info!("Merging {} segments with total duration {:.2}s", segments.len(), total_segment_duration);
        
        // Create temporary concatenation file
        let concat_file = if let Some(parent) = output_path.parent() {
            parent.join("concat.txt")
        } else {
            PathBuf::from("concat.txt")
        };
        
        // Write segment paths to concat file
        let mut file = File::create(&concat_file)?;
        for segment in segments {
            writeln!(file, "file '{}'", segment.to_string_lossy())?;
        }
        file.flush()?;
        
        // Build FFmpeg command
        let mut cmd = self.build_concat_command(&concat_file, &output_path);
        
        // Execute command
        match command::run_command(&mut cmd) {
            Ok(_) => {
                debug!("Successfully executed concat command");
            },
            Err(e) => {
                // Clean up concat file
                if concat_file.exists() {
                    let _ = std::fs::remove_file(&concat_file);
                }
                
                return Err(SegmentMergerError::CreationFailed(
                    format!("Failed to execute concat command: {}", e)
                ).into());
            }
        }
        
        // Clean up concat file
        if concat_file.exists() {
            let _ = std::fs::remove_file(&concat_file);
        }
        
        // Verify the output exists
        if !output_path.exists() || std::fs::metadata(&output_path)?.len() == 0 {
            return Err(SegmentMergerError::ValidationFailed(
                "Merged output file is missing or empty".to_string()
            ).into());
        }
        
        // Validate merged output
        if let Err(e) = self.validate_merged_output(&output_path, total_segment_duration) {
            error!("Merged output validation failed: {}", e);
            return Err(e);
        }
        
        info!("Successfully merged segments into {:?}", output_path);
        Ok(output_path)
    }
    
    /// Build the FFmpeg command for concatenation
    fn build_concat_command(&self, concat_file: &Path, output_file: &Path) -> Command {
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-hide_banner", "-loglevel", "warning"])
            .args(["-f", "concat", "-safe", "0"])
            .args(["-i", concat_file.to_str().unwrap_or_default()]);
            
        // Add copy option if needed
        if self.options.copy_streams {
            cmd.args(["-c", "copy"]);
        }
        
        // Add faststart flag for web optimization
        if self.options.faststart {
            cmd.args(["-movflags", "+faststart"]);
        }
        
        // Generate missing PTS values
        if self.options.generate_pts {
            cmd.args(["-fflags", "+genpts"]);
        }
        
        // Copy metadata from first input
        if self.options.copy_metadata {
            cmd.args(["-map_metadata", "0"]);
        }
        
        // Output file
        cmd.args(["-y", output_file.to_str().unwrap_or_default()]);
        
        cmd
    }
    
    /// Validate the merged output
    fn validate_merged_output<P: AsRef<Path>>(
        &self,
        output_file: P,
        expected_duration: f64,
    ) -> Result<()> {
        let output_path = output_file.as_ref();
        let info = MediaInfo::from_path(output_path)?;
        
        // Check duration
        let output_duration = info.duration().ok_or_else(|| {
            SegmentMergerError::ValidationFailed(
                format!("Could not determine merged output duration: {:?}", output_path)
            )
        })?;
        
        // Verify duration is within tolerance
        if (output_duration - expected_duration).abs() > self.options.duration_tolerance {
            return Err(SegmentMergerError::DurationMismatch {
                expected: expected_duration,
                actual: output_duration,
            }.into());
        }
        
        // Check codec if expected
        if let Some(expected_codec) = &self.options.expected_codec {
            if let Some(video_stream) = info.primary_video_stream() {
                let codec = video_stream.codec_name.clone();
                if codec != *expected_codec {
                    return Err(SegmentMergerError::WrongCodec {
                        expected: expected_codec.clone(),
                        actual: codec,
                    }.into());
                }
            } else {
                return Err(SegmentMergerError::ValidationFailed(
                    "No video stream found in merged output".to_string()
                ).into());
            }
        }
        
        // Validate timing
        if let Some(video_stream) = info.primary_video_stream() {
            let start_time = video_stream.properties.get("start_time")
                .and_then(|v| v.as_str())
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.0);
                
            if start_time.abs() > self.options.start_time_tolerance {
                return Err(SegmentMergerError::TimingAnomaly(
                    format!("Video start time anomaly: {:.2}s", start_time)
                ).into());
            }
        }
        
        Ok(())
    }
}

/// Helper function to merge segments with default options
pub fn merge_segments<P: AsRef<Path>>(
    segments: &[PathBuf],
    output: P,
) -> Result<PathBuf> {
    SegmentMerger::new().merge_segments(segments, output)
}

/// Helper function to build a concatenation command
pub fn build_concat_command<P: AsRef<Path>, Q: AsRef<Path>>(
    concat_file: P,
    output_file: Q,
) -> Command {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-hide_banner", "-loglevel", "warning"])
        .args(["-f", "concat", "-safe", "0"])
        .args(["-i", concat_file.as_ref().to_str().unwrap_or_default()])
        .args(["-c", "copy"])
        .args(["-movflags", "+faststart"])
        .args(["-fflags", "+genpts"])
        .args(["-map_metadata", "0"])
        .args(["-y", output_file.as_ref().to_str().unwrap_or_default()]);
    
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    
    #[test]
    fn test_build_concat_command() {
        let concat_file = Path::new("/tmp/concat.txt");
        let output_file = Path::new("/tmp/output.mkv");
        
        let cmd = build_concat_command(concat_file, output_file);
        let args: Vec<&OsStr> = cmd.get_args().collect();
        
        // Check the command includes these arguments
        assert!(args.contains(&OsStr::new("-f")));
        assert!(args.contains(&OsStr::new("concat")));
        assert!(args.contains(&OsStr::new("-c")));
        assert!(args.contains(&OsStr::new("copy")));
        assert!(args.contains(&OsStr::new("-y")));
    }
    
    #[test]
    fn test_merger_options() {
        let options = MergeOptions {
            copy_streams: false,
            faststart: false,
            expected_codec: Some("h264".to_string()),
            ..Default::default()
        };
        
        let merger = SegmentMerger::with_options(options);
        assert!(!merger.options.copy_streams);
        assert!(!merger.options.faststart);
        assert_eq!(merger.options.expected_codec, Some("h264".to_string()));
        assert!(merger.options.generate_pts);
    }
}