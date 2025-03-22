use std::path::Path;
use std::fs::File;
use std::process::Command;
use tempfile::tempdir;

use drapto_core::encoding::muxer::{Muxer, MuxOptions};

// Helper function to create a dummy video file using ffmpeg
fn create_test_video(path: &Path, duration: u32) -> Result<(), Box<dyn std::error::Error>> {
    // Check if ffmpeg is available
    let status = Command::new("ffmpeg")
        .arg("-version")
        .status();
        
    if status.is_err() {
        return Ok(());  // Skip if ffmpeg not available
    }
    
    // Generate test video
    let status = Command::new("ffmpeg")
        .args([
            "-y",  // Overwrite output files without asking
            "-f", "lavfi",  // Use virtual input device
            "-i", &format!("testsrc=duration={}:size=640x360:rate=30", duration),  // Input pattern
            "-c:v", "libx264",  // Use H.264 codec
            "-pix_fmt", "yuv420p",  // Use YUV pixel format
            path.to_str().unwrap()  // Output path
        ])
        .status()?;
        
    if !status.success() {
        return Err("Failed to create test video".into());
    }
    
    Ok(())
}

// Helper function to create a dummy audio file using ffmpeg
fn create_test_audio(path: &Path, duration: u32) -> Result<(), Box<dyn std::error::Error>> {
    // Check if ffmpeg is available
    let status = Command::new("ffmpeg")
        .arg("-version")
        .status();
        
    if status.is_err() {
        return Ok(());  // Skip if ffmpeg not available
    }
    
    // Generate test audio
    let status = Command::new("ffmpeg")
        .args([
            "-y",  // Overwrite output files without asking
            "-f", "lavfi",  // Use virtual input device
            "-i", &format!("sine=frequency=440:duration={}", duration),  // Input pattern
            "-c:a", "libopus",  // Use Opus codec
            path.to_str().unwrap()  // Output path
        ])
        .status()?;
        
    if !status.success() {
        return Err("Failed to create test audio".into());
    }
    
    Ok(())
}

#[test]
fn test_build_mux_command() {
    // Create temporary directory for test files
    let temp_dir = tempdir().unwrap();
    
    // Create dummy file paths
    let video_path = temp_dir.path().join("video.mp4");
    let audio_path1 = temp_dir.path().join("audio1.opus");
    let audio_path2 = temp_dir.path().join("audio2.opus");
    let output_path = temp_dir.path().join("output.mkv");
    
    // Create empty files
    File::create(&video_path).unwrap();
    File::create(&audio_path1).unwrap();
    File::create(&audio_path2).unwrap();
    
    // Create muxer
    let muxer = Muxer::new();
    
    // Create vectors of paths for proper type handling
    let audio_paths: Vec<&Path> = vec![audio_path1.as_path(), audio_path2.as_path()];
    
    // Build command
    let result = muxer.build_mux_command(
        &video_path,
        &audio_paths,
        &output_path
    );
    
    // Verify command built successfully
    assert!(result.is_ok());
    
    let cmd = result.unwrap();
    
    // Convert args to strings for easier assertions
    let args: Vec<String> = cmd.get_args()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect();
    
    // Verify command structure
    assert!(args.contains(&"-map".to_string()));
    assert!(args.contains(&"0:v:0".to_string()));
    assert!(args.contains(&"1:a:0".to_string()));
    assert!(args.contains(&"2:a:0".to_string()));
    assert!(args.contains(&"copy".to_string()));
    assert!(args.contains(&output_path.to_string_lossy().to_string()));
}

#[test]
fn test_mux_options() {
    // Create custom options
    let options = MuxOptions {
        sync_threshold: 0.5,
        allow_container_duration: false,
    };
    
    let muxer = Muxer::with_options(options.clone());
    
    // Verify options were set correctly
    assert_eq!(muxer.options.sync_threshold, options.sync_threshold);
    assert_eq!(muxer.options.allow_container_duration, options.allow_container_duration);
}

#[test]
#[ignore] // Ignore by default as it requires ffmpeg to be installed
fn test_full_mux_with_ffmpeg() -> Result<(), Box<dyn std::error::Error>> {
    // Skip test if ffmpeg isn't available
    let ffmpeg_status = Command::new("ffmpeg").arg("-version").status();
    if ffmpeg_status.is_err() {
        println!("Skipping test_full_mux_with_ffmpeg, ffmpeg not found");
        return Ok(());
    }
    
    // Create temporary directory for test files
    let temp_dir = tempdir()?;
    
    // Define file paths
    let video_path = temp_dir.path().join("video.mp4");
    let audio_path = temp_dir.path().join("audio.opus");
    let output_path = temp_dir.path().join("output.mkv");
    
    // Create test media files
    create_test_video(&video_path, 5)?;
    create_test_audio(&audio_path, 5)?;
    
    // Create muxer
    let muxer = Muxer::new();
    
    // Create vector of paths for proper type handling
    let audio_paths: Vec<&Path> = vec![audio_path.as_path()];
    
    // Perform muxing
    let result = muxer.mux_tracks(&video_path, &audio_paths, &output_path, None);
    
    // Verify result
    assert!(result.is_ok(), "Muxing failed: {:?}", result.err());
    
    // Verify output file exists and has reasonable size
    assert!(output_path.exists());
    let metadata = std::fs::metadata(&output_path)?;
    assert!(metadata.len() > 1000, "Output file too small: {} bytes", metadata.len());
    
    Ok(())
}

#[test]
fn test_invalid_path_handling() {
    // Create temporary directory for output
    let temp_dir = tempdir().unwrap();
    let output_path = temp_dir.path().join("output.mkv");
    
    // Create muxer
    let muxer = Muxer::new();
    
    // Try with non-existent files
    let video_path = Path::new("/non/existent/video.mp4");
    let audio_path = Path::new("/non/existent/audio.opus");
    
    // Create vector of paths for proper type handling
    let audio_paths: Vec<&Path> = vec![audio_path];
    
    // Test should return invalid path error
    let result = muxer.mux_tracks(video_path, &audio_paths, &output_path, None);
    
    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_string = format!("{}", error);
    assert!(error_string.contains("does not exist"), 
            "Expected error about non-existent file, got: {}", error_string);
}