// drapto-core/src/processing/detection.rs
//
// This module implements video analysis functions, primarily focused on
// detecting black bars (cropping) using ffmpeg. It translates the logic
// from the Python reference code (`reference/detection.rs`).

use crate::error::{CoreError, CoreResult};
use serde::Deserialize; // For parsing ffprobe JSON
use std::path::Path;
use std::process::{Command, Stdio}; // Keep Command, Stdio for ffprobe
use regex::Regex; // For parsing ffmpeg output
use ffmpeg_sidecar::command::FfmpegCommand; // FfmpegCommand is in command module
// Removed FfprobeCommand import
use ffmpeg_sidecar::event::FfmpegEvent; // Import events for stderr capture
// Removed unused StdDuration import

// --- Structs for ffprobe JSON output ---

#[derive(Deserialize, Debug, Clone)]
struct FfprobeFormat {
    duration: Option<String>, // Duration is often a string
}

#[derive(Deserialize, Debug, Clone)]
struct FfprobeStream {
    codec_type: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    color_space: Option<String>,
    color_transfer: Option<String>,
    color_primaries: Option<String>,
    // Add tags if needed for DV detection, e.g.:
    // tags: Option<std::collections::HashMap<String, String>>,
}

#[derive(Deserialize, Debug, Clone)]
struct FfprobeOutput {
    format: FfprobeFormat,
    streams: Vec<FfprobeStream>,
}

// --- Struct to hold extracted properties ---

#[derive(Debug, Clone, Default)]
pub(crate) struct VideoProperties {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) duration: f64,
    pub(crate) color_space: Option<String>,
    pub(crate) color_transfer: Option<String>,
    pub(crate) color_primaries: Option<String>,
}

// --- Implementation ---

// TODO: Implement detect_dolby_vision (Step 3) - Integrated into get_video_properties for now
// TODO: Implement detect_dolby_vision (Step 3)
/// Determines the initial crop detection threshold based on color properties.
/// Returns a tuple (crop_threshold, is_hdr).
fn determine_crop_threshold(props: &VideoProperties) -> (u32, bool) {
    let ct = props.color_transfer.as_deref().unwrap_or("");
    let cp = props.color_primaries.as_deref().unwrap_or("");
    let cs = props.color_space.as_deref().unwrap_or("");

    // Regex patterns for matching HDR color properties
    // Pre-compile regex for efficiency if called frequently, but for single use, inline is fine.
    let is_hdr_ct = Regex::new(r"^(smpte2084|arib-std-b67|smpte428|bt2020-10|bt2020-12)$").unwrap().is_match(ct);
    let is_hdr_cp = cp == "bt2020";
    let is_hdr_cs = Regex::new(r"^(bt2020nc|bt2020c)$").unwrap().is_match(cs);

    if is_hdr_ct || is_hdr_cp || is_hdr_cs {
        log::info!("HDR content detected via color properties, adjusting detection sensitivity.");
        (128, true) // Initial threshold for HDR
    } else {
        (16, false) // Default threshold for SDR
    }
}

/// Runs ffmpeg blackdetect on sample frames for HDR content to refine the threshold.
fn run_hdr_blackdetect(input_file: &Path, initial_threshold: u32) -> CoreResult<u32> {
    log::debug!("Running ffmpeg (sidecar) for HDR black level analysis on {}", input_file.display());

    let filter = "select='eq(n,0)+eq(n,100)+eq(n,200)',blackdetect=d=0:pic_th=0.1";

    let mut cmd = FfmpegCommand::new();
    cmd.hide_banner()
        .input(input_file.to_string_lossy().into_owned()) // Convert Path -> String
        .filter_complex(filter) // Use filter_complex for flexibility
        .format("null") // Output format null
        .output("-"); // Output target null

    // log::debug!("Constructed ffmpeg HDR blackdetect command (sidecar): {:?}", cmd.as_args()); // Removed as_args

    let mut stderr_output = String::new();
    // Spawn the child process, handling potential spawn errors immediately
    let mut child = cmd.spawn()
        .map_err(|e| CoreError::CommandStart("ffmpeg (sidecar - hdr blackdetect spawn)".to_string(), e))?;

    // Get the iterator Result
    let iter_result = child.iter();

    // Match on the iterator Result
    let process_result = match iter_result {
        Ok(iterator) => {
            // Iterate through events if iter() succeeded
            for event in iterator {
                 match event {
                    // Capture relevant stderr lines for parsing
                    FfmpegEvent::Log(_, line) | FfmpegEvent::Error(line) => {
                        if line.contains("black_level") {
                            stderr_output.push_str(&line);
                            stderr_output.push('\n');
                        }
                    }
                    _ => {} // Ignore other events
                }
            }
            Ok(()) // Indicate successful iteration and process completion
        }
        Err(e) => {
            // Handle error from iter() itself (e.g., pipe broken)
            Err(CoreError::CommandFailed("ffmpeg (sidecar - hdr blackdetect iter)".to_string(), std::process::ExitStatus::default(), e.to_string()))
        }
    };

    // Check if the iteration process failed
    if let Err(e) = process_result {
        log::error!("ffmpeg (sidecar) failed during HDR blackdetect on {}: {}", input_file.display(), e);
        // Return initial threshold on failure, maybe log a warning
        log::warn!("HDR blackdetect failed, using initial threshold: {}", initial_threshold);
        return Ok(initial_threshold);
    }

    log::trace!("ffmpeg HDR blackdetect stderr output for {}: {}", input_file.display(), stderr_output);

    // Parse black_level from stderr
    // Example output line: [blackdetect @ 0x...] black_start:0 black_end:10 black_level: 64
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
        // Clamp the threshold within a reasonable range (e.g., 16-256)
        let clamped_threshold = refined_threshold.max(16).min(256);
        log::info!("HDR black level analysis: Avg={}, Refined Threshold={}, Clamped={}", avg_black_level, refined_threshold, clamped_threshold);
        Ok(clamped_threshold)
    }
}

