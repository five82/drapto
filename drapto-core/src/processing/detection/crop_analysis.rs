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
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::FfmpegEvent;
use regex::Regex;

// ---- Internal crate imports ----
use crate::error::CoreResult;
use crate::external::{FfmpegProcess, FfmpegSpawner};
use crate::hardware_accel::add_hardware_acceleration_to_command;
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
    let is_hdr_cs = Regex::new(r"^(bt2020nc|bt2020c)$").unwrap().is_match(cs);

    if is_hdr_cs {
        // For HDR content, use a higher initial threshold
        log::info!("HDR content potentially detected via color space ({}), adjusting detection sensitivity.", cs);
        (128, true) // Initial threshold for potential HDR
    } else {
        // For SDR content, use the standard threshold
        (16, false) // Default threshold for SDR
    }
}

/// Runs ffmpeg blackdetect on sample frames for HDR content to refine the threshold.
fn run_hdr_blackdetect<S: FfmpegSpawner>(spawner: &S, input_file: &Path, initial_threshold: u32) -> CoreResult<u32> {
    log::debug!("Running ffmpeg (sidecar) for HDR black level analysis on {}", input_file.display());

    let filter = "select='eq(n,0)+eq(n,100)+eq(n,200)',blackdetect=d=0:pic_th=0.1";

    let mut cmd = FfmpegCommand::new();
    cmd.hide_banner();

    // Add hardware acceleration options BEFORE the input
    add_hardware_acceleration_to_command(&mut cmd, true, false); // Don't need to check return value

    cmd.input(input_file.to_string_lossy()); // Use reference
    cmd.filter_complex(filter);
    cmd.format("null");
    cmd.output("-");

    let mut stderr_output = String::new();
    // Pass cmd by value, matching trait signature
    let mut child = spawner.spawn(cmd)?;

    let process_result = child.handle_events(|event| {
        match event {
            FfmpegEvent::Log(_, line) | FfmpegEvent::Error(line) => {
                if line.contains("black_level") {
                    stderr_output.push_str(&line);
                    stderr_output.push('\n');
                }
            }
            _ => {}
        }
        Ok(())
    });

    if let Err(e) = process_result {
        log::error!("ffmpeg (sidecar) failed during HDR blackdetect on {}: {}", input_file.display(), e);
        log::warn!("HDR blackdetect failed, using initial threshold: {}", initial_threshold);
        return Ok(initial_threshold);
    }

    log::trace!("ffmpeg HDR blackdetect stderr output for {}: {}", input_file.display(), stderr_output);

    let black_level_re = Regex::new(r"black_level:\s*([0-9.]+)").unwrap();
    let matches: Vec<f64> = black_level_re.captures_iter(&stderr_output)
        .filter_map(|cap| cap.get(1)?.as_str().parse::<f64>().ok())
        .collect();

    if matches.is_empty() {
        log::warn!("Could not parse black_level from ffmpeg output for {}. Using initial threshold.", input_file.display());
        Ok(initial_threshold)
    } else {
        let avg_black_level: f64 = matches.iter().sum::<f64>() / matches.len() as f64;
        let refined_threshold = (avg_black_level * 1.5).round() as u32;
        let clamped_threshold = refined_threshold.clamp(16, 256); // Use clamp()
        log::info!("HDR black level analysis: Avg={}, Refined Threshold={}, Clamped={}", avg_black_level, refined_threshold, clamped_threshold);
        Ok(clamped_threshold)
    }
}

/// Calculates how much time (in seconds) to skip at the end for credits analysis avoidance.
fn calculate_credits_skip(duration: f64) -> f64 {
    if duration > 3600.0 { 180.0 }
    else if duration > 1200.0 { 60.0 }
    else if duration > 300.0 { 30.0 }
    else { 0.0 }
}

