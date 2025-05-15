// drapto-core/src/external/ffmpeg_executor.rs

use crate::error::{CoreResult, command_start_error, command_wait_error, command_failed_error};
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::FfmpegEvent;
use ffmpeg_sidecar::child::FfmpegChild as SidecarChild;
use crate::hardware_accel::add_hardware_acceleration_to_command;
use crate::temp_files;
use std::process::ExitStatus;
use std::path::{Path, PathBuf};

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
    // Signature takes cmd by value, matching ffmpeg-sidecar's spawn(self)
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
            command_failed_error(
                "ffmpeg (sidecar - get iter)",
                ExitStatus::default(), // Placeholder status
                e.to_string()
            )
        })?;
        for event in iterator {
            handler(event)?;
        }
        Ok(())
    }

    fn wait(&mut self) -> CoreResult<ExitStatus> {
        self.0.wait().map_err(|e| command_wait_error("ffmpeg (sidecar)", e))
    }
}

/// Concrete implementation of `FfmpegSpawner` using `ffmpeg-sidecar`.
#[derive(Debug, Clone, Default)] // Added Default derive
pub struct SidecarSpawner;

impl FfmpegSpawner for SidecarSpawner {
    type Process = SidecarProcess;

    // Add mut back to cmd parameter in the IMPL only, based on E0596 hint
    // Trait still takes `cmd: FfmpegCommand`
    fn spawn(&self, mut cmd: FfmpegCommand) -> CoreResult<Self::Process> {
        // spawn consumes cmd, requires mutability if called like cmd.spawn()
        cmd.spawn().map(SidecarProcess)
                 .map_err(|e| command_start_error("ffmpeg (sidecar)", e))
    }
}


// --- Grain Detection Specific Functions ---

/// Extracts a raw video sample using ffmpeg's -c copy.
///
/// Creates a temporary file within the specified `output_dir` using the temp_files module.
/// The file will be cleaned up when the `output_dir` (assumed to be a TempDir) is dropped.
pub fn extract_sample<S: FfmpegSpawner>( // Added generic parameter S
    spawner: &S, // Added spawner argument
    input_path: &Path,
    start_time_secs: f64,
    duration_secs: u32,
    output_dir: &Path,
) -> CoreResult<PathBuf> {
    log::debug!(
        "Extracting sample: input={}, start={}, duration={}, out_dir={}",
        input_path.display(), start_time_secs, duration_secs, output_dir.display()
   );

   // Generate a unique filename for the sample within the output directory
   let output_path = temp_files::create_temp_file_path(output_dir, "raw_sample", "mkv");


   // Use mutable command object and sequential calls
    let mut cmd = FfmpegCommand::new();

    // Add hardware acceleration options BEFORE the input
    // Note: We use hardware acceleration for sample extraction but not for grain analysis
    let hw_accel_added = add_hardware_acceleration_to_command(&mut cmd, true, false);

    if hw_accel_added {
        log::debug!("Using VideoToolbox hardware decoding for sample extraction");
    }

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

    // Spawn the command using the provided spawner and wait for completion
    let status = spawner.spawn(cmd)?.wait()?; // Use spawner here
    if !status.success() {
        log::error!("Sample extraction failed: {}", status);
        return Err(command_failed_error(
            "ffmpeg (sample extraction)",
            status,
            "Sample extraction process failed"
        ));
   }

   // The file existence check is redundant since FFmpeg would have failed if it couldn't create the file
   // FFmpeg's success status is sufficient to confirm the operation completed correctly

   log::debug!("Sample extracted successfully to: {}", output_path.display());
   Ok(output_path) // Return the path to the created sample
}