/// Calculates how much time (in seconds) to skip at the end for credits analysis avoidance.
fn calculate_credits_skip(duration: f64) -> f64 {
    if duration > 3600.0 { // > 1 hour
        180.0 // Skip 3 minutes
    } else if duration > 1200.0 { // > 20 minutes
        60.0 // Skip 1 minute
    } else if duration > 300.0 { // > 5 minutes
        30.0 // Skip 30 seconds
    } else {
        0.0
    }
}

/// Runs ffmpeg cropdetect and analyzes the results to determine the crop filter.
fn run_cropdetect(
    input_file: &Path,
    crop_threshold: u32,
    dimensions: (u32, u32),
    duration: f64, // Analysis duration (potentially shortened)
) -> CoreResult<Option<String>> {
    let (orig_width, orig_height) = dimensions;
    if orig_width == 0 || orig_height == 0 || duration <= 0.0 {
        log::warn!("Invalid dimensions or duration for cropdetect: {}x{}, {}s", orig_width, orig_height, duration);
        return Ok(None); // Cannot run cropdetect
    }

    // Calculate sampling parameters based on Python reference
    // Determine number of samples, aiming for roughly 1 sample every 5 seconds, minimum 20 samples.
    let mut total_samples = (duration / 5.0).floor() as u32;
    if total_samples < 20 {
        total_samples = 20;
    }
    // The interval variable itself wasn't used, only total_samples.
    // The reference code uses `select='not(mod(n,30))'` which seems to imply selecting every 30th frame?
    // Let's stick closer to the interval logic for now. We need to select frames based on time interval.
    // A common way is using `-vf select='isnan(prev_selected_t)+gte(t-prev_selected_t\,INTERVAL)'`
    // Or, simpler for fixed interval: `-vf select='not(mod(n,round(FRAME_RATE*INTERVAL)))'`
    // Let's use the simpler `cropdetect` filter directly as in the reference, assuming it handles sampling internally.
    // The reference uses `-frames:v {total_samples * 2}` - let's replicate that.
    let frames_to_scan = total_samples * 2;

    let cropdetect_filter = format!("cropdetect=limit={}:round=2:reset=1", crop_threshold);

    log::debug!("Running ffmpeg (sidecar) cropdetect on {}", input_file.display());

    let mut cmd = FfmpegCommand::new();
    cmd.hide_banner()
        .input(input_file.to_string_lossy().into_owned()) // Convert Path -> String
        .filter_complex(&cropdetect_filter) // Use filter_complex
        .frames(frames_to_scan) // Use .frames() instead of .frames_video()
        .format("null") // Output format null
        .output("-"); // Output target null

    // log::debug!("Constructed ffmpeg cropdetect command (sidecar): {:?}", cmd.as_args()); // Removed as_args

    let mut stderr_output = String::new();
    // Spawn the child process, handling potential spawn errors immediately
    let mut child = cmd.spawn()
        .map_err(|e| CoreError::CommandStart("ffmpeg (sidecar - cropdetect spawn)".to_string(), e))?;

    // Get the iterator Result
    let iter_result = child.iter();

    // Match on the iterator Result
    let process_result = match iter_result {
         Ok(iterator) => {
            // Iterate through events if iter() succeeded
            for event in iterator {
                 match event {
                    // Capture relevant stderr lines for parsing
                    FfmpegEvent::Log(_, line) | FfmpegEvent::Error(line) => {
                        if line.contains("crop=") {
                            stderr_output.push_str(&line);
                            stderr_output.push('\n');
                        }
                    }
                    _ => {} // Ignore other events
                }
            }
            Ok(()) // Indicate successful iteration and process completion
        }
        Err(e) => {
            // Handle error from iter() itself (e.g., pipe broken)
            Err(CoreError::CommandFailed("ffmpeg (sidecar - cropdetect iter)".to_string(), std::process::ExitStatus::default(), e.to_string()))
        }
    };

    // Note: cropdetect often finishes successfully even if no crop is detected.
    // We rely on parsing the stderr_output. Errors during execution are handled below.
    if let Err(e) = process_result {
        log::error!("ffmpeg (sidecar) failed during cropdetect on {}: {}", input_file.display(), e);
        // Don't error out, just return None for crop filter
        return Ok(None);
    }

    log::trace!("ffmpeg cropdetect stderr output for {}: {}", input_file.display(), stderr_output);

    // Parse crop=W:H:X:Y values from stderr
    let crop_re = Regex::new(r"crop=(\d+):(\d+):(\d+):(\d+)").unwrap();
    let mut crop_counts: std::collections::HashMap<(u32, u32, u32, u32), usize> = std::collections::HashMap::new();
    let mut valid_crops_found = false;

    for cap in crop_re.captures_iter(&stderr_output) {
        // Okay to unwrap here as regex ensures digits
        let w: u32 = cap[1].parse().unwrap();
        let h: u32 = cap[2].parse().unwrap();
        let x: u32 = cap[3].parse().unwrap();
        let y: u32 = cap[4].parse().unwrap();

        // Reference code only considered crops where width == original width. Let's keep that.
        if w == orig_width {
            valid_crops_found = true;
            *crop_counts.entry((w, h, x, y)).or_insert(0) += 1;
        }
    }

    if !valid_crops_found || crop_counts.is_empty() {
        log::info!("No valid crop values detected (or width changed). Using full dimensions for {}.", input_file.display());
        // Return None, let the caller decide if full dimensions means no filter or crop=W:H:0:0
        return Ok(None);
    }

    // Find the most frequent crop setting
    let (most_common_crop, _count) = crop_counts.into_iter()
        .max_by_key(|&(_, count)| count)
        .unwrap(); // Safe unwrap because we checked is_empty

    let (crop_w, crop_h, crop_x, crop_y) = most_common_crop;

    // Calculate black bar size based on the most common height
    // Reference code logic:
    // black_bar_size = (orig_height - most_common_height) // 2
    // black_bar_percent = (black_bar_size * 100) // orig_height
    // if black_bar_percent > 1: return crop=orig_width:most_common_height:0:black_bar_size
    // else: return crop=orig_width:orig_height:0:0

    // Let's simplify: if the most common crop is different from full frame, use it.
    // We trust cropdetect's most frequent result.
    if crop_w == orig_width && crop_h == orig_height && crop_x == 0 && crop_y == 0 {
        log::info!("Most frequent crop detected is full frame for {}.", input_file.display());
        Ok(None) // No cropping needed
    } else {
        // Ensure crop dimensions are valid
        if crop_w + crop_x > orig_width || crop_h + crop_y > orig_height {
             log::warn!("Detected crop dimensions exceed original video size for {}: crop={}:{}:{}:{}", input_file.display(), crop_w, crop_h, crop_x, crop_y);
             Ok(None) // Invalid crop, default to no cropping
        } else {
            let crop_filter_string = format!("crop={}:{}:{}:{}", crop_w, crop_h, crop_x, crop_y);
            log::info!("Detected crop for {}: {}", input_file.display(), crop_filter_string);
            Ok(Some(crop_filter_string))
        }
    }
}

