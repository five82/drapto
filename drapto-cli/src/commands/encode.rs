//! Encode command implementation
//!
//! Responsibilities:
//! - Handle all video and audio encoding operations
//! - Process command-line arguments for encoding
//! - Handle file and directory encoding
//! - Coordinate detection, encoding, and muxing steps
//! - Report encoding progress and results
//!
//! This module implements the core encoding functionality accessed
//! through the CLI, orchestrating the encoding pipeline.

use std::path::PathBuf;
use std::fs;
use std::time::Instant;
use log::{info, debug};
use chrono;
use drapto_core::error::Result;
use drapto_core::Config;
use drapto_core::media::MediaInfo;
use drapto_core::detection::format::{has_hdr, detect_crop};
use drapto_core::detection::scene::detect_scenes;
use drapto_core::encoding::video::{encode_video, VideoEncodingOptions};
use drapto_core::encoding::audio::{encode_audio, AudioEncodingOptions};
use drapto_core::encoding::muxer::{Muxer, MuxOptions};
use num_cpus;

use crate::output::{print_heading, print_section, print_info, print_error, print_success, 
                   print_progress, print_separator, print_warning, print_validation_report};

/// Execute the encode command for a single file
pub fn execute_encode(
    input: PathBuf,
    output: PathBuf,
    quality: Option<f32>,
    jobs: Option<usize>,
    no_hwaccel: bool,
    keep_temp: bool,
    temp_dir: Option<PathBuf>,
    disable_crop: bool,
    verbose: bool,
    memory_per_job: Option<usize>,
) -> Result<()> {
    let start_time = Instant::now();
    
    print_heading("Video Encoding");
    print_info("Input", input.display());
    print_info("Output", output.display());
    
    if let Some(quality) = quality {
        print_info("Target Quality", quality);
    }
    
    // Create parent directory for output if it doesn't exist
    if let Some(parent) = output.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    
    // Create configuration
    let mut config = Config::default();
    config.input = input.clone();
    config.output = output.clone();
    config.video.hardware_acceleration = !no_hwaccel;
    config.video.target_quality = quality;
    config.resources.parallel_jobs = jobs.unwrap_or_else(num_cpus::get);
    config.logging.verbose = verbose;
    config.directories.keep_temp_files = keep_temp;
    config.directories.temp_dir = temp_dir.unwrap_or_else(std::env::temp_dir);
    config.video.disable_crop = disable_crop;
    
    // Apply memory limit if provided via CLI
    if let Some(memory_mb) = memory_per_job {
        config.resources.memory_per_job = memory_mb;
    }
    
    // Validate configuration
    info!("Validating configuration");
    if let Err(e) = config.validate() {
        print_error(&format!("Configuration validation failed: {}", e));
        return Err(e);
    }
    
    debug!("Configuration: {:?}", config);
    
    // Analyze input file
    info!("Analyzing input file");
    print_section("Input Analysis");
    
    let media_info = MediaInfo::from_path(&input)?;
    
    // Print basic media info
    if let Some(format) = &media_info.format {
        print_info("Format", &format.format_name);
        if let Some(duration) = &format.duration {
            print_info("Duration", format!("{:.2} seconds", duration));
        }
        if let Some(bit_rate) = &format.bit_rate {
            print_info("Bitrate", format!("{} bps", bit_rate));
        }
    }
    
    // Print video dimensions
    if let Some(dimensions) = media_info.video_dimensions() {
        print_info("Video dimensions", format!("{}x{}", dimensions.0, dimensions.1));
    }
    
    // Check for HDR content
    let is_hdr = has_hdr(&media_info);
    if is_hdr {
        print_info("HDR", "Yes");
        // Adjust scene detection threshold for HDR content
        config.scene_detection.scene_threshold = config.scene_detection.hdr_scene_threshold;
    } else {
        print_info("HDR", "No");
    }
    
    
    // Detect black bars for cropping if not disabled
    let mut crop_filter = None;
    if !config.video.disable_crop {
        print_info("Analyzing video for black bars", "");
        let crop_result = detect_crop(&input, None)?;
        if let (Some(filter), _) = crop_result {
            print_info("Crop filter", &filter);
            crop_filter = Some(filter);
        } else {
            print_info("Crop filter", "None detected");
        }
    } else {
        print_info("Crop detection", "Disabled");
    }
    
    // Detect scenes - this is a key part of the drapto pipeline
    // Get hardware acceleration options if enabled
    let hw_accel_option = if config.video.hardware_acceleration {
        // Set automatically based on platform using FFprobe 
        let ffprobe = drapto_core::media::probe::FFprobe::new();
        match ffprobe.check_hardware_decoding() {
            Ok(Some(option)) => {
                print_info("Hardware Acceleration", "Enabled");
                print_info("Hardware Decoder", if option.contains("vaapi") { "VAAPI" } else { "VideoToolbox" });
                Some(option)
            },
            _ => {
                print_info("Hardware Acceleration", "Enabled");
                print_info("Hardware Decoder", "None available");
                None
            }
        }
    } else {
        print_info("Hardware Acceleration", "Disabled");
        None
    };
    
    print_section("Scene Detection");
    info!("Detecting scenes in video");
    
    print_info("Scene Detection Threshold", config.scene_detection.scene_threshold);
    print_info("Minimum Segment Length", format!("{} seconds", config.scene_detection.min_segment_length));
    print_info("Maximum Segment Length", format!("{} seconds", config.scene_detection.max_segment_length));
    
    print_progress("Detecting scenes...")?;
    
    let scenes = detect_scenes(
        &input, 
        config.scene_detection.scene_threshold,
        config.scene_detection.hdr_scene_threshold,
        config.scene_detection.min_segment_length,
        config.scene_detection.max_segment_length
    )?;
    
    print_info("Detected Scenes", scenes.len());
    
    // Print some details about the scenes
    if !scenes.is_empty() {
        let first_scene = scenes.first().unwrap_or(&0.0);
        let last_scene = scenes.last().unwrap_or(&0.0);
        
        print_info("Scene Range", format!("{:.2}s - {:.2}s", first_scene, last_scene));
        
        // Calculate average segment length
        if scenes.len() > 1 {
            let mut total_length = 0.0;
            let mut prev_scene = 0.0;
            
            for scene in &scenes {
                total_length += scene - prev_scene;
                prev_scene = *scene;
            }
            
            // Get total duration from media info for the last segment
            if let Some(duration) = media_info.duration() {
                total_length += duration - prev_scene;
                let avg_length = total_length / (scenes.len() as f64 + 1.0);
                print_info("Average Segment Length", format!("{:.2} seconds", avg_length));
            }
        }
    }
    
    // Prepare temp directories
    let working_dir = config.directories.temp_dir.join(format!("drapto_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&working_dir)?;
    debug!("Created working directory: {}", working_dir.display());
    
    // Encode video
    print_section("Video Encoding");
    print_info("Parallel Jobs", config.resources.parallel_jobs);
    
    let video_options = VideoEncodingOptions {
        quality: config.video.target_quality,
        parallel_jobs: config.resources.parallel_jobs,
        hw_accel_option: hw_accel_option.clone(), // Clone here to avoid move
        crop_filter,
        scenes: Some(scenes),
        is_hdr,
        working_dir: working_dir.clone(),
    };
    
    debug!("Video encoding options: {:?}", video_options);
    print_progress("Encoding video...")?;
    
    let video_output = encode_video(&input, &video_options)?;
    print_success(&format!("Video encoded to: {} (preserving original filename)", video_output.display()));
    
    // Encode audio
    print_section("Audio Encoding");
    let audio_options = AudioEncodingOptions {
        working_dir: working_dir.clone(),
        quality: None, // Use default quality
        hw_accel_option: hw_accel_option.clone(), // Reuse the same hardware decoder settings
    };
    
    print_progress("Encoding audio tracks...")?;
    let audio_outputs = encode_audio(&input, &audio_options)?;
    print_success(&format!("Encoded {} audio tracks", audio_outputs.len()));
    
    // Mux video and audio
    print_section("Muxing");
    let muxer = Muxer::new();
    let mux_options = MuxOptions::default();
    
    print_progress("Muxing tracks...")?;
    let muxed_file = muxer.mux_tracks(&video_output, &audio_outputs, &output, &mux_options)?;
    print_success(&format!("Successfully muxed to: {}", muxed_file.display()));
    
    // IMPORTANT: Use the muxed_file path returned by the muxer for all subsequent operations
    // This is critical as the muxer may have modified the output path
    
    // Cleanup
    if !config.directories.keep_temp_files {
        print_progress("Cleaning up temporary files...")?;
        if let Err(e) = fs::remove_dir_all(&working_dir) {
            print_warning(&format!("Failed to clean up temporary files: {}", e));
        }
    }
    
    // Validate output file - do this after cleanup to ensure file is finalized
    print_section("Validation");
    print_progress("Validating output...")?;
    
    // Ensure the output file exists and is readable
    if !muxed_file.exists() {
        return Err(drapto_core::error::DraptoError::Other(
            format!("Output file not found: {}", muxed_file.display())
        ));
    }
    
    // Get file sizes using standard filesystem metadata
    let input_size = get_file_size(&input)?;
    let output_size = get_file_size(&muxed_file)?;
    let size_reduction = (1.0 - (output_size as f64 / input_size as f64)) * 100.0;
    
    print_info("Input Size", format_size(input_size));
    print_info("Output Size", format_size(output_size));
    print_info("Size Reduction", format!("{:.2}%", size_reduction));
    
    // Run comprehensive validation and print the report
    let validation_report = drapto_core::validation::validate_output(&input, &muxed_file, None)?;
    print_validation_report(&validation_report);
    
    // Calculate elapsed time
    let elapsed = start_time.elapsed();
    
    // Generate and display encoding summary
    print_heading("Encoding Summary");
    
    print_info("Input File", input.file_name().unwrap_or_default().to_string_lossy());
    print_info("Input Size", format_size(input_size));
    print_info("Output Size", format_size(output_size));
    print_info("Size Reduction", format!("{:.2}%", size_reduction));
    
    // Format encoding time
    let hours = elapsed.as_secs() / 3600;
    let minutes = (elapsed.as_secs() % 3600) / 60;
    let seconds = elapsed.as_secs() % 60;
    print_info("Encoding Time", format!("{:02}h {:02}m {:02}s", hours, minutes, seconds));
    
    // Use the finished timestamp
    let finished_time = chrono::Local::now().format("%a %b %d %H:%M:%S %Z %Y").to_string();
    print_info("Finished At", finished_time);
    
    print_separator();
    print_success(&format!("Encoding complete in {:02}h {:02}m {:02}s", hours, minutes, seconds));
    
    Ok(())
}

/// Execute the encode command for a directory of files
pub fn execute_encode_directory(
    input_dir: PathBuf,
    output_dir: PathBuf,
    quality: Option<f32>,
    jobs: Option<usize>,
    no_hwaccel: bool,
    keep_temp: bool,
    temp_dir: Option<PathBuf>,
    disable_crop: bool,
    verbose: bool,
    memory_per_job: Option<usize>,
) -> Result<()> {
    print_heading("Directory Encoding");
    print_info("Input Directory", input_dir.display());
    print_info("Output Directory", output_dir.display());

    // Create output directory if it doesn't exist
    if !output_dir.exists() {
        fs::create_dir_all(&output_dir)?;
    }
    
    // Find all video files in the input directory
    let mut video_files = Vec::new();
    for entry in fs::read_dir(&input_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        // Skip hidden files (those that start with a dot)
        if let Some(file_name) = path.file_name() {
            let file_name_str = file_name.to_string_lossy();
            if file_name_str.starts_with(".") {
                continue;
            }
        }
        
        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "mp4" || ext_str == "mkv" || ext_str == "mov" || ext_str == "avi" {
                    video_files.push(path);
                }
            }
        }
    }
    
    if video_files.is_empty() {
        print_warning("No video files found in directory");
        return Ok(());
    }
    
    print_info("Found Video Files", video_files.len());
    print_separator();
    
    // Process each file
    let mut successful_files = 0;
    let mut failed_files = 0;
    let batch_start_time = Instant::now();
    
    // Store encoding summaries for each file
    let mut encoding_summaries = Vec::new();
    let mut total_input_size = 0;
    let mut total_output_size = 0;
    
    for (index, input_file) in video_files.iter().enumerate() {
        let filename = input_file.file_name().unwrap_or_default();
        let output_file = output_dir.join(filename);
        
        print_heading(&format!("Processing File {}/{}: {}", index + 1, video_files.len(), filename.to_string_lossy()));
        
        match execute_encode(
            input_file.clone(),
            output_file.clone(),
            quality,
            jobs,
            no_hwaccel,
            keep_temp,
            temp_dir.clone(),
            disable_crop,
            verbose,
            memory_per_job,
        ) {
            Ok(_) => {
                // Store summary information
                // We need to get the actual file paths from the last successful encode
                let actual_output_file = if let Ok(entries) = std::fs::read_dir(&output_dir) {
                    let mut output_file_path = output_file.clone();
                    for entry in entries {
                        if let Ok(entry) = entry {
                            if entry.file_name().to_string_lossy() == filename.to_string_lossy() {
                                output_file_path = entry.path();
                                break;
                            }
                        }
                    }
                    output_file_path
                } else {
                    output_file.clone()
                };
                
                if let (Ok(input_size), Ok(output_size)) = (get_file_size(input_file), get_file_size(&actual_output_file)) {
                    let reduction = ((input_size as f64 - output_size as f64) / input_size as f64) * 100.0;
                    
                    encoding_summaries.push((
                        filename.to_string_lossy().to_string(),
                        input_size,
                        output_size,
                        reduction
                    ));
                    
                    total_input_size += input_size;
                    total_output_size += output_size;
                }
                
                print_success(&format!("Successfully encoded {}", filename.to_string_lossy()));
                successful_files += 1;
            },
            Err(e) => {
                print_error(&format!("Failed to encode {}: {}", filename.to_string_lossy(), e));
                failed_files += 1;
            }
        }
        
        print_separator();
    }
    
    // Calculate total batch duration
    let batch_elapsed = batch_start_time.elapsed();
    let batch_hours = batch_elapsed.as_secs() / 3600;
    let batch_minutes = (batch_elapsed.as_secs() % 3600) / 60;
    let batch_seconds = batch_elapsed.as_secs() % 60;
    
    // Calculate overall reduction
    let overall_reduction = if total_input_size > 0 {
        ((total_input_size as f64 - total_output_size as f64) / total_input_size as f64) * 100.0
    } else {
        0.0
    };
    
    // Print final batch summary
    print_heading("Final Encoding Summary");
    
    // Print individual file summaries
    for (filename, input_size, output_size, reduction) in &encoding_summaries {
        print_separator();
        print_info("File", filename);
        print_info("Input Size", format_size(*input_size));
        print_info("Output Size", format_size(*output_size));
        print_info("Reduction", format!("{:.2}%", reduction));
    }
    
    print_separator();
    
    // Print overall batch stats
    print_info("Total Files Processed", video_files.len());
    print_info("Successfully Encoded", successful_files);
    if failed_files > 0 {
        print_error(&format!("Failed to encode: {}", failed_files));
    }
    
    print_info("Total Input Size", format_size(total_input_size));
    print_info("Total Output Size", format_size(total_output_size));
    print_info("Overall Reduction", format!("{:.2}%", overall_reduction));
    print_info("Total Execution Time", format!("{:02}h {:02}m {:02}s", batch_hours, batch_minutes, batch_seconds));
    
    print_separator();
    print_success(&format!("Batch encoding complete in {:02}h {:02}m {:02}s", batch_hours, batch_minutes, batch_seconds));
    
    Ok(())
}

/// Format file size in human-readable form
fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

/// Get file size using the filesystem's stat (same method used in the Python code)
fn get_file_size(path: &PathBuf) -> Result<u64> {
    // Simple stat approach, matching the Python implementation
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.len()),
        Err(e) => Err(drapto_core::error::DraptoError::Io(e))
    }
}