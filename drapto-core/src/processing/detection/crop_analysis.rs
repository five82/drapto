// drapto-core/src/processing/detection/crop_analysis.rs

use crate::error::CoreResult; // Remove unused CoreError
use crate::external::{FfmpegProcess, FfmpegSpawner};
use crate::processing::detection::VideoProperties; // Import VideoProperties from parent's re-export
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::FfmpegEvent;
use regex::Regex;
use std::path::Path;

/// Determines the initial crop detection threshold based on color properties.
/// Returns a tuple (crop_threshold, is_hdr).
fn determine_crop_threshold(props: &VideoProperties) -> (u32, bool) {
    // Simplified HDR detection based only on color_space, as color_transfer and color_primaries
    // are not available via ffprobe crate v0.3.3
    let cs = props.color_space.as_deref().unwrap_or("");

    // Regex for common HDR color spaces
    let is_hdr_cs = Regex::new(r"^(bt2020nc|bt2020c)$").unwrap().is_match(cs);

    if is_hdr_cs {
        log::info!("HDR content potentially detected via color space ({}), adjusting detection sensitivity.", cs);
        (128, true) // Initial threshold for potential HDR
    } else {
        (16, false) // Default threshold for SDR
    }
}

/// Runs ffmpeg blackdetect on sample frames for HDR content to refine the threshold.
fn run_hdr_blackdetect<S: FfmpegSpawner>(spawner: &S, input_file: &Path, initial_threshold: u32) -> CoreResult<u32> {
    log::debug!("Running ffmpeg (sidecar) for HDR black level analysis on {}", input_file.display());

    let filter = "select='eq(n,0)+eq(n,100)+eq(n,200)',blackdetect=d=0:pic_th=0.1";

    let mut cmd = FfmpegCommand::new();
    cmd.hide_banner()
        .input(input_file.to_string_lossy().into_owned())
        .filter_complex(filter)
        .format("null")
        .output("-");

    let mut stderr_output = String::new();
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
        let clamped_threshold = refined_threshold.max(16).min(256);
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
    cmd.hide_banner()
        .input(input_file.to_string_lossy().into_owned())
        .filter_complex(&cropdetect_filter)
        .frames(frames_to_scan)
        .format("null")
        .output("-");

    let mut stderr_output = String::new();
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
    } else {
        if crop_w + crop_x > orig_width || crop_h + crop_y > orig_height {
             log::warn!("Detected crop dimensions exceed original video size for {}: crop={}:{}:{}:{}", input_file.display(), crop_w, crop_h, crop_x, crop_y);
             Ok(None)
        } else {
            let crop_filter_string = format!("crop={}:{}:{}:{}", crop_w, crop_h, crop_x, crop_y);
            log::info!("Detected crop for {}: {}", input_file.display(), crop_filter_string);
            Ok(Some(crop_filter_string))
        }
    }
}

/// Main crop detection function (entry point).
/// Returns a tuple: (Option<crop_filter_string>, is_hdr)
pub fn detect_crop<S: FfmpegSpawner>( // Keep public as it's re-exported
    spawner: &S,
    input_file: &Path,
    video_props: &VideoProperties,
    disable_crop: bool,
) -> CoreResult<(Option<String>, bool)> {
    if disable_crop {
        log::info!("Crop detection disabled via parameter for {}", input_file.display());
        return Ok((None, false));
    }

    let (mut crop_threshold, is_hdr) = determine_crop_threshold(&video_props);

    if is_hdr {
        println!("🔬 Performing HDR black level analysis...");
        log::info!("Running HDR black level analysis for {}...", input_file.display());
        crop_threshold = run_hdr_blackdetect(spawner, input_file, crop_threshold)?;
    }

    log::info!("Video properties for {}: {}x{}, {:.2}s, HDR: {}, Crop Threshold: {}",
        input_file.display(),
        video_props.width, video_props.height, video_props.duration_secs, // Use renamed field
        is_hdr,
        crop_threshold);

    let credits_skip = calculate_credits_skip(video_props.duration_secs); // Use renamed field
    let analysis_duration = if video_props.duration_secs > credits_skip { // Use renamed field
        video_props.duration_secs - credits_skip // Use renamed field
    } else {
        video_props.duration_secs // Use renamed field
    };
    if credits_skip > 0.0 {
        log::debug!("Skipping last {:.2}s for crop analysis (credits). Effective duration: {:.2}s", credits_skip, analysis_duration);
    }

    println!("✂️ Running crop detection analysis...");
    log::info!("Running crop detection analysis for {}...", input_file.display());
    let crop_filter = run_cropdetect(
        spawner,
        input_file,
        crop_threshold,
        (video_props.width, video_props.height),
        analysis_duration,
    )?;

    if crop_filter.is_none() {
        println!("✅ Crop detection complete: No cropping needed.");
        log::info!("No cropping filter determined for {}.", input_file.display());
    } else {
        println!("✅ Crop detection complete: {}", crop_filter.as_deref().unwrap_or(""));
    }

    Ok((crop_filter, is_hdr))
}