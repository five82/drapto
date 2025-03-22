use log::info;
use drapto_core::error::Result;
use drapto_core::media::probe::FFprobe;
use drapto_core::util::command;
use std::process::{Command, Stdio};
use num_cpus;

use crate::output::{print_heading, print_section, print_info, print_success, print_error};

/// Execute the FFmpeg info command
pub fn execute_ffmpeg_info() -> Result<()> {
    print_heading("FFmpeg Information");
    
    // Check FFmpeg availability
    info!("Checking FFmpeg availability");
    let ffprobe = FFprobe::new();
    
    match ffprobe.get_version() {
        Ok(version) => {
            print_info("FFmpeg Version", version);
            
            // Check available decoders
            print_section("Available Decoders");
            match ffprobe.get_decoders() {
                Ok(decoders) => {
                    // Filter video decoders - (variable intentionally unused)
                    let _video_decoders: Vec<_> = decoders.iter()
                        .filter(|d| d.type_name == "video")
                        .take(15) // Limit to first 15 to avoid clutter
                        .collect();
                    
                    // Filter key video codecs
                    let key_video_decoders = ["h264", "hevc", "av1", "vp9", "mpeg2"];
                    let important_video_decoders: Vec<_> = decoders.iter()
                        .filter(|d| d.type_name == "video" && key_video_decoders.iter().any(|k| d.name.contains(k)))
                        .collect();
                    
                    // Filter audio decoders - (variable intentionally unused)
                    let _audio_decoders: Vec<_> = decoders.iter()
                        .filter(|d| d.type_name == "audio")
                        .take(15) // Limit to first 15 to avoid clutter
                        .collect();
                    
                    // Filter key audio codecs
                    let key_audio_decoders = ["aac", "opus", "flac", "mp3", "ac3", "eac3"];
                    let important_audio_decoders: Vec<_> = decoders.iter()
                        .filter(|d| d.type_name == "audio" && key_audio_decoders.iter().any(|k| d.name.contains(k)))
                        .collect();
                    
                    print_info("Important Video Decoders", 
                        important_video_decoders.iter().map(|d| d.name.clone()).collect::<Vec<_>>().join(", "));
                    
                    print_info("Important Audio Decoders", 
                        important_audio_decoders.iter().map(|d| d.name.clone()).collect::<Vec<_>>().join(", "));
                },
                Err(e) => print_error(&format!("Failed to get decoders: {}", e)),
            }
            
            // Check available encoders
            print_section("Available Encoders");
            match ffprobe.get_encoders() {
                Ok(encoders) => {
                    // Filter key video encoders
                    let key_video_encoders = ["h264", "hevc", "av1", "vp9", "mpeg2"];
                    let video_encoders: Vec<_> = encoders.iter()
                        .filter(|e| e.type_name == "video" && key_video_encoders.iter().any(|k| e.name.contains(k)))
                        .collect();
                    
                    // Filter key audio encoders
                    let key_audio_encoders = ["aac", "opus", "flac", "mp3", "ac3", "eac3"];
                    let audio_encoders: Vec<_> = encoders.iter()
                        .filter(|e| e.type_name == "audio" && key_audio_encoders.iter().any(|k| e.name.contains(k)))
                        .collect();
                    
                    print_info("Video Encoders", 
                        video_encoders.iter().map(|e| e.name.clone()).collect::<Vec<_>>().join(", "));
                    
                    print_info("Audio Encoders", 
                        audio_encoders.iter().map(|e| e.name.clone()).collect::<Vec<_>>().join(", "));
                },
                Err(e) => print_error(&format!("Failed to get encoders: {}", e)),
            }
            
            // Check hardware acceleration
            print_section("Hardware Acceleration (For Decoding Only)");
            match ffprobe.get_hwaccels() {
                Ok(hwaccels) => {
                    if hwaccels.is_empty() {
                        print_info("Available Hardware Accelerators", "None");
                    } else {
                        print_info("Available Hardware Accelerators", hwaccels.join(", "));
                        
                        // Check for specific decoders we support
                        let has_vaapi = hwaccels.iter().any(|h| h == "vaapi");
                        let has_videotoolbox = hwaccels.iter().any(|h| h == "videotoolbox");
                        
                        if has_vaapi {
                            print_info("VAAPI (Linux)", "Available");
                        }
                        
                        if has_videotoolbox {
                            print_info("VideoToolbox (macOS)", "Available");
                        }
                        
                        if !has_vaapi && !has_videotoolbox {
                            print_info("Supported Hardware Decoders", "None");
                        }
                        
                        // Check what we'll actually use
                        match ffprobe.check_hardware_decoding() {
                            Ok(Some(option)) => {
                                print_info("Selected Hardware Decoder", 
                                    if option.contains("vaapi") { "VAAPI" } else { "VideoToolbox" });
                            },
                            _ => {
                                print_info("Selected Hardware Decoder", "None");
                            }
                        }
                    }
                },
                Err(e) => print_error(&format!("Failed to get hardware accelerators: {}", e)),
            }
            
            // Check for HDR and Dolby Vision capabilities
            print_section("Format Support");
            print_info("HDR Detection", "Available");
            print_info("Dolby Vision Detection", "Available");
            print_info("AV1 Support", check_av1_support() as u8);
            
            // System information
            print_section("System Information");
            print_info("CPU Cores", num_cpus::get());
            
            print_success("FFmpeg information retrieved successfully");
        },
        Err(e) => {
            print_error(&format!("Failed to get FFmpeg version: {}", e));
            return Err(e);
        }
    }
    
    Ok(())
}

/// Check if AV1 support is available
fn check_av1_support() -> bool {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-hide_banner", "-encoders"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    
    match command::run_command(&mut cmd) {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.contains(" av1 ") || stdout.contains("libaom-av1") || stdout.contains("libsvtav1")
        },
        Err(_) => false,
    }
}