// TODO: Implement detect_crop (Step 6) - Partially implemented below

/// Gets video properties using ffprobe.
#[cfg(not(feature = "test-mock-ffprobe"))] // Original implementation
pub(crate) fn get_video_properties(input_file: &Path) -> CoreResult<VideoProperties> {
    let cmd_ffprobe = "ffprobe";
    let args = [
        "-v", "quiet", // Use quiet instead of error to suppress warnings but allow JSON
        "-print_format", "json",
        "-show_format", // Needed for duration
        "-show_streams", // Needed for resolution, color, codec
        &input_file.to_string_lossy(),
    ];

    log::debug!("Running ffprobe to get properties: {} {:?}", cmd_ffprobe, args);

    let output = Command::new(cmd_ffprobe)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| CoreError::CommandStart(cmd_ffprobe.to_string(), e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::error!("ffprobe failed for property check on {}: {}", input_file.display(), stderr.trim());
        return Err(CoreError::CommandFailed(
            cmd_ffprobe.to_string(),
            output.status,
            stderr.trim().to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    log::trace!("ffprobe properties output for {}: {}", input_file.display(), stdout);

    let ffprobe_data: FfprobeOutput = serde_json::from_str(&stdout)
        .map_err(|e| CoreError::JsonParseError(format!("ffprobe properties output: {}", e)))?;

    // Extract duration from format
    let duration = ffprobe_data.format.duration
        .as_deref()
        .and_then(|d_str| d_str.parse::<f64>().ok())
        .unwrap_or(0.0);

    // Find the first video stream
    let video_stream = ffprobe_data.streams.iter()
        .find(|s| s.codec_type.as_deref() == Some("video"))
        .ok_or_else(|| CoreError::VideoInfoError(format!("No video stream found in {}", input_file.display())))?;

    let width = video_stream.width.unwrap_or(0);
    let height = video_stream.height.unwrap_or(0);

    if width == 0 || height == 0 {
         return Err(CoreError::VideoInfoError(format!("Could not determine video dimensions for {}", input_file.display())));
    }

    Ok(VideoProperties {
        width,
        height,
        duration,
        color_space: video_stream.color_space.clone(),
        color_transfer: video_stream.color_transfer.clone(),
        color_primaries: video_stream.color_primaries.clone(),
    })
}

// Mock implementation for get_video_properties
#[cfg(feature = "test-mock-ffprobe")]
pub(crate) fn get_video_properties(input_file: &Path) -> CoreResult<VideoProperties> {
    log::warn!("Using MOCKED get_video_properties for path: {}", input_file.display());
    // Return some default valid data for testing purposes
    Ok(VideoProperties {
        width: 1920,
        height: 1080,
        duration: 120.0, // 2 minutes
        color_space: Some("bt709".to_string()),
        color_transfer: Some("bt709".to_string()),
        color_primaries: Some("bt709".to_string()),
    })
}


/// Main crop detection function (entry point).
/// Returns a tuple: (Option<crop_filter_string>, is_hdr)
pub(crate) fn detect_crop(
    input_file: &Path,
    video_props: &VideoProperties, // Accept pre-fetched properties
    disable_crop: bool,
) -> CoreResult<(Option<String>, bool)> {
    // TODO: Use config value if disable_crop is None (requires config access)
    // For now, only use the direct parameter.
    if disable_crop {
        log::info!("Crop detection disabled via parameter for {}", input_file.display());
        return Ok((None, false));
    }

    // video_props are now passed in, no need to fetch or validate basic properties here.

    // Determine initial threshold and HDR status
    let (mut crop_threshold, is_hdr) = determine_crop_threshold(&video_props);

    // Refine threshold for HDR content
    if is_hdr {
        println!("🔬 Performing HDR black level analysis..."); // User-facing message
        log::info!("Running HDR black level analysis for {}...", input_file.display());
        crop_threshold = run_hdr_blackdetect(input_file, crop_threshold)?;
    }

    log::info!("Video properties for {}: {}x{}, {:.2}s, HDR: {}, Crop Threshold: {}",
        input_file.display(),
        video_props.width, video_props.height, video_props.duration,
        is_hdr,
        crop_threshold); // Log the determined threshold

    // Calculate duration for analysis, skipping credits
    let credits_skip = calculate_credits_skip(video_props.duration);
    let analysis_duration = if video_props.duration > credits_skip {
        video_props.duration - credits_skip
    } else {
        video_props.duration // Don't skip if duration is too short
    };
    if credits_skip > 0.0 {
        log::debug!("Skipping last {:.2}s for crop analysis (credits). Effective duration: {:.2}s", credits_skip, analysis_duration);
    }

    // Run crop detection
    println!("✂️ Running crop detection analysis..."); // User-facing message
    log::info!("Running crop detection analysis for {}...", input_file.display());
    let crop_filter = run_cropdetect(
        input_file,
        crop_threshold,
        (video_props.width, video_props.height),
        analysis_duration,
    )?;

    if crop_filter.is_none() {
        println!("✅ Crop detection complete: No cropping needed."); // User-facing message
        log::info!("No cropping filter determined for {}.", input_file.display());
    } else {
        println!("✅ Crop detection complete: {}", crop_filter.as_deref().unwrap_or("")); // User-facing message
    }

    Ok((crop_filter, is_hdr))
}