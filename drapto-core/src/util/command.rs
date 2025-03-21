use std::process::{Command, Stdio, Output};
use std::io::{BufRead, BufReader};
use std::time::Duration;
use thiserror::Error;
use log::{debug, error};

use crate::error::{DraptoError, Result};
use crate::logging;

/// Command execution errors
#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Command failed to execute: {0}")]
    ExecutionFailed(String),
    
    #[error("Command timed out after {0} seconds")]
    Timeout(u64),
    
    #[error("Command was killed: {0}")]
    Killed(String),
    
    #[error("Command exited with non-zero status: {0}")]
    NonZeroExit(i32),
    
    #[error("Failed to create process: {0}")]
    ProcessCreation(#[from] std::io::Error),
}

/// Progress callback signature
pub type ProgressCallback = Box<dyn Fn(f32) + Send>;

/// Execute a simple command and return the output
pub fn run_command(cmd: &mut Command) -> Result<Output> {
    logging::log_command(cmd);
    
    let output = cmd.output()
        .map_err(|e| {
            error!("Failed to execute command: {}", e);
            DraptoError::CommandExecution(format!("Failed to execute command: {}", e))
        })?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Command failed with exit code {}: {}", 
               output.status.code().unwrap_or(-1), stderr);
        
        return Err(DraptoError::CommandExecution(format!(
            "Command failed with exit code {}: {}", 
            output.status.code().unwrap_or(-1),
            stderr
        )));
    }
    
    Ok(output)
}

/// Execute a command with progress reporting
pub fn run_command_with_progress(
    cmd: &mut Command, 
    progress_cb: Option<ProgressCallback>,
    timeout: Option<Duration>,
) -> Result<Output> {
    logging::log_command(cmd);
    
    // Set up pipes for stdout/stderr
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            error!("Failed to spawn command: {}", e);
            DraptoError::CommandExecution(format!("Failed to spawn command: {}", e))
        })?;
    
    // Set up readers for stdout and stderr
    let stdout = BufReader::new(child.stdout.take().unwrap());
    let stderr = BufReader::new(child.stderr.take().unwrap());
    
    // Create a thread for monitoring stdout
    let stdout_handle = std::thread::spawn(move || {
        let mut lines = Vec::new();
        // Using explicit if-let pattern for better error handling
        #[allow(clippy::manual_flatten)]
        for line_result in stdout.lines() {
            if let Ok(line) = line_result {
                debug!("STDOUT: {}", line);
                
                // Parse progress if appropriate
                if let Some(progress) = parse_ffmpeg_progress(&line) {
                    if let Some(cb) = &progress_cb {
                        cb(progress);
                    }
                }
                
                lines.push(line);
            }
        }
        lines
    });
    
    // Create a thread for monitoring stderr
    let stderr_handle = std::thread::spawn(move || {
        let mut lines = Vec::new();
        // Using explicit if-let pattern for better error handling
        #[allow(clippy::manual_flatten)]
        for line_result in stderr.lines() {
            if let Ok(line) = line_result {
                debug!("STDERR: {}", line);
                lines.push(line);
            }
        }
        lines
    });
    
    // Wait for the process with optional timeout
    let status = if let Some(timeout) = timeout {
        let start = std::time::Instant::now();
        let mut status = None;
        
        while start.elapsed() < timeout {
            match child.try_wait() {
                Ok(Some(s)) => {
                    status = Some(s);
                    break;
                },
                Ok(None) => {
                    // Process still running, sleep a bit
                    std::thread::sleep(Duration::from_millis(100));
                },
                Err(e) => {
                    return Err(DraptoError::CommandExecution(
                        format!("Error waiting for process: {}", e)
                    ));
                }
            }
        }
        
        // If we didn't get a status, kill the process due to timeout
        if status.is_none() {
            let _ = child.kill();
            return Err(DraptoError::CommandExecution(
                format!("Command timed out after {} seconds", timeout.as_secs())
            ));
        }
        
        status.unwrap()
    } else {
        // No timeout, just wait
        child.wait().map_err(|e| {
            DraptoError::CommandExecution(format!("Error waiting for process: {}", e))
        })?
    };
    
    // Collect output from stdout and stderr threads
    let all_stdout = stdout_handle.join().unwrap_or_default();
    let all_stderr = stderr_handle.join().unwrap_or_default();
    
    // Prepare the output
    let output = Output {
        status,
        stdout: all_stdout.join("\n").as_bytes().to_vec(),
        stderr: all_stderr.join("\n").as_bytes().to_vec(),
    };
    
    // Check for successful exit
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Command failed with exit code {}: {}", 
               output.status.code().unwrap_or(-1), stderr);
        
        return Err(DraptoError::CommandExecution(format!(
            "Command failed with exit code {}: {}", 
            output.status.code().unwrap_or(-1),
            stderr
        )));
    }
    
    Ok(output)
}