/// Runs ffmpeg cropdetect and analyzes the results to determine the crop filter.
fn run_cropdetect<S: FfmpegSpawner>(
    spawner: &S,
    input_file: &Path,
    crop_threshold: u32,
    dimensions: (u32, u32),
    duration: f64,
) -> CoreResult<Option<String>> {
    let (orig_width, orig_height) = dimensions;
    if orig_width == 0 || orig_height == 0 || duration <= 0.0 {
        log::warn!("Invalid dimensions or duration for cropdetect: {}x{}, {}s", orig_width, orig_height, duration);
        return Ok(None);
    }

    let mut total_samples = (duration / 5.0).floor() as u32;
    if total_samples < 20 { total_samples = 20; }
    let frames_to_scan = total_samples * 2;

    let cropdetect_filter = format!("cropdetect=limit={}:round=2:reset=1", crop_threshold);

    log::debug!("Running ffmpeg (sidecar) cropdetect on {}", input_file.display());

    let mut cmd = FfmpegCommand::new();
    cmd.hide_banner();

    // Add hardware acceleration options BEFORE the input - no need to log status
    add_hardware_acceleration_to_command(&mut cmd, true, false);

    cmd.input(input_file.to_string_lossy()); // Use reference
    cmd.filter_complex(&cropdetect_filter);
    cmd.frames(frames_to_scan);
    cmd.format("null");
    cmd.output("-");

    let mut stderr_output = String::new();
    // Pass cmd by value, matching trait signature
    let mut child = spawner.spawn(cmd)?;

    let process_result = child.handle_events(|event| {
        match event {
            FfmpegEvent::Log(_, line) | FfmpegEvent::Error(line) => {
                if line.contains("crop=") {
                    stderr_output.push_str(&line);
                    stderr_output.push('\n');
                }
            }
            _ => {}
        }
        Ok(())
    });

    if let Err(e) = process_result {
        log::error!("ffmpeg (sidecar) failed during cropdetect on {}: {}", input_file.display(), e);
        return Ok(None);
    }

    log::trace!("ffmpeg cropdetect stderr output for {}: {}", input_file.display(), stderr_output);

    let crop_re = Regex::new(r"crop=(\d+):(\d+):(\d+):(\d+)").unwrap();
    let mut crop_counts: std::collections::HashMap<(u32, u32, u32, u32), usize> = std::collections::HashMap::new();
    let mut valid_crops_found = false;

    for cap in crop_re.captures_iter(&stderr_output) {
        let w: u32 = cap[1].parse().unwrap();
        let h: u32 = cap[2].parse().unwrap();
        let x: u32 = cap[3].parse().unwrap();
        let y: u32 = cap[4].parse().unwrap();

        if w == orig_width {
            valid_crops_found = true;
            *crop_counts.entry((w, h, x, y)).or_insert(0) += 1;
        }
    }

    if !valid_crops_found || crop_counts.is_empty() {
        log::info!("No valid crop values detected (or width changed). Using full dimensions for {}.", input_file.display());
        return Ok(None);
    }

    let (most_common_crop, _count) = crop_counts.into_iter()
        .max_by_key(|&(_, count)| count)
        .unwrap();

    let (crop_w, crop_h, crop_x, crop_y) = most_common_crop;

    if crop_w == orig_width && crop_h == orig_height && crop_x == 0 && crop_y == 0 {
        log::info!("Most frequent crop detected is full frame for {}.", input_file.display());
        Ok(None)
    } else if crop_w + crop_x > orig_width || crop_h + crop_y > orig_height {
         log::warn!("Detected crop dimensions exceed original video size for {}: crop={}:{}:{}:{}", input_file.display(), crop_w, crop_h, crop_x, crop_y);
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
/// use drapto_core::external::SidecarSpawner;
/// use std::path::Path;
///
/// let spawner = SidecarSpawner;
/// let input_file = Path::new("/path/to/video.mkv");
/// let video_props = VideoProperties {
///     width: 1920,
///     height: 1080,
///     duration_secs: 3600.0,
///     color_space: Some("bt709".to_string()),
/// };
///
/// match detect_crop(&spawner, input_file, &video_props, false) {
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
pub fn detect_crop<S: FfmpegSpawner>(
    spawner: &S,
    input_file: &Path,
    video_props: &VideoProperties,
    disable_crop: bool,
) -> CoreResult<(Option<String>, bool)> {
    // Check if crop detection is disabled by user preference
    if disable_crop {
        log::info!("Crop detection disabled via parameter for {}", input_file.display());
        return Ok((None, false));
    }

    // STEP 1: Determine initial crop threshold based on video properties
    let (mut crop_threshold, is_hdr) = determine_crop_threshold(video_props);

    // STEP 2: For HDR content, refine the threshold using black level analysis
    if is_hdr {
        println!("🔬 Performing HDR black level analysis...");
        log::info!("Running HDR black level analysis for {}...", input_file.display());
        crop_threshold = run_hdr_blackdetect(spawner, input_file, crop_threshold)?;
    }

    // STEP 3: Log video properties for debugging
    // Extract filename for logging
    let filename_cow = input_file
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| input_file.to_string_lossy());

    // Log video properties with multiple lines
    log::info!(
        "Video Properties for: {}",
        filename_cow
    );
    log::info!(
        "  {:<18} {}", // Left-align label with padding
        "Resolution:",
        format!("{}x{}", video_props.width, video_props.height)
    );
    log::info!(
        "  {:<18} {}", // Left-align label with padding
        "Duration:",
        format!("{:.2}s", video_props.duration_secs)
    );
    log::info!(
        "  {:<18} {}", // Left-align label with padding
        "HDR:",
        format!("{}", is_hdr)
    );
    log::info!(
        "  {:<18} {}", // Left-align label with padding
        "Crop Threshold:",
        format!("{}", crop_threshold)
    );

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
            credits_skip, analysis_duration
        );
    }

    // STEP 5: Run crop detection analysis
    println!("Running crop detection analysis...");
    log::info!("Running crop detection analysis for {}...", filename_cow);

    let crop_filter = run_cropdetect(
        spawner,
        input_file,
        crop_threshold,
        (video_props.width, video_props.height),
        analysis_duration,
    )?;

    // STEP 6: Report results
    if crop_filter.is_none() {
        println!("Crop detection complete: No cropping needed.");
        log::info!("No cropping filter determined for {}.", input_file.display());
    } else {
        println!(
            "Crop detection complete: {}",
            crop_filter.as_deref().unwrap_or("")
        );
    }

    // Return the crop filter and HDR status
    Ok((crop_filter, is_hdr))
}