use std::path::Path;
use std::process::Command;
use std::cmp::Ordering;
use regex::Regex;
use log::{debug, info, warn, error};

use crate::error::{DraptoError, Result};
use crate::media::MediaInfo;
use crate::config::Config;
use crate::util::command;

/// Detect scenes in a video file and return scene change timestamps
///
/// This function:
/// 1. Determines video properties like duration and color space
/// 2. Sets the appropriate detection threshold for HDR/SDR content
/// 3. Uses FFmpeg scene detection filters to identify scene changes
/// 4. Filters out scenes that are too close together
/// 5. Inserts artificial boundaries for segments that exceed max length
///
/// # Arguments
///
/// * `input_file` - Path to the input video file
/// * `scene_threshold` - Detection threshold for SDR content (0-100)
/// * `hdr_scene_threshold` - Detection threshold for HDR content (0-100)
/// * `min_segment_length` - Minimum segment length in seconds
/// * `max_segment_length` - Maximum segment length in seconds
///
/// # Returns
///
/// * `Result<Vec<f64>>` - Vector of scene change timestamps in seconds
pub fn detect_scenes<P: AsRef<Path>>(
    input_file: P,
    scene_threshold: f32,
    hdr_scene_threshold: f32,
    min_segment_length: f32,
    max_segment_length: f32,
) -> Result<Vec<f64>> {
    // Get media info for duration and color properties
    let media_info = MediaInfo::from_path(&input_file)?;
    
    // Get total duration
    let total_duration = match media_info.duration() {
        Some(duration) if duration > 0.0 => duration,
        Some(duration) => {
            warn!("Invalid duration {:.2}, using fallback detection", duration);
            return Ok(Vec::new());
        },
        None => {
            error!("Could not determine video duration");
            return Ok(Vec::new());
        }
    };
    
    // Skip scene detection for very short videos
    if total_duration < 2.0 {
        info!("Skipping scene detection for ultra-short video");
        return Ok(vec![total_duration]);
    }
    
    // Determine scene detection threshold based on HDR status
    let is_hdr = media_info.is_hdr();
    let threshold = if is_hdr {
        info!("Using HDR scene threshold: {}", hdr_scene_threshold);
        hdr_scene_threshold
    } else {
        info!("Using standard scene threshold: {}", scene_threshold);
        scene_threshold
    };
    
    // Get candidate scenes
    match get_candidate_scenes(&input_file, threshold) {
        Ok(candidates) => {
            // Filter and process scenes
            let filtered_scenes = filter_scene_candidates(candidates, min_segment_length);
            let final_boundaries = insert_artificial_boundaries(filtered_scenes, total_duration, max_segment_length);
            
            info!("Detected {} scenes, final boundaries: {:?}", 
                 final_boundaries.len(), final_boundaries);
            
            Ok(final_boundaries)
        },
        Err(e) => {
            error!("Scene detection failed: {}", e);
            Ok(Vec::new())
        }
    }
}

/// Run FFmpeg to detect candidate scenes
pub fn get_candidate_scenes<P: AsRef<Path>>(input_file: P, threshold: f32) -> Result<Vec<f64>> {
    let input_path = input_file.as_ref();
    
    // Try first with a basic scene detection approach
    if let Ok(scenes) = detect_with_basic_method(input_path, threshold) {
        if !scenes.is_empty() {
            debug!("Scene detection successful with basic method, found {} scenes", scenes.len());
            return Ok(scenes);
        }
    }
    
    // If that fails, try with the more complex scdet filter
    debug!("Basic scene detection didn't find scenes, trying with scdet filter");
    let scdet_scenes = detect_with_scdet(input_path, threshold)?;
    
    // If still no scenes, try one more method
    if scdet_scenes.is_empty() {
        debug!("scdet filter didn't find scenes, trying blackframe detection");
        detect_with_blackframe(input_path)
    } else {
        Ok(scdet_scenes)
    }
}

