// ============================================================================
// drapto-core/src/processing/detection/crop_analysis.rs
// ============================================================================
//
// CROP DETECTION: Black Bar Detection and Removal
//
// This module handles the detection of black bars in video files and generates
// appropriate crop parameters to remove them. It uses ffmpeg's cropdetect filter
// to analyze video frames and determine the optimal crop values.
//
// KEY COMPONENTS:
// - detect_crop: Main entry point for crop detection
// - HDR-aware black level detection
// - Adaptive sampling based on video duration
//
// WORKFLOW:
// 1. Determine initial crop threshold based on video properties
// 2. For HDR content, refine the threshold using black level analysis
// 3. Run ffmpeg cropdetect on sample frames
// 4. Analyze the results to determine the most common crop values
// 5. Return the crop filter string if black bars are detected
//
// AI-ASSISTANT-INFO: Black bar detection and crop parameter generation

// ---- External crate imports ----

// ---- Internal crate imports ----
use crate::error::CoreResult;
use crate::processing::detection::VideoProperties;

// ---- Standard library imports ----
use std::path::Path;

// ============================================================================
// THRESHOLD DETERMINATION
// ============================================================================

/// Determines the initial crop detection threshold based on color properties.
///
/// This function analyzes the video's color space to determine if it's HDR content
/// and returns an appropriate threshold for black level detection. HDR content
/// typically requires a higher threshold due to its expanded luminance range.
///
/// # Arguments
///
/// * `props` - Video properties containing color space information
///
/// # Returns
///
/// * A tuple containing:
///   - The initial crop threshold value (u32)
///   - A boolean indicating whether the content is HDR
///
/// # Note
///
/// HDR detection is simplified to only use color_space since color_transfer and
/// color_primaries are not available in the current ffprobe crate version.
fn determine_crop_threshold(props: &VideoProperties) -> (u32, bool) {
    // Get the color space, defaulting to empty string if not available
    let cs = props.color_space.as_deref().unwrap_or("");

    // Check if the color space matches common HDR color spaces (bt2020)
    let is_hdr_cs = cs == "bt2020nc" || cs == "bt2020c";

    if is_hdr_cs {
        // For HDR content, use a higher initial threshold
        log::debug!(
            "HDR content potentially detected via color space ({}), adjusting detection sensitivity.",
            cs
        );
        (128, true) // Initial threshold for potential HDR
    } else {
        // For SDR content, use the standard threshold
        (16, false) // Default threshold for SDR
    }
}

