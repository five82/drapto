//! Black bar detection and crop parameter generation.
//!
//! This module implements a simple and efficient crop detection algorithm
//! inspired by alabamaEncoder. It samples multiple points throughout the video
//! and uses the most common crop result.

use crate::error::CoreResult;
use crate::processing::video_properties::VideoProperties;
// Removed unused log::info import
use std::path::Path;
use std::collections::HashMap;
use rayon::prelude::*;

/// Detects black bars by sampling 141 points from 15-85% of video. Returns crop filter and HDR status.
pub fn detect_crop(
    input_file: &Path,
    video_props: &VideoProperties,
    disable_crop: bool,
) -> CoreResult<(Option<String>, bool)> {
    // Check if crop detection is disabled
    if disable_crop {
        crate::progress_reporting::report_operation_complete("Crop detection", "Detected crop", "Disabled");
        return Ok((None, false));
    }

    // Detect HDR content
    let color_space = video_props.color_space.as_deref().unwrap_or("");
    let is_hdr = color_space == "bt2020nc" || color_space == "bt2020c";
    
    // Set threshold based on content type
    let threshold = if is_hdr { 100 } else { 16 };

    // Sample every 0.5% from 15% to 85% (141 points total)
    // Avoids first/last 15% where intros/credits typically appear
    let sample_points: Vec<f64> = (30..=170)
        .map(|i| i as f64 / 200.0)
        .collect();
    let mut crop_results = HashMap::new();
    
    log::debug!(
        "Sampling crop at {} points throughout the video (15% to 85%, every 0.5%)",
        sample_points.len()
    );
    
    // Process samples in parallel for faster detection
    let crops: Vec<Option<String>> = sample_points
        .par_iter()
        .map(|&position| {
            let start_time = video_props.duration_secs * position;
            sample_crop_at_position(input_file, start_time, threshold)
                .unwrap_or(None)
        })
        .collect();
    
    // Count the results
    for crop in crops.into_iter().flatten() {
        *crop_results.entry(crop).or_insert(0) += 1;
    }
    
    // Analyze crop results
    let (best_crop, has_multiple_ratios) = if crop_results.is_empty() {
        (None, false)
    } else if crop_results.len() == 1 {
        // Only one crop detected - use it
        (crop_results.into_iter().next().map(|(crop, _)| crop), false)
    } else {
        // Multiple crops detected - check if they're significantly different
        let total_samples: usize = crop_results.values().sum();
        let mut sorted_crops: Vec<(String, usize)> = crop_results.into_iter().collect();
        sorted_crops.sort_by(|a, b| b.1.cmp(&a.1));
        
        let (most_common_crop, most_common_count) = &sorted_crops[0];
        let ratio = *most_common_count as f64 / total_samples as f64;
        
        // If one crop is dominant (>80% of samples), use it
        if ratio > 0.8 {
            (Some(most_common_crop.clone()), false)
        } else {
            // Multiple significant aspect ratios detected
            crate::progress_reporting::warning(&format!(
                "Multiple aspect ratios detected in {}",
                input_file.display()
            ));
            for (crop, count) in &sorted_crops {
                let percentage = (count * 100) / total_samples;
                crate::progress_reporting::status(&format!("  {}", crop), &format!("{}% of samples", percentage), false);
            }
            
            // Conservative approach: don't crop at all for mixed aspect ratio content
            crate::progress_reporting::info("Using conservative approach - no cropping for mixed aspect ratio content");
            (None, true)
        }
    };
    
    // Report results
    match &best_crop {
        Some(crop) => {
            crate::progress_reporting::report_operation_complete("Crop detection", "Detected crop", crop);
            log::debug!("Applied crop filter: {}", crop);
        }
        None => {
            if has_multiple_ratios {
                crate::progress_reporting::report_operation_complete("Crop detection", "Detected crop", "Multiple ratios (no crop)");
                log::debug!("Multiple aspect ratios detected - no cropping applied");
            } else {
                crate::progress_reporting::report_operation_complete("Crop detection", "Detected crop", "None required");
                log::debug!("No cropping needed for {}", input_file.display());
            }
        }
    }
    
    Ok((best_crop, is_hdr))
}

/// Sample crop detection at a specific position in the video.
fn sample_crop_at_position(
    input_file: &Path,
    start_time: f64,
    threshold: u32,
) -> CoreResult<Option<String>> {
    log::trace!(
        "Sampling crop at {:.1}s with threshold {}",
        start_time,
        threshold
    );
    
    let mut cmd = crate::external::FfmpegCommandBuilder::new()
        .with_hardware_accel(true)
        .build();
    
    // Start at the specified time
    cmd.args(["-ss", &format!("{:.2}", start_time)]);
    
    // Input file
    cmd.input(input_file.to_string_lossy());
    
    // Cropdetect filter and output
    cmd.args([
        "-vframes", "10",  // Analyze 10 frames
        "-vf", &format!("cropdetect=limit={threshold}:round=2:reset=1"),
        "-f", "null",
        "-"
    ]);
    
    // Spawn and collect output
    let mut child = cmd
        .spawn()
        .map_err(|e| crate::error::command_start_error("ffmpeg", e))?;
    
    let mut crop_output = String::new();
    
    for event in child.iter().map_err(|e| {
        crate::error::command_failed_error(
            "ffmpeg",
            std::process::ExitStatus::default(),
            e.to_string(),
        )
    })? {
        if let ffmpeg_sidecar::event::FfmpegEvent::Log(_, line) = event {
            if line.contains("crop=") {
                crop_output.push_str(&line);
                crop_output.push('\n');
            }
        }
    }
    
    // Parse the most common crop from this sample
    parse_crop_from_output(&crop_output)
}

