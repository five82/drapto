// ============================================================================
// drapto-core/src/external/ffprobe_executor.rs
// ============================================================================
//
// FFPROBE INTEGRATION: Video Analysis and Media Information Extraction
//
// This module provides abstractions for executing ffprobe commands to analyze
// media files and extract properties such as dimensions, duration, audio channels,
// and bitplane noise for grain analysis.
//
// KEY COMPONENTS:
// - FfprobeExecutor trait: Defines the interface for ffprobe operations
// - CrateFfprobeExecutor: Concrete implementation using the ffprobe crate
// - MediaInfo: Structure containing extracted media information
//
// DESIGN PHILOSOPHY:
// The module follows the dependency injection pattern with a trait-based
// interface, allowing for testing and flexibility in implementation.
//
// AI-ASSISTANT-INFO: FFprobe execution for media analysis and information extraction
use crate::error::{CoreError, CoreResult, command_failed_error, command_start_error};
use crate::processing::detection::properties::VideoProperties;
use ffprobe::{FfProbeError, ffprobe};
use std::path::Path;
use std::process::Command;

/// Struct containing media information.
#[derive(Debug, Default, Clone)]
pub struct MediaInfo {
    /// Duration of the media in seconds
    pub duration: Option<f64>,
    /// Width of the video stream
    pub width: Option<i64>,
    /// Height of the video stream
    pub height: Option<i64>,
}

// --- Ffprobe Execution Abstraction ---

/// Trait for executing ffprobe commands.
pub trait FfprobeExecutor {
    /// Gets audio channel counts for a given input file.
    fn get_audio_channels(&self, input_path: &Path) -> CoreResult<Vec<u32>>;
    /// Gets video properties (dimensions, duration, color info) for a given input file.
    fn get_video_properties(&self, input_path: &Path) -> CoreResult<VideoProperties>;
    /// Runs ffprobe with bitplanenoise filter (bitplane 1, luma plane 0) to analyze grain, sampling based on duration.
    fn run_ffprobe_bitplanenoise(
        &self,
        input_path: &Path,
        duration_secs: f64,
    ) -> CoreResult<Vec<f32>>; // Changed return type
    /// Gets media information for a given input file.
    fn get_media_info(&self, input_path: &Path) -> CoreResult<MediaInfo>;
}

// --- New Implementation using `ffprobe` crate (and Command for specific tasks) ---

/// Concrete implementation using the `ffprobe` crate.
#[derive(Debug, Clone, Default)] // Add derive for potential future use and consistency
pub struct CrateFfprobeExecutor;

impl CrateFfprobeExecutor {
    pub fn new() -> Self {
        Self
    }