/// Detect scenes using basic frame difference method
fn detect_with_basic_method<P: AsRef<Path>>(input_path: P, threshold: f32) -> Result<Vec<f64>> {
    let input_path = input_path.as_ref();
    let threshold_normalized = threshold / 100.0;
    
    // Use a simple showinfo filter to get frame info and manually detect scenes
    // This is more reliable across different FFmpeg versions
    let mut cmd = Command::new("ffmpeg");
    cmd.args([
        "-hide_banner",
        "-i", input_path.to_str().unwrap_or_default(),
        "-vf", "showinfo",  // Just show all frame info
        "-loglevel", "info",
        "-f", "null", "-"
    ]);
    
    // Run command and parse output
    let output = command::run_command(&mut cmd)?;
    let output_str = String::from_utf8_lossy(&output.stderr);
    
    // For debugging
    debug!("Basic method FFmpeg output (truncated): {:.500}", output_str);
    
    // With showinfo filter, we get details about each frame
    // We want to extract the pts_time (timestamp) of each frame
    let pts_re = Regex::new(r"pts_time:(\d+\.\d+)").unwrap();
    
    // Also extract information about pixel differences between frames
    // We'll use this to detect scene changes
    let pkt_size_re = Regex::new(r"size=(\d+)").unwrap();
    
    let mut frames = Vec::new();
    let mut pts_values = Vec::new();
    let mut pkt_sizes = Vec::new();
    
    // First extract all PTS values (timestamps)
    for cap in pts_re.captures_iter(&output_str) {
        if let Some(ts) = cap.get(1) {
            if let Ok(time) = ts.as_str().parse::<f64>() {
                pts_values.push(time);
            }
        }
    }
    
    // Extract all packet sizes (rough indicator of frame complexity)
    for cap in pkt_size_re.captures_iter(&output_str) {
        if let Some(size) = cap.get(1) {
            if let Ok(size_val) = size.as_str().parse::<u32>() {
                pkt_sizes.push(size_val);
            }
        }
    }
    
    debug!("Found {} pts values and {} packet sizes", pts_values.len(), pkt_sizes.len());
    
    // Create a list of frames with timestamps
    let frame_count = pts_values.len().min(pkt_sizes.len());
    for i in 0..frame_count {
        frames.push((pts_values[i], pkt_sizes[i]));
    }
    
    // Now analyze frames to detect scene changes based on packet size changes
    // This is similar to content-based scene detection
    let mut scene_timestamps = Vec::new();
    if frames.len() >= 5 { // Need at least a few frames to detect changes
        let mut prev_sizes: Vec<u32> = Vec::new();
        for i in 5..frames.len() {
            // Calculate average of previous 5 frames' sizes
            prev_sizes.clear();
            for j in (i-5)..i {
                prev_sizes.push(frames[j].1);
            }
            let avg_prev = prev_sizes.iter().sum::<u32>() as f64 / 5.0;
            
            // If current frame is significantly different from previous average
            // (threshold% change), consider it a scene change
            let current = frames[i].1 as f64;
            let change_ratio = (current - avg_prev).abs() / avg_prev;
            
            if change_ratio > threshold_normalized as f64 {
                debug!("Detected scene change at {}: change ratio {}, threshold {}", 
                       frames[i].0, change_ratio, threshold_normalized);
                scene_timestamps.push(frames[i].0);
            }
        }
    }
    
    // Minimum 1 second between scene changes (filter out rapid flashes)
    let mut filtered_timestamps = Vec::new();
    if !scene_timestamps.is_empty() {
        filtered_timestamps.push(scene_timestamps[0]);
        let mut last = scene_timestamps[0];
        
        for &ts in &scene_timestamps[1..] {
            if ts - last > 1.0 {
                filtered_timestamps.push(ts);
                last = ts;
            }
        }
    }
    
    debug!("Basic method detected {} raw scene changes, {} after filtering", 
          scene_timestamps.len(), filtered_timestamps.len());
    
    debug!("Basic method found {} scenes", filtered_timestamps.len());
    Ok(filtered_timestamps)
}

