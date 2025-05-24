// ============================================================================
// drapto-core/src/external/ffmpeg_executor.rs
// ============================================================================
//
// FFMPEG EXECUTOR: FFmpeg Process Management and Abstraction
//
// This module provides abstractions for spawning and interacting with FFmpeg
// processes. It defines traits and implementations for executing FFmpeg commands
// and handling their events and lifecycle.
//
// KEY COMPONENTS:
// - FfmpegProcess: Trait representing an active FFmpeg process
// - FfmpegSpawner: Trait for creating new FFmpeg processes
// - SidecarFfmpegSpawner: Concrete implementation using ffmpeg-sidecar
//
// ARCHITECTURE:
// The module follows a trait-based design that allows for flexible process
// management and testing through dependency injection patterns.
//
// AI-ASSISTANT-INFO: FFmpeg process management and execution abstraction

use crate::error::{CoreResult, command_failed_error, command_start_error, command_wait_error};
use crate::hardware_accel::add_hardware_acceleration_to_command;
use crate::temp_files;
use ffmpeg_sidecar::child::FfmpegChild as SidecarChild;
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::FfmpegEvent;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;

// --- Direct FFmpeg Functions ---

/// Spawns an ffmpeg command and returns the child process.
pub fn spawn_ffmpeg(mut cmd: FfmpegCommand) -> CoreResult<SidecarChild> {
    cmd.spawn()
        .map_err(|e| command_start_error("ffmpeg", e))
}

/// Processes events from an ffmpeg child process.
pub fn handle_ffmpeg_events<F>(child: &mut SidecarChild, mut handler: F) -> CoreResult<()>
where
    F: FnMut(FfmpegEvent) -> CoreResult<()>,
{
    let iterator = child.iter().map_err(|e| {
        log::error!("Failed to get ffmpeg event iterator: {}", e);
        command_failed_error(
            "ffmpeg",
            ExitStatus::default(),
            e.to_string(),
        )
    })?;
    for event in iterator {
        handler(event)?;
    }
    Ok(())
}

/// Waits for an ffmpeg child process to complete.
pub fn wait_for_ffmpeg(child: &mut SidecarChild) -> CoreResult<ExitStatus> {
    child.wait()
        .map_err(|e| command_wait_error("ffmpeg", e))
}

// --- Grain Detection Specific Functions ---

/// Extracts a raw video sample using ffmpeg's -c copy.
///
/// Creates a temporary file within the specified `output_dir` using the temp_files module.
/// The file will be cleaned up when the `output_dir` (assumed to be a TempDir) is dropped.
pub fn extract_sample(
    input_path: &Path,
    start_time_secs: f64,
    duration_secs: u32,
    output_dir: &Path,
) -> CoreResult<PathBuf> {
    log::debug!(
        "Extracting sample: input={}, start={}, duration={}, out_dir={}",
        input_path.display(),
        start_time_secs,
        duration_secs,
        output_dir.display()
    );

    // Generate a unique filename for the sample within the output directory
    let output_path = temp_files::create_temp_file_path(output_dir, "raw_sample", "mkv");

    // Use mutable command object and sequential calls
    let mut cmd = FfmpegCommand::new();

    // Add hardware acceleration options BEFORE the input - no need to log status
    // Note: We use hardware acceleration for sample extraction but not for grain analysis
    add_hardware_acceleration_to_command(&mut cmd, true, false);

    cmd.input(input_path.to_string_lossy().as_ref());
    cmd.arg("-ss");
    cmd.arg(start_time_secs.to_string());
    cmd.arg("-t");
    cmd.arg(duration_secs.to_string());
    cmd.arg("-c"); // Use stream copy
    cmd.arg("copy");
    cmd.arg("-an"); // No audio
    cmd.arg("-sn"); // No subtitles
    cmd.arg("-map"); // Explicitly map video stream
    cmd.arg("0:v"); // Map video stream 0 (mandatory)
    cmd.arg("-map_metadata"); // Avoid copying global metadata
    cmd.arg("0"); // Map metadata from input 0
    cmd.output(output_path.to_string_lossy().as_ref());

    // Log the debug representation of the command struct
    log::debug!("Running sample extraction command: {:?}", cmd);

    // Spawn the command and wait for completion
    let mut child = spawn_ffmpeg(cmd)?;
    let status = wait_for_ffmpeg(&mut child)?;
    if !status.success() {
        log::error!("Sample extraction failed: {}", status);
        return Err(command_failed_error(
            "ffmpeg (sample extraction)",
            status,
            "Sample extraction process failed",
        ));
    }

    log::debug!(
        "Sample extracted successfully to: {}",
        output_path.display()
    );
    Ok(output_path) // Return the path to the created sample
}