/// Runs ffmpeg with signalstats to analyze HDR black levels and refine the threshold.
fn run_hdr_blackdetect(input_file: &Path, initial_threshold: u32) -> CoreResult<u32> {
    log::debug!(
        "Running ffmpeg (sidecar) for HDR black level analysis on {}",
        input_file.display()
    );

    // Use signalstats to analyze luminance values at key frames
    // The metadata filter prints the signalstats values to stderr using file=/dev/stderr
    let filter =
        "select='eq(n,0)+eq(n,100)+eq(n,200)',signalstats,metadata=mode=print:file=/dev/stderr";

    let mut cmd = crate::external::FfmpegCommandBuilder::new()
        .with_hardware_accel(true)
        .build();

    cmd.input(input_file.to_string_lossy())
        .filter_complex(filter)
        .format("null")
        .output("-");

    let mut metadata_output = String::new();
    // Spawn the command
    let mut child = cmd
        .spawn()
        .map_err(|e| crate::error::command_start_error("ffmpeg", e))?;

    // Process events
    let process_result: CoreResult<()> = (|| {
        for event in child.iter().map_err(|e| {
            crate::error::command_failed_error(
                "ffmpeg",
                std::process::ExitStatus::default(),
                e.to_string(),
            )
        })? {
            match event {
                ffmpeg_sidecar::event::FfmpegEvent::Log(_, line)
                | ffmpeg_sidecar::event::FfmpegEvent::Error(line) => {
                    // Capture signalstats metadata output lines
                    if line.contains("lavfi.signalstats.") {
                        metadata_output.push_str(&line);
                        metadata_output.push('\n');
                    }
                }
                _ => {}
            }
        }
        Ok(())
    })();

    if let Err(e) = process_result {
        log::error!(
            "ffmpeg (sidecar) failed during HDR black level analysis on {}: {}",
            input_file.display(),
            e
        );
        log::warn!(
            "HDR black level analysis failed, using initial threshold: {}",
            initial_threshold
        );
        return Ok(initial_threshold);
    }

    log::trace!(
        "ffmpeg HDR signalstats output for {}: {}",
        input_file.display(),
        metadata_output
    );

    // Parse YMIN values from signalstats metadata output
    let matches: Vec<f64> = metadata_output
        .lines()
        .filter_map(|line| {
            // Look for lavfi.signalstats.YMIN=value pattern
            if line.contains("lavfi.signalstats.YMIN=") {
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() >= 2 {
                    parts[1].parse::<f64>().ok()
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    if matches.is_empty() {
        log::warn!(
            "Could not parse YMIN values from signalstats output for {}. Using initial threshold.",
            input_file.display()
        );
        Ok(initial_threshold)
    } else {
        // For HDR content, YMIN represents the minimum luminance
        // We adjust the threshold based on the minimum values found
        let avg_ymin: f64 = matches.iter().sum::<f64>() / matches.len() as f64;

        // For HDR content, we need a higher threshold than the minimum black level
        // The signalstats values are already in 8-bit range (0-255)
        // We use a multiplier to set the threshold well above the black level
        let refined_threshold = (avg_ymin * 2.5).round() as u32; // 2.5x multiplier for HDR
        let clamped_threshold = refined_threshold.clamp(64, 256); // Higher minimum for HDR

        log::debug!(
            "HDR black level analysis: Avg YMIN={:.2}, Refined Threshold={}, Final={}",
            avg_ymin,
            refined_threshold,
            clamped_threshold
        );
        Ok(clamped_threshold)
    }
}

/// Calculates how much time (in seconds) to skip at the end for credits analysis avoidance.
fn calculate_credits_skip(duration: f64) -> f64 {
    if duration > 3600.0 {
        180.0
    } else if duration > 1200.0 {
        60.0
    } else if duration > 300.0 {
        30.0
    } else {
        0.0
    }
}

/// Runs ffmpeg cropdetect and analyzes the results to determine the crop filter.
fn run_cropdetect(
    input_file: &Path,
    crop_threshold: u32,
    dimensions: (u32, u32),
    duration: f64,
) -> CoreResult<Option<String>> {
    let (orig_width, orig_height) = dimensions;
    if orig_width == 0 || orig_height == 0 || duration <= 0.0 {
        log::warn!(
            "Invalid dimensions or duration for cropdetect: {}x{}, {}s",
            orig_width,
            orig_height,
            duration
        );
        return Ok(None);
    }

    let mut total_samples = (duration / 5.0).floor() as u32;
    if total_samples < 20 {
        total_samples = 20;
    }
    let frames_to_scan = total_samples * 2;

    let cropdetect_filter = format!("cropdetect=limit={}:round=2:reset=1", crop_threshold);

    log::debug!(
        "Running ffmpeg (sidecar) cropdetect on {}",
        input_file.display()
    );

    let mut cmd = crate::external::FfmpegCommandBuilder::new()
        .with_hardware_accel(true)
        .build();

    cmd.input(input_file.to_string_lossy())
        .filter_complex(&cropdetect_filter)
        .frames(frames_to_scan)
        .format("null")
        .output("-");

    let mut stderr_output = String::new();
    // Spawn the command
    let mut child = cmd
        .spawn()
        .map_err(|e| crate::error::command_start_error("ffmpeg", e))?;

    // Process events
    let process_result: CoreResult<()> = (|| {
        for event in child.iter().map_err(|e| {
            crate::error::command_failed_error(
                "ffmpeg",
                std::process::ExitStatus::default(),
                e.to_string(),
            )
        })? {
            match event {
                ffmpeg_sidecar::event::FfmpegEvent::Log(_, line)
                | ffmpeg_sidecar::event::FfmpegEvent::Error(line) => {
                    if line.contains("crop=") {
                        stderr_output.push_str(&line);
                        stderr_output.push('\n');
                    }
                }
                _ => {}
            }
        }
        Ok(())
    })();

    if let Err(e) = process_result {
        log::error!(
            "ffmpeg (sidecar) failed during cropdetect on {}: {}",
            input_file.display(),
            e
        );
        return Ok(None);
    }

    log::trace!(
        "ffmpeg cropdetect stderr output for {}: {}",
        input_file.display(),
        stderr_output
    );

    // Parse crop values from stderr output
    let mut crop_counts: std::collections::HashMap<(u32, u32, u32, u32), usize> =
        std::collections::HashMap::new();
    let mut valid_crops_found = false;

    // Parse crop=w:h:x:y patterns from stderr output
    for line in stderr_output.lines() {
        if let Some(crop_start) = line.find("crop=") {
            let crop_part = &line[crop_start + 5..]; // Skip "crop="
            let crop_end = crop_part.find(']').unwrap_or(crop_part.len());
            let crop_values = &crop_part[..crop_end];

            let parts: Vec<&str> = crop_values.split(':').collect();
            if parts.len() == 4 {
                if let (Ok(w), Ok(h), Ok(x), Ok(y)) = (
                    parts[0].parse::<u32>(),
                    parts[1].parse::<u32>(),
                    parts[2].parse::<u32>(),
                    parts[3].parse::<u32>(),
                ) {
                    if w == orig_width {
                        valid_crops_found = true;
                        *crop_counts.entry((w, h, x, y)).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    if !valid_crops_found || crop_counts.is_empty() {
        log::info!(
            "No valid crop values detected (or width changed). Using full dimensions for {}.",
            input_file.display()
        );
        return Ok(None);
    }

    let (most_common_crop, _count) = crop_counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .unwrap();

    let (crop_w, crop_h, crop_x, crop_y) = most_common_crop;

    if crop_w == orig_width && crop_h == orig_height && crop_x == 0 && crop_y == 0 {
        log::info!(
            "Most frequent crop detected is full frame for {}.",
            input_file.display()
        );
        Ok(None)
    } else if crop_w + crop_x > orig_width || crop_h + crop_y > orig_height {
        log::warn!(
            "Detected crop dimensions exceed original video size for {}: crop={}:{}:{}:{}",
            input_file.display(),
            crop_w,
            crop_h,
            crop_x,
            crop_y
        );
        Ok(None)
    } else {
        let crop_filter_string = format!("crop={}:{}:{}:{}", crop_w, crop_h, crop_x, crop_y);
        // Removed redundant log::info! for detected crop, as println! below covers it.
        Ok(Some(crop_filter_string))
    }
}

// ============================================================================
// PUBLIC API
// ============================================================================

/// Main entry point for crop detection.
///
/// This function analyzes a video file to detect black bars and determine the
/// appropriate crop parameters. It handles both SDR and HDR content, with special
/// processing for HDR to ensure accurate black level detection.
///
/// # Arguments
///
/// * `spawner` - Implementation of FfmpegSpawner for executing ffmpeg
/// * `input_file` - Path to the video file to analyze
/// * `video_props` - Properties of the video (resolution, duration, color space)
/// * `disable_crop` - Whether to skip crop detection (e.g., user preference)
///
/// # Returns
///
/// * `Ok((Option<String>, bool))` - A tuple containing:
///   - An optional crop filter string (e.g., "crop=1920:800:0:140")
///   - A boolean indicating whether the content is HDR
/// * `Err(CoreError)` - If an error occurs during analysis
///
/// # Example
///
/// ```rust,no_run
/// use drapto_core::processing::detection::{detect_crop, VideoProperties};
/// use std::path::Path;
///
/// let input_file = Path::new("/path/to/video.mkv");
/// let video_props = VideoProperties {
///     width: 1920,
///     height: 1080,
///     duration_secs: 3600.0,
///     color_space: Some("bt709".to_string()),
/// };
///
/// match detect_crop(input_file, &video_props, false) {
///     Ok((Some(crop_filter), is_hdr)) => {
///         println!("Crop filter: {}, HDR: {}", crop_filter, is_hdr);
///     },
///     Ok((None, is_hdr)) => {
///         println!("No cropping needed, HDR: {}", is_hdr);
///     },
///     Err(e) => {
///         eprintln!("Error during crop detection: {}", e);
///     }
/// }
/// ```
pub fn detect_crop(
    input_file: &Path,
    video_props: &VideoProperties,
    disable_crop: bool,
) -> CoreResult<(Option<String>, bool)> {
    // Check if crop detection is disabled by user preference
    if disable_crop {
        crate::progress_reporting::success("Crop detection complete");
        crate::progress_reporting::status("Detected crop", "Disabled", false);
        return Ok((None, false));
    }

    // STEP 1: Determine initial crop threshold based on video properties
    let (mut crop_threshold, is_hdr) = determine_crop_threshold(video_props);

    // STEP 2: For HDR content, refine the threshold using black level analysis
    if is_hdr {
        log::debug!("Performing HDR black level analysis...");
        log::debug!(
            "Running HDR black level analysis for {}...",
            input_file.display()
        );
        crop_threshold = run_hdr_blackdetect(input_file, crop_threshold)?;
    }

    // STEP 3: Log video properties for debugging
    // Extract filename for logging
    let _filename_cow = input_file
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| input_file.to_string_lossy());

    // We're moving HDR display to the initialization section
    // Keep the is_hdr detection logic but don't log it here

    // Crop threshold is already reported in video.rs

    // STEP 4: Calculate effective analysis duration (skipping credits)
    let credits_skip = calculate_credits_skip(video_props.duration_secs);
    let analysis_duration = if video_props.duration_secs > credits_skip {
        video_props.duration_secs - credits_skip
    } else {
        video_props.duration_secs
    };

    if credits_skip > 0.0 {
        log::debug!(
            "Skipping last {:.2}s for crop analysis (credits). Effective duration: {:.2}s",
            credits_skip,
            analysis_duration
        );
    }

    // STEP 5: Run crop detection analysis
    // Removed redundant "Analyzing frames..." message for cleaner output
    // We'll implement real progress tracking later if needed

    let crop_filter = run_cropdetect(
        input_file,
        crop_threshold,
        (video_props.width, video_props.height),
        analysis_duration,
    )?;

    // STEP 6: Report results using the centralized formatting function
    if crop_filter.is_none() {
        // Use the centralized function for success+status formatting
        crate::progress_reporting::success("Crop detection complete");
        crate::progress_reporting::status("Detected crop", "None required", false);
        log::debug!("No cropping needed for {}", input_file.display());
    } else {
        // Use the centralized function for success+status formatting
        crate::progress_reporting::success("Crop detection complete");
        crate::progress_reporting::status(
            "Detected crop",
            crop_filter.as_deref().unwrap_or(""),
            false,
        );
        log::debug!(
            "Applied crop filter: {}",
            crop_filter.as_deref().unwrap_or("")
        );
    }

    // Return the crop filter and HDR status
    Ok((crop_filter, is_hdr))
}