/// Detect scenes using the scdet filter
fn detect_with_scdet<P: AsRef<Path>>(input_path: P, threshold: f32) -> Result<Vec<f64>> {
    let input_path = input_path.as_ref();
    let threshold_normalized = threshold / 100.0;
    
    // Use scdet filter which is specifically designed for scene detection
    // Note: 's' is a boolean parameter, not a resolution parameter
    let scene_filter = format!("scdet=threshold={}:sc_pass=1,metadata=print", threshold_normalized);
    
    let mut cmd = Command::new("ffmpeg");
    cmd.args([
        "-hide_banner",
        "-loglevel", "info",
        "-i", input_path.to_str().unwrap_or_default(),
        "-vf", &scene_filter,
        "-fps_mode", "passthrough",
        "-f", "null", "-"
    ]);
    
    // Run the command
    let output = command::run_command(&mut cmd)?;
    
    // Parse the output to extract scene change timestamps
    let output_str = String::from_utf8_lossy(&output.stderr);
    
    // The scdet filter with metadata=print outputs information about scene changes
    // We need to look for these patterns in the output
    let time_re = Regex::new(r"lavfi\.time=(\d+\.\d+)").unwrap();
    let scene_re = Regex::new(r"lavfi\.scdet\.scene_score=(\d+\.\d+)").unwrap();
    
    // First collect all timestamps and scores
    let mut timestamps = Vec::new();
    let mut scores = Vec::new();
    
    for cap in time_re.captures_iter(&output_str) {
        if let Some(ts_match) = cap.get(1) {
            if let Ok(timestamp) = ts_match.as_str().parse::<f64>() {
                timestamps.push(timestamp);
            }
        }
    }
    
    for cap in scene_re.captures_iter(&output_str) {
        if let Some(score_match) = cap.get(1) {
            if let Ok(score) = score_match.as_str().parse::<f64>() {
                scores.push(score);
            }
        }
    }
    
    // Build scene timestamps from matching pairs
    let mut scene_timestamps = Vec::new();
    let min_score = (threshold / 100.0) as f64;
    
    debug!("Found {} timestamps and {} scores", timestamps.len(), scores.len());
    
    // Use whichever set is smaller to avoid index out of bounds
    let len = timestamps.len().min(scores.len());
    for i in 0..len {
        if scores[i] >= min_score {
            debug!("Detected scene at timestamp: {} with score {}", timestamps[i], scores[i]);
            scene_timestamps.push(timestamps[i]);
        }
    }
    
    // If no scenes were detected, log the output for debugging
    if scene_timestamps.is_empty() {
        debug!("No scenes detected with scdet. Raw FFmpeg stderr output (truncated): {:.500}", output_str);
        debug!("No timestamps found. Regex patterns used: time='{}', scene='{}'", 
               r"lavfi\.time=(\d+\.\d+)", r"lavfi\.scdet\.scene_score=(\d+\.\d+)");
    }
    
    // Ensure timestamps are sorted
    scene_timestamps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    
    debug!("Found {} raw scene candidates", scene_timestamps.len());
    Ok(scene_timestamps)
}

