// ============================================================================
// drapto-core/src/processing/video.rs
// ============================================================================
//
// VIDEO PROCESSING: Main Video Encoding Orchestration
//
// This module houses the main video processing orchestration logic for the
// drapto-core library. It coordinates the entire encoding workflow, from
// analyzing video properties to executing ffmpeg and reporting results.
//
// KEY COMPONENTS:
// - process_videos: Main entry point for processing multiple video files
// - Dependency checking for required external tools
// - Video property detection and quality selection
// - Crop detection and grain analysis
// - ffmpeg execution and result handling
// - Notification sending for encoding events
//
// WORKFLOW:
// 1. Check for required external dependencies (ffmpeg, ffprobe)
// 2. For each video file:
//    a. Determine output path and check for existing files
//    b. Detect video properties (resolution, duration, etc.)
//    c. Select quality settings based on resolution
//    d. Perform crop detection if enabled
//    e. Analyze audio streams and determine bitrates
//    f. Perform grain analysis if denoising is enabled
//    g. Execute ffmpeg with the determined parameters
//    h. Handle results and send notifications
//
// AI-ASSISTANT-INFO: Main video encoding orchestration module

// ---- Internal crate imports ----
use crate::config::{CoreConfig, DEFAULT_CORE_QUALITY_HD, DEFAULT_CORE_QUALITY_SD, DEFAULT_CORE_QUALITY_UHD};
use crate::error::{CoreError, CoreResult};
use crate::external::check_dependency;
use crate::external::{FileMetadataProvider, FfmpegSpawner, FfprobeExecutor};
use crate::external::ffmpeg::{run_ffmpeg_encode, EncodeParams};
use crate::notifications::{NotificationType, NtfyNotificationSender};
use crate::processing::audio;
use crate::processing::detection::{self, grain_analysis};
use crate::progress::{ProgressCallback, ProgressEvent, LogLevel};
use crate::utils::{format_duration, get_file_size};
use crate::EncodeResult;
use colored::Colorize;

// ---- External crate imports ----
use log::{info, warn, error};

// ---- Standard library imports ----
use std::path::PathBuf;
use std::time::Instant;


// ============================================================================
// MAIN PROCESSING FUNCTION
// ============================================================================

/// Processes a list of video files according to the provided configuration.
///
/// This is the main entry point for the drapto-core library. It orchestrates
/// the entire encoding workflow, from analyzing video properties to executing
/// ffmpeg and reporting results.
///
/// The function is generic over the types that implement the required traits:
/// - `S`: FfmpegSpawner - For spawning ffmpeg processes
/// - `P`: FfprobeExecutor - For executing ffprobe commands
/// - `N`: NotificationSender - For sending notifications
/// - `M`: FileMetadataProvider - For file system operations
/// - `C`: ProgressCallback - For reporting progress (defaults to NullProgressCallback)
///
/// This design allows for dependency injection and easier testing.
///
/// # Arguments
///
/// * `spawner` - Implementation of FfmpegSpawner for executing ffmpeg
/// * `ffprobe_executor` - Implementation of FfprobeExecutor for executing ffprobe
/// * `notification_sender` - Implementation of NotificationSender for sending notifications
/// * `metadata_provider` - Implementation of FileMetadataProvider for file operations
/// * `config` - Core configuration containing encoding parameters and paths
/// * `files_to_process` - List of paths to the video files to process
/// * `target_filename_override` - Optional override for the output filename
/// * `progress_callback` - Implementation of ProgressCallback for reporting progress
///
/// # Returns
///
/// * `Ok(Vec<EncodeResult>)` - A vector of results for successfully processed files
/// * `Err(CoreError)` - If a critical error occurs during processing
///
/// # Examples
///
/// ```rust,no_run
/// use drapto_core::{CoreConfig, process_videos, EncodeResult};
/// use drapto_core::external::{SidecarSpawner, CrateFfprobeExecutor, StdFsMetadataProvider};
/// use drapto_core::notifications::NtfyNotificationSender;
/// use drapto_core::progress::NullProgressCallback;
/// use drapto_core::processing::detection::GrainLevel;
/// use std::path::PathBuf;
///
/// // Create dependencies
/// let spawner = SidecarSpawner;
/// let ffprobe_executor = CrateFfprobeExecutor::new();
/// let notification_sender = NtfyNotificationSender::new("https://ntfy.sh/my-topic").unwrap();
/// let metadata_provider = StdFsMetadataProvider;
/// let progress_callback = NullProgressCallback;
///
/// // Create configuration using the builder pattern
/// let config = drapto_core::config::CoreConfigBuilder::new()
///     .input_dir(PathBuf::from("/path/to/input"))
///     .output_dir(PathBuf::from("/path/to/output"))
///     .log_dir(PathBuf::from("/path/to/logs"))
///     .enable_denoise(true)
///     .default_encoder_preset(6)
///     .quality_sd(24)
///     .quality_hd(26)
///     .quality_uhd(28)
///     .default_crop_mode("auto")
///     .ntfy_topic("https://ntfy.sh/my-topic")
///     .film_grain_sample_duration(5)
///     .film_grain_knee_threshold(0.8)
///     .film_grain_fallback_level(GrainLevel::Baseline)
///     .film_grain_max_level(GrainLevel::Moderate)
///     .film_grain_refinement_points_count(5)
///     .build();
///
/// // Find files to process
/// let files = vec![PathBuf::from("/path/to/video.mkv")];
///
/// // Process videos
/// match process_videos(
///     &spawner,
///     &ffprobe_executor,
///     Some(&notification_sender),
///     &metadata_provider,
///     &config,
///     &files,
///     None,
///     &progress_callback,
/// ) {
///     Ok(results) => {
///         println!("Successfully processed {} files", results.len());
///         for result in results {
///             println!("File: {}, Duration: {}", result.filename, result.duration.as_secs());
///         }
///     },
///     Err(e) => {
///         eprintln!("Error processing videos: {}", e);
///     }
/// }
/// ```
pub fn process_videos<
    S: FfmpegSpawner,
    P: FfprobeExecutor,
    M: FileMetadataProvider,
    C: ProgressCallback