    // --- Bitplane Noise Analysis Implementation ---
    fn run_ffprobe_bitplanenoise_impl(
        &self,
        input_path: &Path,
        duration_secs: f64,
    ) -> CoreResult<Vec<f32>> {
        // Changed return type
        let cmd_name = "ffprobe";
        const TARGET_SAMPLES: f64 = 10.0; // Aim for roughly 10 samples

        // Calculate sampling interval, ensuring it's at least a small value to avoid division by zero
        // and handle very short videos reasonably (e.g., sample at least the start).
        let sample_interval = if duration_secs > 0.0 {
            (duration_secs / TARGET_SAMPLES).max(0.1) // Sample at least every 0.1s if duration is tiny
        } else {
            1.0 // Default interval if duration is zero or negative (shouldn't happen)
        };

        let input_path_str = input_path.to_str().ok_or_else(|| {
            CoreError::PathError(format!(
                // Changed to PathError
                "Input path is not valid UTF-8: {}",
                input_path.display()
            ))
        })?;

        // Construct the filter graph string carefully, escaping the filename
        // Basic escaping for common shell characters, might need refinement
        let escaped_input_path = input_path_str.replace('\'', "'\\''"); // Handle single quotes
        // Use time-based selection: select the first frame, then frames at least 'sample_interval' seconds apart.
        // Use default bitplanenoise (bitplane=1 for luma plane 0).
        let filter_graph = format!(
            "movie='{}',select='isnan(prev_selected_t)+gte(t-prev_selected_t\\,{:.3})',bitplanenoise,metadata=print", // Removed explicit bitplane settings
            escaped_input_path, // Use escaped path
            sample_interval     // Use calculated interval
        );

        log::debug!(
            "Running {} for bitplanenoise on: {}",
            cmd_name,
            input_path.display()
        );
        log::trace!("Filter graph: {}", filter_graph); // Log the filter graph for debugging

        let output = Command::new(cmd_name)
            .args([
                "-v",
                "error", // Use error level to see potential ffprobe errors
                "-f",
                "lavfi",
                "-i",
                &filter_graph, // Use -i for input filtergraph
                "-show_entries",
                // Request the correct metadata tag for luma plane (0), bitplane 1
                "frame_tags=lavfi.bitplanenoise.0.1",
                "-of",
                "csv=p=0", // Output format: CSV, no header (print_section=0)
            ])
            .output() // Use output() for simplicity first
            .map_err(|e| command_start_error(cmd_name, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            log::error!(
                "{} bitplanenoise failed for {}. Status: {}. Stderr: {}",
                cmd_name,
                input_path.display(),
                output.status,
                stderr
            );
            return Err(command_failed_error(
                format!("{} bitplanenoise", cmd_name),
                output.status,
                stderr,
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        log::trace!(
            "{} bitplanenoise stdout for {}:\n{}",
            cmd_name,
            input_path.display(),
            stdout
        );

        let mut results: Vec<f32> = Vec::new(); // Changed result type
        for line in stdout.lines() {
            let trimmed_line = line.trim();
            if trimmed_line.is_empty() {
                continue;
            }
            // Remove trailing comma if present before parsing
            let value_str = trimmed_line.strip_suffix(',').unwrap_or(trimmed_line);
            // Expecting a single float value per line now
            match value_str.parse::<f32>() {
                Ok(n1) => results.push(n1), // Push the single value
                Err(_) => log::warn!(
                    "Failed to parse bitplanenoise value as f32: '{}' (original line: '{}') for {}",
                    value_str,
                    trimmed_line,
                    input_path.display()
                ),
            }
        }

        if results.is_empty() {
            // Log the stdout content at TRACE level when no results are parsed
            log::trace!(
                "{} bitplanenoise analysis produced no valid results for {}. Stdout content was:\n---\n{}\n---",
                cmd_name,
                input_path.display(),
                stdout
            );
        }
        Ok(results)
    }
}

impl FfprobeExecutor for CrateFfprobeExecutor {
    fn get_audio_channels(&self, input_path: &Path) -> CoreResult<Vec<u32>> {
        log::debug!(
            "Running ffprobe (via crate) for audio channels on: {}",
            input_path.display()
        );
        match ffprobe(input_path) {
            Ok(metadata) => {
                let channels: Vec<u32> = metadata
                    .streams
                    .iter()
                    .filter(|s| s.codec_type.as_deref() == Some("audio"))
                    .filter_map(|s| s.channels) // filter_map handles Option<i64> in ffprobe v0.3.3
                    .map(|c| {
                        // Cast i64 to u32
                        if c < 0 {
                            log::warn!(
                                "Negative channel count ({}) found for {}, treating as 0",
                                c,
                                input_path.display()
                            );
                            0u32 // Or handle as error? Unlikely scenario.
                        } else {
                            c as u32
                        }
                    })
                    .collect();
                if channels.is_empty() {
                    log::warn!(
                        "No audio streams found by ffprobe for {}",
                        input_path.display()
                    );
                }
                Ok(channels)
            }
            Err(err) => {
                log::error!(
                    "ffprobe (crate) failed for audio channels on {}: {:?}",
                    input_path.display(),
                    err
                );
                Err(map_ffprobe_error(err, "audio channels"))
            }
        }
    }

    fn get_video_properties(&self, input_path: &Path) -> CoreResult<VideoProperties> {
        log::debug!(
            "Running ffprobe (via crate) for video properties on: {}",
            input_path.display()
        );
        match ffprobe(input_path) {
            Ok(metadata) => {
                let duration_secs = metadata
                    .format
                    .duration
                    .as_deref()
                    .and_then(|d| d.parse::<f64>().ok())
                    .ok_or_else(|| {
                        CoreError::FfprobeParse(format!(
                            "Failed to parse duration from format for {}",
                            input_path.display()
                        ))
                    })?;

                let video_stream = metadata
                    .streams
                    .iter()
                    .find(|s| s.codec_type.as_deref() == Some("video"))
                    .ok_or_else(|| {
                        CoreError::VideoInfoError(format!(
                            "No video stream found in {}",
                            input_path.display()
                        ))
                    })?;

                // Use i64 from ffprobe crate and cast carefully
                let width = video_stream.width.ok_or_else(|| {
                    CoreError::VideoInfoError(format!(
                        "Video stream missing width in {}",
                        input_path.display()
                    ))
                })?;
                let height = video_stream.height.ok_or_else(|| {
                    CoreError::VideoInfoError(format!(
                        "Video stream missing height in {}",
                        input_path.display()
                    ))
                })?;

                // Ensure non-negative before casting
                if width < 0 || height < 0 {
                    return Err(CoreError::VideoInfoError(format!(
                        "Invalid dimensions (negative) found in {}: width={}, height={}",
                        input_path.display(),
                        width,
                        height
                    )));
                }

                Ok(VideoProperties {
                    width: width as u32,   // Cast after check
                    height: height as u32, // Cast after check
                    duration_secs,
                    color_space: video_stream.color_space.clone(),
                    // color_primaries and color_transfer removed as they are not in ffprobe v0.3.3 Stream struct
                })
            }
            Err(err) => {
                log::error!(
                    "ffprobe (crate) failed for video properties on {}: {:?}",
                    input_path.display(),
                    err
                );
                Err(map_ffprobe_error(err, "video properties"))
            }
        }
    }

    // Implement the trait method by calling the internal implementation
    fn run_ffprobe_bitplanenoise(
        &self,
        input_path: &Path,
        duration_secs: f64,
    ) -> CoreResult<Vec<f32>> {
        // Changed return type
        self.run_ffprobe_bitplanenoise_impl(input_path, duration_secs)
    }

    // Implement the new get_media_info trait method
    fn get_media_info(&self, input_path: &Path) -> CoreResult<MediaInfo> {
        // Implement directly to avoid infinite recursion
        log::debug!(
            "Running ffprobe (via crate) for media info on: {}",
            input_path.display()
        );
        match ffprobe(input_path) {
            Ok(metadata) => {
                // Get duration from format
                let duration = metadata
                    .format
                    .duration
                    .as_deref()
                    .and_then(|d| d.parse::<f64>().ok());

                let mut info = MediaInfo {
                    duration,
                    ..Default::default()
                };

                // Find video stream
                if let Some(video_stream) = metadata
                    .streams
                    .iter()
                    .find(|s| s.codec_type.as_deref() == Some("video"))
                {
                    // Get width and height
                    info.width = video_stream.width;
                    info.height = video_stream.height;
                }

                Ok(info)
            }
            Err(err) => {
                log::warn!("Failed to get media info: {:?}", err);
                Err(map_ffprobe_error(err, "media info"))
            }
        }
    }
}

// Helper function to map ffprobe crate errors to CoreError
fn map_ffprobe_error(err: FfProbeError, context: &str) -> CoreError {
    match err {
        FfProbeError::Io(io_err) => {
            // Use CommandStart for all IO errors when executing ffprobe
            command_start_error(format!("ffprobe ({})", context), io_err)
        }
        // Adjusted for ffprobe v0.3.3 FfProbeError::Status variant
        FfProbeError::Status(output) => {
            // Pass the actual ExitStatus (output.status) instead of a string representation
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            command_failed_error(format!("ffprobe ({})", context), output.status, stderr)
        }
        // Adjusted for ffprobe v0.3.3 FfProbeError::Deserialize variant (assuming name)
        FfProbeError::Deserialize(err) => CoreError::JsonParseError(format!(
            "ffprobe {} output deserialization: {}",
            context, err
        )),
        // Add wildcard arm for non-exhaustive enum
        _ => CoreError::FfprobeParse(format!(
            "Unknown ffprobe error during {}: {:?}",
            context, err
        )),
    }
}