/// Detect scenes using blackframe detection
fn detect_with_blackframe<P: AsRef<Path>>(input_path: P) -> Result<Vec<f64>> {
    let input_path = input_path.as_ref();
    
    // Use blackframe filter with more lenient settings to detect more potential transitions
    // Try to detect frames that are at least 80% black, with a higher noise threshold
    // This works well for more modern content without strict black frame transitions
    let mut cmd = Command::new("ffmpeg");
    cmd.args([
        "-hide_banner",
        "-loglevel", "info",
        "-i", input_path.to_str().unwrap_or_default(),
        "-vf", "blackframe=amount=0.80:threshold=48",
        "-f", "null", "-"
    ]);
    
    // Run command and parse output
    let output = command::run_command(&mut cmd)?;
    let output_str = String::from_utf8_lossy(&output.stderr);
    
    // Blackframe filter reports frames like: [Parsed_blackframe_0 @ 0x7f8a2c0008c0] frame:120 pblack:98 pts:120 t:4.800000
    let re = Regex::new(r"frame:\d+.*t:(\d+\.\d+)").unwrap();
    let mut timestamps = Vec::new();
    
    for cap in re.captures_iter(&output_str) {
        if let Some(ts) = cap.get(1) {
            if let Ok(time) = ts.as_str().parse::<f64>() {
                timestamps.push(time);
            }
        }
    }
    
    debug!("Blackframe method found {} scenes", timestamps.len());
    
    // Post-process blackframes to remove ones too close together
    let mut filtered = Vec::new();
    if !timestamps.is_empty() {
        filtered.push(timestamps[0]);
        let mut last = timestamps[0];
        for &ts in &timestamps[1..] {
            if ts - last > 1.0 { // Minimum 1 second between scene changes
                filtered.push(ts);
                last = ts;
            }
        }
    }
    
    Ok(filtered)
}

/// Filter scene candidates to ensure minimum distance between scenes
pub fn filter_scene_candidates(candidate_timestamps: Vec<f64>, min_gap: f32) -> Vec<f64> {
    let min_gap = min_gap as f64;
    let mut filtered = Vec::new();
    
    if candidate_timestamps.is_empty() {
        return filtered;
    }
    
    // Always include 0.0 as the first timestamp
    // This matches the Python implementation's behavior
    filtered.push(0.0);
    let mut last_ts = 0.0;
    
    // Skip the first timestamp if it's very close to 0.0
    let start_index = if !candidate_timestamps.is_empty() && candidate_timestamps[0] < 0.1 {
        debug!("First timestamp {} is close to 0.0, using exact 0.0 instead", candidate_timestamps[0]);
        1
    } else {
        0
    };
    
    // Add timestamps that are at least min_gap apart
    for ts in candidate_timestamps.into_iter().skip(start_index) {
        if ts - last_ts >= min_gap {
            filtered.push(ts);
            last_ts = ts;
        }
    }
    
    filtered
}

/// Insert additional boundaries when there are gaps exceeding max_segment_length
pub fn insert_artificial_boundaries(filtered_scenes: Vec<f64>, total_duration: f64, max_segment_length: f32) -> Vec<f64> {
    let max_length = max_segment_length as f64;
    let mut final_boundaries = Vec::new();
    
    if filtered_scenes.is_empty() {
        return final_boundaries;
    }
    
    // Add the first scene boundary
    final_boundaries.push(filtered_scenes[0]);
    let mut prev_boundary = filtered_scenes[0];
    
    // Process the rest of the scenes
    for ts in filtered_scenes.into_iter().skip(1) {
        let gap = ts - prev_boundary;
        
        // If gap is too large, insert artificial boundaries
        if gap > max_length {
            let num_inserts = (gap / max_length).ceil() as usize - 1;
            for i in 1..=num_inserts {
                let inserted = prev_boundary + (gap * i as f64 / (num_inserts + 1) as f64);
                final_boundaries.push(inserted);
            }
        }
        
        final_boundaries.push(ts);
        prev_boundary = ts;
    }
    
    // Check if additional boundaries needed before end of video
    if total_duration - prev_boundary > max_length {
        let gap = total_duration - prev_boundary;
        let num_inserts = (gap / max_length).ceil() as usize - 1;
        
        for i in 1..=num_inserts {
            let inserted = prev_boundary + (gap * i as f64 / (num_inserts + 1) as f64);
            final_boundaries.push(inserted);
        }
    }
    
    // Sort the final boundaries to ensure they're in order
    final_boundaries.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    
    final_boundaries
}

