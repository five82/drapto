use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

use crate::error::Result;
use super::command::{run_command, run_command_with_progress, ProgressCallback};

/// A trait for executable command jobs
pub trait CommandJob {
    /// Get the command as a list of arguments
    fn get_command(&self) -> Command;
    
    /// Execute the job
    fn execute(&self) -> Result<Output> {
        let mut cmd = self.get_command();
        run_command(&mut cmd)
    }
    
    /// Execute the job with progress reporting
    fn execute_with_progress(&self, progress_cb: Option<ProgressCallback>) -> Result<Output> {
        let mut cmd = self.get_command();
        run_command_with_progress(&mut cmd, progress_cb, None)
    }
    
    /// Execute the job with a timeout
    fn execute_with_timeout(&self, timeout: Duration) -> Result<Output> {
        let mut cmd = self.get_command();
        run_command_with_progress(&mut cmd, None, Some(timeout))
    }
    
    /// Execute the job with progress reporting and timeout
    fn execute_with_progress_and_timeout(
        &self, 
        progress_cb: Option<ProgressCallback>,
        timeout: Duration
    ) -> Result<Output> {
        let mut cmd = self.get_command();
        run_command_with_progress(&mut cmd, progress_cb, Some(timeout))
    }
}

/// A command job for FFmpeg encoding
pub struct FFmpegEncodeJob {
    input: PathBuf,
    output: PathBuf,
    args: Vec<String>,
}

impl FFmpegEncodeJob {
    /// Create a new FFmpeg encode job
    pub fn new<P: AsRef<Path>>(input: P, output: P, args: Vec<String>) -> Self {
        Self {
            input: input.as_ref().to_path_buf(),
            output: output.as_ref().to_path_buf(),
            args,
        }
    }
}

impl CommandJob for FFmpegEncodeJob {
    fn get_command(&self) -> Command {
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y")
           .arg("-i")
           .arg(&self.input);
           
        // Add all custom arguments
        for arg in &self.args {
            cmd.arg(arg);
        }
        
        cmd.arg(&self.output);
        cmd
    }
}

/// A command job for FFprobe analysis
pub struct FFprobeJob {
    input: PathBuf,
    format_json: bool,
}

impl FFprobeJob {
    /// Create a new FFprobe job
    pub fn new<P: AsRef<Path>>(input: P, format_json: bool) -> Self {
        Self {
            input: input.as_ref().to_path_buf(),
            format_json,
        }
    }
}

impl CommandJob for FFprobeJob {
    fn get_command(&self) -> Command {
        let mut cmd = Command::new("ffprobe");
        cmd.args(["-v", "quiet"]);
        
        if self.format_json {
            cmd.args(["-print_format", "json"]);
        }
        
        cmd.args([
            "-show_format",
            "-show_streams",
            "-show_chapters",
        ]);
        
        cmd.arg(&self.input);
        cmd
    }
}

/// A command job for audio encoding
pub struct AudioEncodeJob {
    input: PathBuf,
    output: PathBuf,
    bitrate: u32,
    channels: u32,
}

impl AudioEncodeJob {
    /// Create a new audio encode job
    pub fn new<P: AsRef<Path>>(input: P, output: P, bitrate: u32, channels: u32) -> Self {
        Self {
            input: input.as_ref().to_path_buf(),
            output: output.as_ref().to_path_buf(),
            bitrate,
            channels,
        }
    }
}

impl CommandJob for AudioEncodeJob {
    fn get_command(&self) -> Command {
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y")
           .arg("-i")
           .arg(&self.input)
           .arg("-c:a")
           .arg("libopus")
           .arg("-b:a")
           .arg(format!("{}k", self.bitrate))
           .arg("-ac")
           .arg(self.channels.to_string())
           .arg(&self.output);
        cmd
    }
}

/// A command job for video segmentation
pub struct SegmentationJob {
    input: PathBuf,
    output_pattern: PathBuf,
    segment_list: PathBuf,
    args: Vec<String>,
}

impl SegmentationJob {
    /// Create a new segmentation job
    pub fn new<P: AsRef<Path>>(
        input: P, 
        output_pattern: P,
        segment_list: P,
        args: Vec<String>
    ) -> Self {
        Self {
            input: input.as_ref().to_path_buf(),
            output_pattern: output_pattern.as_ref().to_path_buf(),
            segment_list: segment_list.as_ref().to_path_buf(),
            args,
        }
    }
}

impl CommandJob for SegmentationJob {
    fn get_command(&self) -> Command {
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y")
           .arg("-i")
           .arg(&self.input);
           
        // Add all custom arguments
        for arg in &self.args {
            cmd.arg(arg);
        }
        
        cmd.arg("-f")
           .arg("segment")
           .arg("-segment_list")
           .arg(&self.segment_list)
           .arg(&self.output_pattern);
        cmd
    }
}

/// A command job for concatenating video segments
pub struct ConcatenationJob {
    segment_list: PathBuf,
    output: PathBuf,
    copy_codecs: bool,
}

impl ConcatenationJob {
    /// Create a new concatenation job
    pub fn new<P: AsRef<Path>>(segment_list: P, output: P, copy_codecs: bool) -> Self {
        Self {
            segment_list: segment_list.as_ref().to_path_buf(),
            output: output.as_ref().to_path_buf(),
            copy_codecs,
        }
    }
}

impl CommandJob for ConcatenationJob {
    fn get_command(&self) -> Command {
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y")
           .arg("-f")
           .arg("concat")
           .arg("-safe")
           .arg("0")
           .arg("-i")
           .arg(&self.segment_list);
        
        if self.copy_codecs {
            cmd.arg("-c")
               .arg("copy");
        }
        
        cmd.arg(&self.output);
        cmd
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ffmpeg_encode_job() {
        let job = FFmpegEncodeJob::new("input.mp4", "output.mp4", vec!["-c:v".to_string(), "libx264".to_string()]);
        let cmd = job.get_command();
        
        // Verify command structure
        assert_eq!(cmd.get_program(), "ffmpeg");
        
        // This is a simplistic check that just verifies the command is structured correctly
        // A more robust test would check the exact arguments, but that would be more complex
    }
    
    #[test]
    fn test_ffprobe_job() {
        let job = FFprobeJob::new("input.mp4", true);
        let cmd = job.get_command();
        
        assert_eq!(cmd.get_program(), "ffprobe");
    }
    
    #[test]
    fn test_audio_encode_job() {
        let job = AudioEncodeJob::new("input.wav", "output.opus", 128, 2);
        let cmd = job.get_command();
        
        assert_eq!(cmd.get_program(), "ffmpeg");
    }
    
    #[test]
    fn test_segmentation_job() {
        let job = SegmentationJob::new(
            "input.mp4", 
            "segment_%03d.mp4",
            "segments.txt",
            vec!["-c:v".to_string(), "copy".to_string()]
        );
        let cmd = job.get_command();
        
        assert_eq!(cmd.get_program(), "ffmpeg");
    }
    
    #[test]
    fn test_concatenation_job() {
        let job = ConcatenationJob::new("segments.txt", "output.mp4", true);
        let cmd = job.get_command();
        
        assert_eq!(cmd.get_program(), "ffmpeg");
    }
}