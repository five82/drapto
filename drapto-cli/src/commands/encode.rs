use std::path::PathBuf;
use std::fs;
use std::time::Instant;
use log::{info, debug};
use drapto_core::error::Result;
use drapto_core::Config;
use drapto_core::media::MediaInfo;
use drapto_core::detection::format::{detect_dolby_vision, has_hdr, detect_crop};
use drapto_core::detection::scene::detect_scenes;
use drapto_core::encoding::video::{encode_video, VideoEncodingOptions};
use drapto_core::encoding::audio::{encode_audio, AudioEncodingOptions};
use drapto_core::encoding::muxer::{Muxer, MuxOptions};
use num_cpus;

use crate::output::{print_heading, print_section, print_info, print_error, print_success, print_progress, print_separator, print_warning};

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
    let mut config = Config {
        input: input.clone(),
        output: output.clone(),
        hardware_acceleration: !no_hwaccel,
        target_quality: quality,
        parallel_jobs: jobs.unwrap_or_else(num_cpus::get),
        verbose,
        keep_temp_files: keep_temp,
        temp_dir: temp_dir.unwrap_or_else(std::env::temp_dir),
        disable_crop,
        ..Default::default()
    };
    
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
    
    // Check for HDR and Dolby Vision
    let is_hdr = has_hdr(&media_info);
    if is_hdr {
        print_info("HDR", "Yes");
        // Adjust scene detection threshold for HDR content
        config.scene_threshold = config.hdr_scene_threshold;
    } else {
        print_info("HDR", "No");
    }
    
    // Use dedicated mediainfo-based Dolby Vision detection
    let is_dolby_vision = detect_dolby_vision(&input);
    if is_dolby_vision {
        print_info("Dolby Vision", "Yes");
    } else {
        print_info("Dolby Vision", "No");
    }
    
    // Detect black bars for cropping if not disabled
    let mut crop_filter = None;
    if !config.disable_crop {
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
    print_section("Scene Detection");
    info!("Detecting scenes in video");
    
    print_info("Scene Detection Threshold", config.scene_threshold);
    print_info("Minimum Segment Length", format!("{} seconds", config.min_segment_length));
    print_info("Maximum Segment Length", format!("{} seconds", config.max_segment_length));
    
    print_progress("Detecting scenes...")?;
    
    let scenes = detect_scenes(
        &input, 
        config.scene_threshold,
        config.hdr_scene_threshold,
        config.min_segment_length,
        config.max_segment_length
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
    let working_dir = config.temp_dir.join(format!("drapto_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&working_dir)?;
    debug!("Created working directory: {}", working_dir.display());
    
    // Encode video
    print_section("Video Encoding");
    print_info("Parallel Jobs", config.parallel_jobs);
    print_info("Hardware Acceleration", if config.hardware_acceleration { "Enabled" } else { "Disabled" });
    
    let video_options = VideoEncodingOptions {
        quality: config.target_quality,
        parallel_jobs: config.parallel_jobs,
        hardware_acceleration: config.hardware_acceleration,
        crop_filter,
        scenes: Some(scenes),
        is_hdr,
        is_dolby_vision,
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
    };
    
    print_progress("Encoding audio tracks...")?;
    let audio_outputs = encode_audio(&input, &audio_options)?;
    print_success(&format!("Encoded {} audio tracks", audio_outputs.len()));
    
    // Mux video and audio
    print_section("Muxing");
    let muxer = Muxer::new();
    let mux_options = MuxOptions::default();
    
    print_progress("Muxing tracks...")?;
    muxer.mux_tracks(&video_output, &audio_outputs, &output, &mux_options)?;
    print_success(&format!("Successfully muxed to: {}", output.display()));
    
    // Validate output file
    print_section("Validation");
    print_progress("Validating output...")?;
    
    // In a real implementation, we would do more validation here
    let output_size = fs::metadata(&output)?.len();
    let input_size = fs::metadata(&input)?.len();
    let size_reduction = (1.0 - (output_size as f64 / input_size as f64)) * 100.0;
    
    print_info("Input Size", format_size(input_size));
    print_info("Output Size", format_size(output_size));
    print_info("Size Reduction", format!("{:.2}%", size_reduction));
    
    // Cleanup
    if !config.keep_temp_files {
        print_progress("Cleaning up temporary files...")?;
        if let Err(e) = fs::remove_dir_all(&working_dir) {
            print_warning(&format!("Failed to clean up temporary files: {}", e));
        }
    }
    
    // Calculate elapsed time
    let elapsed = start_time.elapsed();
    let hours = elapsed.as_secs() / 3600;
    let minutes = (elapsed.as_secs() % 3600) / 60;
    let seconds = elapsed.as_secs() % 60;
    
    print_section("Summary");
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
    
    for (index, input_file) in video_files.iter().enumerate() {
        let filename = input_file.file_name().unwrap_or_default();
        let output_file = output_dir.join(filename);
        
        print_heading(&format!("Processing File {}/{}: {}", index + 1, video_files.len(), filename.to_string_lossy()));
        
        match execute_encode(
            input_file.clone(),
            output_file,
            quality,
            jobs,
            no_hwaccel,
            keep_temp,
            temp_dir.clone(),
            disable_crop,
            verbose,
        ) {
            Ok(_) => {
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
    
    // Print summary
    print_heading("Directory Encoding Summary");
    print_success(&format!("Total files processed: {}", video_files.len()));
    print_success(&format!("Successfully encoded: {}", successful_files));
    
    if failed_files > 0 {
        print_error(&format!("Failed to encode: {}", failed_files));
    }
    
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