/// Detect scenes in a video file using configuration from the Config struct
///
/// This is a convenience wrapper around `detect_scenes` that uses the configuration
/// values from the provided Config struct.
///
/// # Arguments
///
/// * `input_file` - Path to the input video file
/// * `config` - The Config struct containing scene detection parameters
///
/// # Returns
///
/// * `Result<Vec<f64>>` - Vector of scene change timestamps in seconds
pub fn detect_scenes_with_config<P: AsRef<Path>>(
    input_file: P,
    config: &Config,
) -> Result<Vec<f64>> {
    detect_scenes(
        input_file,
        config.scene_threshold,
        config.hdr_scene_threshold,
        config.min_segment_length,
        config.max_segment_length,
    )
}

/// Validate segment boundaries against scene change points
pub fn validate_segment_boundaries<P: AsRef<Path>>(
    segments_dir: P,
    scene_timestamps: &[f64],
    min_duration: f64,
    scene_tolerance: f64,
) -> Result<Vec<(std::path::PathBuf, bool)>> {
    let segments_path = segments_dir.as_ref();
    let mut short_segments = Vec::new();
    
    // Validate segments directory exists
    if !segments_path.exists() || !segments_path.is_dir() {
        return Err(DraptoError::MediaFile(
            format!("Segments directory not found: {:?}", segments_path)
        ));
    }
    
    // Get all .mkv files in the directory
    let paths = std::fs::read_dir(segments_path)?;
    let mut segments = Vec::new();
    
    for path_result in paths {
        let path = path_result?.path();
        if path.extension().is_some_and(|ext| ext == "mkv") {
            segments.push(path);
        }
    }
    
    // Sort segments by filename (assuming sequential naming)
    segments.sort();
    
    // Check each segment's duration
    let mut cumulative_duration = 0.0;
    
    for segment_path in segments {
        match MediaInfo::from_path(&segment_path) {
            Ok(info) => {
                if let Some(duration) = info.duration() {
                    if duration < min_duration {
                        let segment_end = cumulative_duration + duration;
                        
                        // Check if this segment end aligns with a scene change
                        let is_scene = scene_timestamps.iter()
                            .any(|&scene_time| (scene_time - segment_end).abs() <= scene_tolerance);
                        
                        if is_scene {
                            info!("Short segment {} ({:.2}s) aligns with scene change", 
                                 segment_path.file_name().unwrap_or_default().to_string_lossy(), duration);
                        } else {
                            warn!("Short segment {} ({:.2}s) does not align with scene changes", 
                                  segment_path.file_name().unwrap_or_default().to_string_lossy(), duration);
                        }
                        
                        short_segments.push((segment_path, is_scene));
                    }
                    
                    cumulative_duration += duration;
                }
            },
            Err(e) => {
                error!("Failed to validate segment {:?}: {}", segment_path, e);
            }
        }
    }
    
    Ok(short_segments)
}

/// Validate segment boundaries using a standard tolerance value
///
/// This is a convenience wrapper around `validate_segment_boundaries` that uses
/// a standard tolerance value of 0.5 seconds.
///
/// # Arguments
///
/// * `segments_dir` - Directory containing video segments
/// * `scene_timestamps` - List of scene change timestamps
/// * `min_duration` - Minimum acceptable segment duration
///
/// # Returns
///
/// * `Result<Vec<(PathBuf, bool)>>` - List of tuples (segment_path, is_scene_boundary)
///   for segments shorter than min_duration
pub fn validate_segments<P: AsRef<Path>>(
    segments_dir: P,
    scene_timestamps: &[f64],
    min_duration: f64,
) -> Result<Vec<(std::path::PathBuf, bool)>> {
    validate_segment_boundaries(segments_dir, scene_timestamps, min_duration, 0.5)
}