use std::path::{Path, PathBuf};
use std::process::Command;
use log::{debug, info, warn, error};

use crate::error::{DraptoError, Result};
use crate::media::MediaInfo;
use crate::detection::scene::{detect_scenes_with_config, validate_segments};
use crate::util::command;
use crate::config::Config;

/// Segmentation error types
#[derive(Debug, thiserror::Error)]
pub enum SegmentationError {
    #[error("No segments found in segments directory")]
    NoSegmentsFound,
    
    #[error("Failed to get input duration: {0}")]
    DurationError(String),
    
    #[error("Failed to validate segment {0}: {1}")]
    SegmentValidationError(String, String),
    
    #[error("Total segment duration ({total_segment:.2}s) differs significantly from input ({input_duration:.2}s)")]
    DurationMismatch { total_segment: f64, input_duration: f64 },
    
    #[error("Scene detection failed; no scenes detected")]
    NoScenesDetected,
    
    #[error("Segmentation failed: {0}")]
    Other(String),
}

/// Segment a video into chunks based on scene detection
///
/// This function:
/// 1. Detects scenes in the input video
/// 2. Uses FFmpeg to split the video at scene boundaries
/// 3. Validates the resulting segments
///
/// # Arguments
///
/// * `input_file` - Path to the input video file
/// * `segments_dir` - Directory to save the segments
/// * `config` - Configuration settings
///
/// # Returns
///
/// * `Result<Vec<PathBuf>>` - List of segment file paths if successful
pub fn segment_video<P1, P2>(
    input_file: P1,
    segments_dir: P2,
    config: &Config,
) -> Result<Vec<PathBuf>>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    // Ensure segments directory exists
    if !segments_dir.as_ref().exists() {
        std::fs::create_dir_all(segments_dir.as_ref())?;
    }
    
    // Get scene boundaries
    let scenes = detect_scenes_with_config(&input_file, config)?;
    if scenes.is_empty() {
        return Err(DraptoError::Segmentation(SegmentationError::NoScenesDetected));
    }
    
    crate::logging::log_subsection("SCENE DETECTION");
    info!("Segmenting video with {} detected scene boundaries", scenes.len());
    
    // Build segmentation command
    let segments = segment_video_at_scenes(&input_file, &segments_dir, &scenes, config)?;
    
    // Validate segments
    validate_segments_integrity(&input_file, &segments, &scenes, config)?;
    
    Ok(segments)
}

/// Create video segments at scene boundaries using FFmpeg
///
/// # Arguments
///
/// * `input_file` - Path to the input video file
/// * `segments_dir` - Directory to save the segments
/// * `scene_times` - List of scene change timestamps
/// * `config` - Configuration settings
///
/// # Returns
///
/// * `Result<Vec<PathBuf>>` - List of segment file paths if successful
pub fn segment_video_at_scenes<P1, P2>(
    input_file: P1,
    segments_dir: P2,
    scene_times: &[f64],
    config: &Config,
) -> Result<Vec<PathBuf>>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let input_path = input_file.as_ref();
    let segments_path = segments_dir.as_ref();
    
    // Get hardware acceleration option from config
    let hw_opt = &config.hw_accel_option;
    
    // Format scene times for FFmpeg segment_times option
    // Skip times < 1.0 to avoid FFmpeg issues with very early splits
    let segment_times = scene_times.iter()
        .filter(|&&t| t >= 1.0)
        .map(|&t| format!("{:.2}", t))
        .collect::<Vec<_>>()
        .join(",");
    
    // Build FFmpeg command
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-hide_banner", "-loglevel", "warning"]);
    
    // Add hardware acceleration if available
    if !hw_opt.is_empty() {
        for opt in hw_opt.split_whitespace() {
            cmd.arg(opt);
        }
    }
    
    // Add input file and segmentation parameters
    cmd.args([
        "-i", input_path.to_str().unwrap_or_default(),
        "-c:v", "copy",
        "-an",  // No audio
        "-f", "segment",
        "-segment_times", &segment_times,
        "-reset_timestamps", "1",
    ]);
    
    // Add output pattern
    let output_pattern = segments_path.join("%04d.mkv").to_str()
        .ok_or_else(|| DraptoError::InvalidPath("Invalid segments directory".to_string()))?
        .to_string();
    
    cmd.arg(&output_pattern);
    
    // Execute the command
    debug!("Running segmentation command: {:?}", cmd);
    let output = command::run_command(&mut cmd)?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Segmentation failed: {}", stderr);
        return Err(DraptoError::Segmentation(SegmentationError::Other(stderr.to_string())));
    }
    
    // Find all created segment files
    let mut segments = Vec::new();
    for entry in std::fs::read_dir(segments_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "mkv") {
            segments.push(path);
        }
    }
    
    // Sort segments by name (should be numbered sequentially)
    segments.sort();
    
    crate::logging::log_section("SEGMENTATION");
    crate::logging::log_subsection("SEGMENT CREATION");
    info!("Created {} video segments", segments.len());
    
    if segments.is_empty() {
        return Err(DraptoError::Segmentation(SegmentationError::NoSegmentsFound));
    }
    
    Ok(segments)
}

