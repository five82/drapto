// drapto-core/src/external/ffmpeg_executor.rs

use crate::error::{CoreError, CoreResult};
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::FfmpegEvent;
use ffmpeg_sidecar::child::FfmpegChild as SidecarChild;
use std::process::ExitStatus;

// --- FFmpeg Execution Abstraction ---

/// Trait representing an active ffmpeg process instance.
pub trait FfmpegProcess {
    /// Processes events from the running command using a provided handler closure.
    fn handle_events<F>(&mut self, handler: F) -> CoreResult<()>
        where F: FnMut(FfmpegEvent) -> CoreResult<()>;

    /// Waits for the command to complete and returns its exit status.
    fn wait(&mut self) -> CoreResult<ExitStatus>;
}

/// Trait representing something that can spawn an FfmpegProcess.
pub trait FfmpegSpawner {
    type Process: FfmpegProcess;
    /// Spawns the ffmpeg command.
    fn spawn(&self, cmd: FfmpegCommand) -> CoreResult<Self::Process>;
}

// --- Concrete Implementation using ffmpeg-sidecar ---

/// Wrapper around `ffmpeg_sidecar::child::Child` implementing `FfmpegProcess`.
pub struct SidecarProcess(SidecarChild); // Use the imported alias

impl FfmpegProcess for SidecarProcess {
    fn handle_events<F>(&mut self, mut handler: F) -> CoreResult<()>
        where F: FnMut(FfmpegEvent) -> CoreResult<()>
    {
        let iterator = self.0.iter().map_err(|e| {
            log::error!("Failed to get ffmpeg event iterator: {}", e);
            CoreError::CommandFailed(
                "ffmpeg (sidecar - get iter)".to_string(),
                ExitStatus::default(),
                e.to_string(),
            )
        })?;
        for event in iterator {
            handler(event)?;
        }
        Ok(())
    }

    fn wait(&mut self) -> CoreResult<ExitStatus> {
        self.0.wait().map_err(|e| CoreError::CommandWait("ffmpeg (sidecar)".to_string(), e))
    }
}

/// Concrete implementation of `FfmpegSpawner` using `ffmpeg-sidecar`.
pub struct SidecarSpawner;

impl FfmpegSpawner for SidecarSpawner {
    type Process = SidecarProcess;

    fn spawn(&self, mut cmd: FfmpegCommand) -> CoreResult<Self::Process> {
        cmd.spawn().map(SidecarProcess)
                 .map_err(|e| CoreError::CommandStart("ffmpeg (sidecar)".to_string(), e))
    }
}