/// Parse crop values from ffmpeg output.
fn parse_crop_from_output(output: &str) -> CoreResult<Option<String>> {
    let mut crop_counts: HashMap<String, usize> = HashMap::new();
    
    // Extract all crop values
    for line in output.lines() {
        if let Some(crop_pos) = line.find("crop=") {
            let crop_part = &line[crop_pos + 5..];
            
            // Find the end of the crop value (space or end of line)
            let end_pos = crop_part
                .find(|c: char| c.is_whitespace())
                .unwrap_or(crop_part.len());
            
            let crop_value = &crop_part[..end_pos];
            
            // Validate it's a proper crop format (w:h:x:y)
            if is_valid_crop_format(crop_value) {
                *crop_counts.entry(crop_value.to_string()).or_insert(0) += 1;
            }
        }
    }
    
    // Return the most common crop value
    Ok(crop_counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(crop, _)| format!("crop={}", crop)))
}

/// Validate that a crop string is in the format w:h:x:y with valid numbers.
fn is_valid_crop_format(crop: &str) -> bool {
    let parts: Vec<&str> = crop.split(':').collect();
    
    if parts.len() != 4 {
        return false;
    }
    
    // All parts must be valid numbers
    parts.iter().all(|part| part.parse::<u32>().is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_crop_format() {
        // Valid formats
        assert!(is_valid_crop_format("1920:1080:0:0"));
        assert!(is_valid_crop_format("1920:800:0:140"));
        assert!(is_valid_crop_format("3840:2160:0:0"));
        assert!(is_valid_crop_format("100:200:10:20"));
        
        // Invalid formats - wrong number of parts
        assert!(!is_valid_crop_format("1920:1080:0"));
        assert!(!is_valid_crop_format("1920:1080:0:0:0"));
        assert!(!is_valid_crop_format("1920"));
        assert!(!is_valid_crop_format(""));
        
        // Invalid formats - non-numeric values
        assert!(!is_valid_crop_format("1920:1080:0:a"));
        assert!(!is_valid_crop_format("width:height:x:y"));
        assert!(!is_valid_crop_format("1920:1080:0:-10"));
        assert!(!is_valid_crop_format("1920.5:1080:0:0"));
        
        // Edge cases
        assert!(is_valid_crop_format("0:0:0:0"));
        assert!(is_valid_crop_format("4294967295:4294967295:4294967295:4294967295")); // max u32
    }

    #[test]
    fn test_parse_crop_from_output() {
        // Test with single crop value
        let output = "[Parsed_cropdetect_0 @ 0x7f8] x1:0 x2:1919 y1:140 y2:939 w:1920 h:800 x:0 y:140 pts:0 t:0.000000 crop=1920:800:0:140\n";
        assert_eq!(
            parse_crop_from_output(output).unwrap(),
            Some("crop=1920:800:0:140".to_string())
        );
        
        // Test with multiple identical crop values
        let output = "[Parsed_cropdetect_0 @ 0x7f8] crop=1920:800:0:140\n\
                     [Parsed_cropdetect_0 @ 0x7f8] crop=1920:800:0:140\n\
                     [Parsed_cropdetect_0 @ 0x7f8] crop=1920:800:0:140\n";
        assert_eq!(
            parse_crop_from_output(output).unwrap(),
            Some("crop=1920:800:0:140".to_string())
        );
        
        // Test with multiple different crop values (most common wins)
        let output = "[Parsed_cropdetect_0 @ 0x7f8] crop=1920:800:0:140\n\
                     [Parsed_cropdetect_0 @ 0x7f8] crop=1920:800:0:140\n\
                     [Parsed_cropdetect_0 @ 0x7f8] crop=1920:1080:0:0\n\
                     [Parsed_cropdetect_0 @ 0x7f8] crop=1920:800:0:140\n";
        assert_eq!(
            parse_crop_from_output(output).unwrap(),
            Some("crop=1920:800:0:140".to_string())
        );
        
        // Test with no crop values
        let output = "[Parsed_cropdetect_0 @ 0x7f8] x1:0 x2:1919 y1:0 y2:1079\n\
                     Some other ffmpeg output without crop\n";
        assert_eq!(parse_crop_from_output(output).unwrap(), None);
        
        // Test with invalid crop formats (should be ignored)
        let output = "[Parsed_cropdetect_0 @ 0x7f8] crop=invalid:format\n\
                     [Parsed_cropdetect_0 @ 0x7f8] crop=1920:800:0:140\n";
        assert_eq!(
            parse_crop_from_output(output).unwrap(),
            Some("crop=1920:800:0:140".to_string())
        );
        
        // Test with crop value at end of line (no trailing space)
        let output = "[Parsed_cropdetect_0 @ 0x7f8] crop=1920:800:0:140";
        assert_eq!(
            parse_crop_from_output(output).unwrap(),
            Some("crop=1920:800:0:140".to_string())
        );
        
        // Test with crop value followed by other parameters
        let output = "[Parsed_cropdetect_0 @ 0x7f8] crop=1920:800:0:140 pts:1234 t:1.234";
        assert_eq!(
            parse_crop_from_output(output).unwrap(),
            Some("crop=1920:800:0:140".to_string())
        );
        
        // Test empty output
        assert_eq!(parse_crop_from_output("").unwrap(), None);
    }
}