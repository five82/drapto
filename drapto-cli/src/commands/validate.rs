use std::path::PathBuf;
use log::{info, warn};
use drapto_core::error::Result;
use drapto_core::validation;
use drapto_core::media::MediaInfo;
use drapto_core::detection::format::{has_dolby_vision, has_hdr};

use crate::output::{print_heading, print_section, print_info, print_validation_report, print_success, print_warning};

/// Execute the validate command
pub fn execute_validate(
    input: PathBuf,
    reference: Option<PathBuf>,
    target_score: f32
) -> Result<()> {
    print_heading("Media Validation");
    print_info("Input file", input.display());
    
    // Get media info
    info!("Retrieving media information from {}", input.display());
    let media_info = MediaInfo::from_path(&input)?;
    
    // Print basic media info
    print_section("Media Information");
    if let Some(format) = &media_info.format {
        print_info("Format", &format.format_name);
        if let Some(duration) = &format.duration {
            print_info("Duration", format!("{:.2} seconds", duration));
        }
        if let Some(bit_rate) = &format.bit_rate {
            print_info("Bitrate", format!("{} bps", bit_rate));
        }
    }
    
    // Print stream info
    print_section("Streams");
    let video_streams = media_info.video_streams();
    let audio_streams = media_info.audio_streams();
    let subtitle_streams = media_info.subtitle_streams();
    
    print_info("Video Streams", video_streams.len());
    print_info("Audio Streams", audio_streams.len());
    print_info("Subtitle Streams", subtitle_streams.len());
    
    // Print detailed stream info
    for stream in &media_info.streams {
        let stream_label = format!("Stream #{} ({})", stream.index, stream.codec_type);
        let stream_value = format!("{} {}", 
            stream.codec_name,
            stream.codec_long_name.as_deref().unwrap_or(""));
        print_info(&stream_label, &stream_value);
        
        // Print additional details for video streams
        if stream.codec_type == drapto_core::media::StreamType::Video {
            if let (Some(width), Some(height)) = (
                stream.properties.get("width").and_then(|w| w.as_u64()),
                stream.properties.get("height").and_then(|h| h.as_u64())
            ) {
                print_info(&format!("  Resolution"), &format!("{}x{}", width, height));
            }
            
            if let Some(fps) = stream.properties.get("r_frame_rate").and_then(|r| r.as_str()) {
                if fps.contains("/") {
                    if let Some((num, den)) = fps.split_once("/") {
                        if let (Ok(n), Ok(d)) = (num.parse::<f64>(), den.parse::<f64>()) {
                            if d > 0.0 {
                                print_info(&format!("  Framerate"), &format!("{:.2} fps", n / d));
                            }
                        }
                    }
                }
            }
            
            // Check HDR info
            if has_hdr(&media_info) {
                print_info(&format!("  HDR"), "Yes");
                
                // Print color space info if available
                if let Some(color_space) = stream.properties.get("color_space").and_then(|c| c.as_str()) {
                    print_info(&format!("  Color Space"), color_space);
                }
                
                if let Some(color_transfer) = stream.properties.get("color_transfer").and_then(|c| c.as_str()) {
                    print_info(&format!("  Color Transfer"), color_transfer);
                }
                
                if let Some(color_primaries) = stream.properties.get("color_primaries").and_then(|c| c.as_str()) {
                    print_info(&format!("  Color Primaries"), color_primaries);
                }
            }
            
            // Check Dolby Vision
            if has_dolby_vision(&media_info) {
                print_info(&format!("  Dolby Vision"), "Yes");
            }
        }
        
        // Print additional details for audio streams
        if stream.codec_type == drapto_core::media::StreamType::Audio {
            if let Some(channels) = stream.properties.get("channels").and_then(|c| c.as_u64()) {
                print_info(&format!("  Channels"), channels);
            }
            
            if let Some(sample_rate) = stream.properties.get("sample_rate").and_then(|s| s.as_str()) {
                if let Ok(rate) = sample_rate.parse::<u32>() {
                    print_info(&format!("  Sample Rate"), &format!("{} Hz", rate));
                }
            }
            
            // Print language if available
            if let Some(lang) = stream.tags.get("language") {
                print_info(&format!("  Language"), lang);
            }
        }
        
        // Print additional details for subtitle streams
        if stream.codec_type == drapto_core::media::StreamType::Subtitle {
            // Print language if available
            if let Some(lang) = stream.tags.get("language") {
                print_info(&format!("  Language"), lang);
            }
            
            // Print title if available
            if let Some(title) = stream.tags.get("title") {
                print_info(&format!("  Title"), title);
            }
        }
    }
    
    // Run validation
    print_section("Validation Results");
    info!("Running media validation");
    let report = validation::validate_media(&input)?;
    
    // Print validation report
    print_validation_report(&report);
    
    // Also run A/V sync validation
    info!("Running A/V sync validation");
    let av_report = validation::validate_av_sync(&input)?;
    print_validation_report(&av_report);
    
    // Run quality validation if a reference file is provided
    if let Some(ref_path) = reference {
        print_section("Quality Validation");
        print_info("Reference file", ref_path.display());
        print_info("Target VMAF score", target_score);
        
        // Check if VMAF is available
        if validation::quality::is_vmaf_available() {
            info!("Running VMAF quality validation");
            
            let mut quality_report = validation::ValidationReport::new();
            match validation::quality::validate_vmaf(&ref_path, &input, target_score, &mut quality_report) {
                Ok(score) => {
                    print_info("VMAF Score", format!("{:.2}", score));
                    print_validation_report(&quality_report);
                },
                Err(e) => {
                    print_warning(&format!("VMAF validation failed: {}", e));
                }
            }
        } else {
            print_warning("VMAF validation not available - libvmaf not found in FFmpeg");
            warn!("VMAF validation requires FFmpeg compiled with libvmaf");
        }
    }
    
    print_success("Validation completed");
    
    Ok(())
}