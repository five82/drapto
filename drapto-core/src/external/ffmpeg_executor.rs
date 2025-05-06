// drapto-core/src/external/ffmpeg_executor.rs

use crate::error::{CoreError, CoreResult};
use crate::external::ffmpeg::{build_ffmpeg_args, EncodeParams}; // Import build_ffmpeg_args and EncodeParams
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::FfmpegEvent;
use ffmpeg_sidecar::child::FfmpegChild as SidecarChild;
use std::process::ExitStatus;
use std::path::{Path, PathBuf};
use std::fs; // Keep fs for metadata in encode_sample_for_grain_test
use tempfile::Builder as TempFileBuilder; // Keep for encode_sample_for_grain_test
use rand::{thread_rng, Rng}; // Add Rng
use rand::distributions::Alphanumeric; // Add Alphanumeric for filename generation

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
            CoreError::CommandFailed(
                "ffmpeg (sidecar - get iter)".to_string(),
                ExitStatus::default(), // Placeholder status
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
#[derive(Debug, Clone, Default)] // Added Default derive
pub struct SidecarSpawner;

impl FfmpegSpawner for SidecarSpawner {
    type Process = SidecarProcess;

    // Add mut back to cmd parameter in the IMPL only, based on E0596 hint
    // Trait still takes `cmd: FfmpegCommand`
    fn spawn(&self, mut cmd: FfmpegCommand) -> CoreResult<Self::Process> {
        // spawn consumes cmd, requires mutability if called like cmd.spawn()
        cmd.spawn().map(SidecarProcess)
                 .map_err(|e| CoreError::CommandStart("ffmpeg (sidecar)".to_string(), e))
    }
}


// --- Grain Detection Specific Functions ---

/// Extracts a raw video sample using ffmpeg's -c copy.
///
/// Creates a temporary file within the specified `output_dir`.
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
   let random_suffix: String = thread_rng()
       .sample_iter(&Alphanumeric)
       .take(6) // 6 random characters
       .map(char::from)
       .collect();
   let filename = format!("raw_sample_{}.mkv", random_suffix);
   let output_path = output_dir.join(filename);


   // Use mutable command object and sequential calls
    let mut cmd = FfmpegCommand::new();
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
        return Err(CoreError::CommandFailed(
            "ffmpeg (sample extraction)".to_string(),
            status,
           "Sample extraction process failed".to_string()));
   }

   // Add check: Verify the file actually exists after ffmpeg command succeeded
   if !output_path.exists() {
       log::error!("Sample extraction command succeeded (status 0), but output file {} was not found!", output_path.display());
       return Err(CoreError::OperationFailed(format!("Sample extraction succeeded but output file not found: {}", output_path.display())));
   }

   log::debug!("Sample extracted successfully to: {}", output_path.display());
   Ok(output_path) // Return the path to the created sample
}


/// Encodes a given sample file using base encode parameters and an optional hqdn3d override,
/// returning the output file size in bytes.
///
/// Creates a temporary output file within the specified `output_dir`.
/// The caller is responsible for cleaning up the input `sample_path`.
// This function is no longer used directly by analyze_grain, but might be useful for other tests.
// It still uses direct ffmpeg execution, not the spawner.
#[allow(dead_code)]
pub fn encode_sample_for_grain_test(
    sample_path: &Path,
    base_encode_params: &EncodeParams, // Use base params struct
    hqdn3d_override: Option<&str>,     // Accept override
    output_dir: &Path,
) -> CoreResult<u64> {
    let hqdn3d_desc = hqdn3d_override.unwrap_or("None");
    log::debug!(
        "Encoding sample for grain test: sample={}, hqdn3d_override={}, out_dir={}",
        sample_path.display(), hqdn3d_desc, output_dir.display()
    );

    // Create a temporary file path for the encoded output
    let temp_encoded_file = TempFileBuilder::new()
        .prefix(&format!("encoded_sample_hqdn3d_{}_", hqdn3d_desc.replace([':', '=', ','], "_"))) // Sanitize override for filename
        .suffix(".mkv") // Match container if needed
        .tempfile_in(output_dir)
        .map_err(CoreError::Io)?;

    let encoded_output_path = temp_encoded_file.path().to_path_buf();

    // --- Build FFmpeg arguments using the override ---
    let ffmpeg_args = match build_ffmpeg_args(
        base_encode_params,
        hqdn3d_override,
        true, /* disable_audio */
        true, /* is_grain_analysis_sample */
    ) {
        Ok(args) => args,
        Err(e) => {
            log::error!("Failed to build ffmpeg args for sample encoding (hqdn3d={}): {}", hqdn3d_desc, e);
            return Err(e);
        }
    };
    // --- End Argument Building ---

    // Use mutable command object and sequential calls
    let mut cmd = FfmpegCommand::new();
    cmd.input(sample_path.to_string_lossy().as_ref());
    cmd.args(ffmpeg_args.iter().map(|s| s.as_str())); // Add built args
    cmd.output(encoded_output_path.to_string_lossy().as_ref());

    // Log the debug representation of the command struct
    log::debug!("Running sample encoding command (hqdn3d={}): {:?}", hqdn3d_desc, cmd);

    // Spawn the command (consumes cmd) and wait for completion
    let status = cmd.spawn()?.wait()?; // Still uses direct spawn
    if !status.success() {
        log::error!("Sample encoding failed (hqdn3d={}): {}", hqdn3d_desc, status);
        return Err(CoreError::CommandFailed(
            format!("ffmpeg (sample encoding hqdn3d={})", hqdn3d_desc),
            status,
            "Sample encoding process failed".to_string()));
    }

    // Get file size
    let metadata = fs::metadata(&encoded_output_path).map_err(CoreError::Io)?;
    let file_size = metadata.len();

    log::debug!(
        "Sample encoded successfully (hqdn3d={}) to: {} (Size: {} bytes)",
        hqdn3d_desc, encoded_output_path.display(), file_size
    );

    // Clean up the encoded file explicitly, as we only need its size
    let _ = fs::remove_file(&encoded_output_path);

    Ok(file_size)
}