/// Parse ffmpeg output for progress information
fn parse_ffmpeg_progress(line: &str) -> Option<f32> {
    // Look for time= or out_time= in ffmpeg output
    if line.contains("time=") || line.contains("out_time=") {
        // Extract time value, format is typically HH:MM:SS.MS
        let time_start = if line.contains("time=") {
            line.find("time=")? + 5
        } else {
            line.find("out_time=")? + 9
        };
        
        let time_end = time_start + line[time_start..].find(' ')?;
        let time_str = &line[time_start..time_end].trim();
        
        // Parse time string to seconds
        let seconds = parse_time_to_seconds(time_str)?;
        
        // Look for duration in the line
        if let Some(duration) = line.find("duration=") {
            let duration_start = duration + 9;
            let duration_end = duration_start + line[duration_start..].find(' ')?;
            let duration_str = &line[duration_start..duration_end].trim();
            
            let total_seconds = parse_time_to_seconds(duration_str)?;
            if total_seconds > 0.0 {
                return Some(seconds / total_seconds);
            }
        }
        
        // If duration not found in this line, return None
    }
    
    None
}

/// Parse a time string in the format HH:MM:SS.MS to seconds
fn parse_time_to_seconds(time_str: &str) -> Option<f32> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    
    let hours: f32 = parts[0].parse().ok()?;
    let minutes: f32 = parts[1].parse().ok()?;
    let seconds: f32 = parts[2].parse().ok()?;
    
    Some(hours * 3600.0 + minutes * 60.0 + seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_time_to_seconds() {
        assert_eq!(parse_time_to_seconds("01:30:45"), Some(5445.0));
        assert_eq!(parse_time_to_seconds("00:05:30"), Some(330.0));
        assert_eq!(parse_time_to_seconds("00:00:10"), Some(10.0));
        assert_eq!(parse_time_to_seconds("invalid"), None);
    }
    
    #[test]
    fn test_parse_ffmpeg_progress() {
        // This one doesn't have a duration, so we can't calculate a progress percentage
        let line1 = "frame= 1000 fps=25 q=29.0 size=   12345kB time=00:01:30 bitrate=1000.0kbits/s speed=1.5x";
        let progress1 = parse_ffmpeg_progress(line1);
        assert_eq!(progress1, None); // Can't calculate progress without duration
        
        // This one has a duration, so we can calculate progress
        let line2 = "frame= 500 fps=25 q=29.0 size=   5000kB time=00:00:45 bitrate=1000.0kbits/s duration=00:03:00 speed=1.5x";
        let progress2 = parse_ffmpeg_progress(line2);
        assert!(progress2.is_some());
        assert_eq!(progress2, Some(0.25)); // 45 seconds out of 180 = 0.25
        
        // This line has no time info at all
        let line3 = "no progress info here";
        let progress3 = parse_ffmpeg_progress(line3);
        assert_eq!(progress3, None);
    }
    
    #[test]
    fn test_run_command_echo() {
        let mut cmd = Command::new("echo");
        cmd.arg("test");
        let result = run_command(&mut cmd);
        assert!(result.is_ok());
        if let Ok(output) = result {
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert_eq!(stdout.trim(), "test");
        }
    }
}