/// Validate the integrity of created segments
///
/// This function:
/// 1. Ensures all segments are valid
/// 2. Verifies total duration matches input file
/// 3. Checks that segment boundaries align with scene changes
///
/// # Arguments
///
/// * `input_file` - Original input video file
/// * `segments` - List of segment file paths
/// * `scene_times` - Scene change timestamps
/// * `config` - Configuration settings
///
/// # Returns
///
/// * `Result<()>` - Ok if all segments pass validation
fn validate_segments_integrity<P: AsRef<Path>>(
    input_file: P,
    segments: &[PathBuf],
    scene_times: &[f64],
    config: &Config,
) -> Result<()> {
    if segments.is_empty() {
        return Err(DraptoError::Segmentation(SegmentationError::NoSegmentsFound));
    }
    
    // Get input file duration
    let input_info = MediaInfo::from_path(input_file)?;
    let total_duration = input_info.duration().ok_or_else(|| {
        DraptoError::Segmentation(SegmentationError::DurationError("Could not determine input duration".to_string()))
    })?;
    
    // Validate each segment
    let mut total_segment_duration = 0.0;
    
    for segment in segments {
        match validate_single_segment(segment) {
            Ok(duration) => {
                total_segment_duration += duration;
                debug!("Segment {}: duration={:.2}s", 
                       segment.file_name().unwrap_or_default().to_string_lossy(), 
                       duration);
            },
            Err(e) => {
                let segment_name = segment.file_name().unwrap_or_default().to_string_lossy();
                return Err(DraptoError::Segmentation(
                    SegmentationError::SegmentValidationError(segment_name.to_string(), e.to_string())
                ));
            }
        }
    }
    
    // Check that total duration matches within tolerance
    let duration_tolerance = (total_duration * 0.02).max(1.0);
    if (total_segment_duration - total_duration).abs() > duration_tolerance {
        return Err(DraptoError::Segmentation(
            SegmentationError::DurationMismatch {
                total_segment: total_segment_duration,
                input_duration: total_duration,
            }
        ));
    }
    
    // Validate segment boundaries against scene changes
    let min_segment_duration = config.min_segment_length as f64;
    
    if let Ok(short_segments) = validate_segments(segments[0].parent().unwrap(), scene_times, min_segment_duration) {
        let problematic_segments = short_segments.iter()
            .filter(|(_, is_scene)| !is_scene)
            .count();
            
        if problematic_segments > 0 {
            warn!("Found {} problematic short segments not aligned with scene changes", 
                  problematic_segments);
        }
    }
    
    crate::logging::log_subsection("SEGMENT VALIDATION");
    info!("Successfully validated {} segments", segments.len());
    Ok(())
}

/// Validate a single segment file
///
/// Checks that:
/// 1. The segment file exists and is not empty
/// 2. The segment has valid video metadata
/// 3. The segment timestamp starts near 0
///
/// # Arguments
///
/// * `segment` - Path to the segment file
///
/// # Returns
///
/// * `Result<f64>` - Segment duration in seconds if valid
fn validate_single_segment<P: AsRef<Path>>(segment: P) -> Result<f64> {
    let segment_path = segment.as_ref();
    
    // Check if file exists and has content
    if !segment_path.exists() {
        return Err(DraptoError::MediaFile(format!("Segment file not found: {:?}", segment_path)));
    }
    
    let metadata = std::fs::metadata(segment_path)?;
    if metadata.len() < 1024 {  // 1KB minimum size
        return Err(DraptoError::MediaFile(format!("Segment file is too small: {:?}", segment_path)));
    }
    
    // Get segment media info
    let info = MediaInfo::from_path(segment_path)?;
    
    // Validate duration
    let duration = info.duration().ok_or_else(|| {
        DraptoError::MediaFile(format!("Could not determine segment duration: {:?}", segment_path))
    })?;
    
    if duration <= 0.0 {
        return Err(DraptoError::MediaFile(format!("Invalid segment duration: {:.2}", duration)));
    }
    
    // Validate video stream
    match info.primary_video_stream() {
        Some(stream) => {
            // Check start time is near 0
            let start_time = stream.properties.get("start_time")
                .and_then(|v| v.as_str())
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.0);
                
            if start_time.abs() > 0.2 {  // 200ms tolerance
                return Err(DraptoError::MediaFile(
                    format!("Segment timestamp issue: video_start={:.2}s is not near 0", start_time)
                ));
            }
            
            // Check codec is present
            if stream.codec_name.is_empty() {
                return Err(DraptoError::MediaFile(
                    format!("Invalid segment: missing codec information")
                ));
            }
        },
        None => {
            return Err(DraptoError::MediaFile(format!("No video stream found in segment")));
        }
    }
    
    Ok(duration)
}