>(
    spawner: &S,
    ffprobe_executor: &P,
    notification_sender: Option<&NtfyNotificationSender>,
    metadata_provider: &M,
    config: &CoreConfig,
    files_to_process: &[PathBuf],
    target_filename_override: Option<PathBuf>,
    progress_callback: &C,
) -> CoreResult<Vec<EncodeResult>>
{
    // ========================================================================
    // STEP 1: CHECK DEPENDENCIES
    // ========================================================================

    // Verify that required external tools (ffmpeg and ffprobe) are available
    progress_callback.on_progress(ProgressEvent::LogMessage {
        message: "Checking for required external commands...".to_string(),
        level: LogLevel::Info,
    });

    // Check for ffmpeg
    let _ffmpeg_cmd_parts = check_dependency("ffmpeg")?;
    progress_callback.on_progress(ProgressEvent::LogMessage {
        message: "ffmpeg found.".to_string(),
        level: LogLevel::Info,
    });

    // Check for ffprobe
    let _ffprobe_cmd_parts = check_dependency("ffprobe")?;
    progress_callback.on_progress(ProgressEvent::LogMessage {
        message: "ffprobe found.".to_string(),
        level: LogLevel::Info,
    });

    progress_callback.on_progress(ProgressEvent::LogMessage {
        message: "External dependency check passed.".to_string(),
        level: LogLevel::Info,
    });

    // ========================================================================
    // STEP 2: GET SYSTEM INFORMATION
    // ========================================================================

    // Get the hostname for logging and notifications
    // This helps identify which machine is performing the encoding
    let hostname = hostname::get()
        .map(|s| s.into_string().unwrap_or_else(|_| "unknown-host-invalid-utf8".to_string()))
        .unwrap_or_else(|_| "unknown-host-error".to_string());

    progress_callback.on_progress(ProgressEvent::LogMessage {
        message: format!("Running on host: {}", hostname),
        level: LogLevel::Info,
    });

    // Report hardware acceleration capabilities
    // We don't use is_macos() directly anymore - this should be handled by the CLI
    let hw_accel_available = std::env::consts::OS == "macos";
    let hw_accel_type = if hw_accel_available { "VideoToolbox" } else { "None" };

    progress_callback.on_progress(ProgressEvent::HardwareAcceleration {
        available: hw_accel_available,
        acceleration_type: hw_accel_type.to_string(),
    });


    // Initialize the results vector to store successful encoding results
    let mut results: Vec<EncodeResult> = Vec::new();

    // ========================================================================
    // STEP 3: PROCESS EACH VIDEO FILE
    // ========================================================================

    for input_path in files_to_process {
        // Start timing the processing of this file
        let file_start_time = Instant::now();

        // Extract the filename for logging and output path construction
        let filename = input_path
            .file_name()
            .ok_or_else(|| CoreError::PathError(format!("Failed to get filename for {}", input_path.display())))?
            .to_string_lossy()
            .to_string();

        // Extract the filename without extension (not currently used but kept for future use)
        let _filename_noext = input_path
            .file_stem()
            .ok_or_else(|| CoreError::PathError(format!("Failed to get filename stem for {}", input_path.display())))?
            .to_string_lossy()
            .to_string();

        // ========================================================================
        // STEP 3.1: DETERMINE OUTPUT PATH
        // ========================================================================

        // Determine the output path based on configuration and target filename override
        let output_path = match &target_filename_override {
            // If a target filename is provided and we're only processing one file,
            // use it as the output filename in the output directory
            Some(target_filename) if files_to_process.len() == 1 => {
                config.output_dir.join(target_filename)
            }
            // Otherwise, use the original filename in the output directory
            _ => config.output_dir.join(&filename),
        };

        // ========================================================================
        // STEP 3.2: CHECK FOR EXISTING OUTPUT FILE
        // ========================================================================

        // Skip processing if the output file already exists to avoid overwriting
        if output_path.exists() {
            // Log the error with details
            let error_msg = format!(
                "ERROR: Output file already exists: {}. Skipping encode.",
                output_path.display()
            );
            error!("{}", error_msg);

            // Send a notification if notification_sender is provided
            if let Some(sender) = notification_sender {
                // Create a notification
                let notification = NotificationType::Custom {
                    title: "Drapto Encode Skipped".to_string(),
                    message: format!(
                        "[{hostname}]: Skipped encode for {filename}: Output file already exists at {output_display}",
                        hostname = hostname,
                        filename = filename,
                        output_display = output_path.display()
                    ),
                    priority: 3,
                };

                // Send the notification
                if let Err(e) = sender.send_notification(&notification) {
                    warn!("Failed to send notification for {}: {}", filename, e);
                }
            }

            // Add a separator in the log and skip to the next file
            info!("----------------------------------------");
            continue;
        }

        // Log the current file being processed
        info!("{} {}", "Processing:".cyan().bold(), filename.yellow());

        // ========================================================================
        // STEP 3.3: SEND START NOTIFICATION
        // ========================================================================

        // Send a notification that encoding is starting for this file
        if let Some(sender) = notification_sender {
            // Create a notification
            let notification = NotificationType::EncodeStart {
                input_path: input_path.to_path_buf(),
                output_path: output_path.clone(),
                hostname: hostname.clone(),
            };

            // Send the notification
            if let Err(e) = sender.send_notification(&notification) {
                warn!("Failed to send start notification for {}: {}", filename, e);
            }
        }

        // ========================================================================
        // STEP 3.4: GET VIDEO PROPERTIES
        // ========================================================================

        // Analyze the video file to get its properties (resolution, duration, etc.)
        let video_props = match ffprobe_executor.get_video_properties(input_path) {
            Ok(props) => props,
            Err(e) => {
                // Log the error and skip this file
                error!("Failed to get video properties for {}: {}. Skipping file.", filename, e);

                // Send an error notification if notification_sender is provided
                if let Some(sender) = notification_sender {
                    // Create a notification
                    let notification = NotificationType::EncodeError {
                        input_path: input_path.to_path_buf(),
                        message: "Failed to get video properties".to_string(),
                        hostname: hostname.clone(),
                    };

                    // Send the notification
                    if let Err(e) = sender.send_notification(&notification) {
                        warn!("Failed to send error notification for {}: {}", filename, e);
                    }
                }

                // Add a separator in the log and skip to the next file
                info!("----------------------------------------");
                continue;
            }
        };

        // Extract key properties for later use
        let video_width = video_props.width;
        let duration_secs = video_props.duration_secs;

        // ========================================================================
        // STEP 3.5: DETERMINE QUALITY SETTINGS
        // ========================================================================

        // Define resolution thresholds for quality selection
        const UHD_WIDTH_THRESHOLD: u32 = 3840; // 4K and above
        const HD_WIDTH_THRESHOLD: u32 = 1920;  // 1080p and above

        // Select quality (CRF) based on video resolution
        // Lower CRF values = higher quality but larger files
        let quality = if video_width >= UHD_WIDTH_THRESHOLD {
            // UHD (4K) quality setting
            config.quality_uhd.unwrap_or(DEFAULT_CORE_QUALITY_UHD)
        } else if video_width >= HD_WIDTH_THRESHOLD {
            // HD (1080p) quality setting
            config.quality_hd.unwrap_or(DEFAULT_CORE_QUALITY_HD)
        } else {
            // SD (below 1080p) quality setting
            config.quality_sd.unwrap_or(DEFAULT_CORE_QUALITY_SD)
        };

        // Determine the category label for logging
        let category = if video_width >= UHD_WIDTH_THRESHOLD {
            "UHD"
        } else if video_width >= HD_WIDTH_THRESHOLD {
            "HD"
        } else {
            "SD"
        };

        // Log the detected resolution and selected quality
        info!(
            "Detected video width: {} ({}) - CRF set to {}",
            video_width.to_string().green(),
            category.green(),
            quality.to_string().green().bold()
        );

        // ========================================================================
        // STEP 3.6: PERFORM CROP DETECTION
        // ========================================================================

        // Check if crop detection is disabled in the configuration
        let disable_crop = config.default_crop_mode.as_deref() == Some("off");

        // Detect black bars in the video and generate crop parameters if needed
        let (crop_filter_opt, _is_hdr) = match detection::detect_crop(spawner, input_path, &video_props, disable_crop) {
            Ok(result) => result,
            Err(e) => {
                // Log warning and proceed without cropping
                warn!("Crop detection failed for {}: {}. Proceeding without cropping.", filename, e);
                // Note: detect_crop currently returns Ok(None) for failures, but this handles
                // potential future changes where it might return Err
                (None, false)
            }
        };

        // ========================================================================
        // STEP 3.7: ANALYZE AUDIO STREAMS
        // ========================================================================

        // Log information about audio streams (channels, bitrates)
        let _ = audio::log_audio_info(ffprobe_executor, input_path, progress_callback);

        // Get audio channel information for encoding
        let audio_channels = match ffprobe_executor.get_audio_channels(input_path) {
             Ok(channels) => channels,
             Err(e) => {
                 // Log warning and continue with empty channel list
                 warn!("Error getting audio channels for ffmpeg command build: {}. Using empty list.", e);
                 vec![] // Empty vector as fallback
             }
         };

        // ========================================================================
        // STEP 3.8: PREPARE ENCODING PARAMETERS
        // ========================================================================

        // Determine encoder preset (speed vs quality tradeoff, lower = better quality but slower)
        let preset_value = config.preset.or(config.default_encoder_preset).unwrap_or(6);

        // Build initial encoding parameters (without denoising settings)
        let mut initial_encode_params = EncodeParams {
            input_path: input_path.to_path_buf(),
            output_path: output_path.clone(),
            quality: quality.into(),
            preset: preset_value,
            use_hw_decode: true, // Enable hardware decoding by default
            crop_filter: crop_filter_opt.clone(),
            audio_channels: audio_channels.clone(),
            duration: duration_secs,
            hqdn3d_params: None, // Will be determined by grain analysis
        };

        // ========================================================================
        // STEP 3.9: PERFORM GRAIN ANALYSIS
        // ========================================================================

        // Analyze grain/noise in the video to determine optimal denoising parameters
        let final_hqdn3d_params_result = if config.enable_denoise {
            // Perform grain analysis if denoising is enabled
            progress_callback.on_progress(ProgressEvent::LogMessage {
                message: "Grain detection enabled, analyzing video using relative sample comparison...".to_string(),
                level: LogLevel::Info,
            });
            grain_analysis::analyze_grain(
                input_path,
                config,
                &initial_encode_params,
                duration_secs,
                spawner,
                metadata_provider,
                progress_callback
            )
        } else {
            // Skip grain analysis if denoising is disabled
            info!("Denoising disabled via config.");
            Ok(None)
        };

        // ========================================================================
        // STEP 3.10: PROCESS GRAIN ANALYSIS RESULTS
        // ========================================================================

        // Process the results of grain analysis and determine final denoising parameters
        let final_hqdn3d_params: Option<String> = match final_hqdn3d_params_result {
             // Case 1: Grain analysis completed successfully with a result
             Ok(Some(result)) => {
                 // Convert the detected grain level to hqdn3d filter parameters
                 let params_opt = grain_analysis::determine_hqdn3d_params(result.detected_level);

                 // Log the results
                 info!(
                     "Grain analysis result: {:?}, applying filter: {}",
                     result.detected_level,
                     params_opt.as_deref().unwrap_or("No parameters") // No parameters means no denoising needed (Baseline)
                 );

                 // Return the parameters (or None for Baseline videos)
                 params_opt
             }
             // Case 2: Grain analysis was skipped or produced no result
             Ok(None) => {
                 info!("Grain analysis skipped or did not produce a result. Proceeding without denoising.");
                 None
             }
             // Case 3: Grain analysis failed with an error
             Err(e) => {
                 // Log the error and skip this file
                 error!("Grain analysis failed critically: {}. Skipping file.", e);

                 // Send an error notification if notification_sender is provided
                 if let Some(sender) = notification_sender {
                     // Create a notification
                     let notification = NotificationType::EncodeError {
                         input_path: input_path.to_path_buf(),
                         message: "Grain analysis failed".to_string(),
                         hostname: hostname.clone(),
                     };

                     // Send the notification
                     if let Err(e) = sender.send_notification(&notification) {
                         warn!("Failed to send error notification for {}: {}", filename, e);
                     }
                 }

                 // Add a separator in the log and skip to the next file
                 info!("----------------------------------------");
                 continue; // Skip this file
             }
         };

        // ========================================================================
        // STEP 3.11: FINALIZE ENCODING PARAMETERS
        // ========================================================================

        // Update the initial parameters with the determined denoising filter
        initial_encode_params.hqdn3d_params = final_hqdn3d_params;

        // Create the final set of parameters for the main encode
        let final_encode_params = initial_encode_params;

        // ========================================================================
        // STEP 3.12: EXECUTE FFMPEG ENCODING
        // ========================================================================

        // Run the ffmpeg encoding process with the finalized parameters
        // Note: The run_ffmpeg_encode function handles its own start logging
        let encode_result = run_ffmpeg_encode(
            spawner,
            &final_encode_params,
            false, // disable_audio: Keep audio in the output
            false, // is_grain_analysis_sample: This is the main encode, not a sample
            None,  // grain_level_being_tested: Not applicable for final encode
            progress_callback, // Pass the progress callback
        );

        // ========================================================================
        // STEP 3.13: HANDLE ENCODING RESULTS
        // ========================================================================

        match encode_result {
            // Case 1: Encoding completed successfully
            Ok(()) => {
                // Calculate elapsed time for this file
                let file_elapsed_time = file_start_time.elapsed();

                // Get input and output file sizes for comparison
                let input_size = get_file_size(input_path)?;
                let output_size = get_file_size(&output_path)?;

                // Add this file to the successful results
                results.push(EncodeResult {
                    filename: filename.clone(),
                    duration: file_elapsed_time,
                    input_size,
                    output_size,
                });

                // Log completion message
                let completion_log_msg = format!("Completed: {} in {}",
                    filename,
                    format_duration(file_elapsed_time)
                );
                info!("{}", completion_log_msg);

                // ========================================================================
                // STEP 3.14: SEND SUCCESS NOTIFICATION
                // ========================================================================

                // Send a success notification if notification_sender is provided
                if let Some(sender) = notification_sender {
                    // Create a notification
                    let notification = NotificationType::EncodeComplete {
                        input_path: input_path.to_path_buf(),
                        output_path: output_path.clone(),
                        input_size,
                        output_size,
                        duration: file_elapsed_time,
                        hostname: hostname.clone(),
                    };

                    // Send the notification
                    if let Err(e) = sender.send_notification(&notification) {
                        warn!("Failed to send success notification for {}: {}", filename, e);
                    }
                }
            }

            // Case 2: No streams found in the input file
            Err(CoreError::NoStreamsFound(path)) => {
                // Log the specific error
                warn!(
                    "Skipping encode for {}: FFmpeg reported no processable streams found in '{}'.",
                    filename, path
                );

                // Send a notification if notification_sender is provided
                if let Some(sender) = notification_sender {
                    // Create a notification
                    let notification = NotificationType::Custom {
                        title: "Drapto Encode Skipped".to_string(),
                        message: format!(
                            "[{hostname}]: Skipped encode for {filename}: No streams found.",
                            hostname = hostname,
                            filename = filename
                        ),
                        priority: 3,
                    };

                    // Send the notification
                    if let Err(e) = sender.send_notification(&notification) {
                        warn!("Failed to send skip notification for {}: {}", filename, e);
                    }
                }
                // No `continue` here, just let the loop proceed after logging/notifying
            }

            // Case 3: Generic encoding failure
            Err(e) => {
                // Log the error
                error!(
                    "ffmpeg encode failed for {}: {}. Check logs for details.",
                    filename, e
                );

                // Send an error notification if notification_sender is provided
                if let Some(sender) = notification_sender {
                    // Create a notification
                    let notification = NotificationType::EncodeError {
                        input_path: input_path.to_path_buf(),
                        message: format!("ffmpeg failed: {}", e),
                        hostname: hostname.clone(),
                    };

                    // Send the notification
                    if let Err(err) = sender.send_notification(&notification) {
                        warn!("Failed to send error notification for {}: {}", filename, err);
                    }
                }
                // No `continue` here either for general errors, loop proceeds
            }
        }

        // Add a separator in the log before processing the next file
        info!("----------------------------------------");

    } // End of loop through files

    // ========================================================================
    // STEP 4: RETURN RESULTS
    // ========================================================================

    // Return the list of successfully processed files
    Ok(results)
}