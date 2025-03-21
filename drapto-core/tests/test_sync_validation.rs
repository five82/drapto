use std::collections::HashMap;
use serde_json::Value;
use drapto_core::media::{MediaInfo, StreamInfo, StreamType};
use drapto_core::validation::{ValidationReport, sync};

fn create_test_media_info(video_start: Option<f64>, video_duration: Option<f64>, 
                         audio_start: Option<f64>, audio_duration: Option<f64>) -> MediaInfo {
    // Create test streams
    let mut video_props = HashMap::new();
    let mut audio_props = HashMap::new();
    
    if let Some(vs) = video_start {
        video_props.insert("start_time".to_string(), Value::String(vs.to_string()));
    }
    
    if let Some(vd) = video_duration {
        video_props.insert("duration".to_string(), Value::String(vd.to_string()));
    }
    
    if let Some(as_) = audio_start {
        audio_props.insert("start_time".to_string(), Value::String(as_.to_string()));
    }
    
    if let Some(ad) = audio_duration {
        audio_props.insert("duration".to_string(), Value::String(ad.to_string()));
    }
    
    let video_stream = StreamInfo {
        index: 0,
        codec_type: StreamType::Video,
        codec_name: "h264".to_string(),
        codec_long_name: Some("H.264 / AVC / MPEG-4 AVC".to_string()),
        tags: HashMap::new(),
        properties: video_props,
    };
    
    let audio_stream = StreamInfo {
        index: 1,
        codec_type: StreamType::Audio,
        codec_name: "aac".to_string(),
        codec_long_name: Some("AAC (Advanced Audio Coding)".to_string()),
        tags: HashMap::new(),
        properties: audio_props,
    };
    
    let mut streams = Vec::new();
    streams.push(video_stream);
    streams.push(audio_stream);
    
    MediaInfo {
        format: None,
        streams,
        chapters: Vec::new(),
    }
}

#[test]
fn test_sync_validation_perfect() {
    // Create media info with perfectly synced audio and video
    let media_info = create_test_media_info(
        Some(0.0),       // video start
        Some(60.0),      // video duration
        Some(0.0),       // audio start
        Some(60.0),      // audio duration
    );
    
    let mut report = ValidationReport::new();
    sync::validate_sync(&media_info, &mut report);
    
    // Should not have errors
    assert!(report.passed);
    assert_eq!(report.errors().len(), 0);
    
    // Should have info message about sync being ok
    let info_messages = report.infos();
    assert!(info_messages.len() > 0);
    assert!(info_messages[0].message.contains("A/V sync OK"));
}

#[test]
fn test_sync_validation_start_time_mismatch() {
    // Create media info with mismatched start times
    let media_info = create_test_media_info(
        Some(0.0),        // video start
        Some(60.0),       // video duration
        Some(0.2),        // audio start (200ms difference, above threshold)
        Some(60.0),       // audio duration
    );
    
    let mut report = ValidationReport::new();
    sync::validate_sync(&media_info, &mut report);
    
    // Should have failed
    assert!(!report.passed);
    
    // Should have error about start time mismatch
    let error_messages = report.errors();
    assert!(error_messages.len() > 0);
    assert!(error_messages[0].message.contains("start time error"));
}

#[test]
fn test_sync_validation_duration_mismatch() {
    // Create media info with mismatched durations
    let media_info = create_test_media_info(
        Some(0.0),        // video start
        Some(60.0),       // video duration
        Some(0.0),        // audio start
        Some(60.3),       // audio duration (300ms difference, above threshold)
    );
    
    let mut report = ValidationReport::new();
    sync::validate_sync(&media_info, &mut report);
    
    // Should have failed
    assert!(!report.passed);
    
    // Should have error about duration mismatch
    let error_messages = report.errors();
    assert!(error_messages.len() > 0);
    assert!(error_messages[0].message.contains("duration error"));
}

#[test]
fn test_sync_validation_missing_streams() {
    // Create media info with missing streams
    let mut media_info = create_test_media_info(
        Some(0.0),        // video start
        Some(60.0),       // video duration
        Some(0.0),        // audio start
        Some(60.0),       // audio duration
    );
    
    // Remove audio stream
    media_info.streams.remove(1);
    
    let mut report = ValidationReport::new();
    sync::validate_sync(&media_info, &mut report);
    
    // Should have failed
    assert!(!report.passed);
    
    // Should have error about missing audio stream
    let error_messages = report.errors();
    assert!(error_messages.len() > 0);
    assert!(error_messages[0].message.contains("No audio stream found"));
}

#[test]
fn test_sync_validation_missing_duration() {
    // Create media info with missing duration information
    let media_info = create_test_media_info(
        Some(0.0),        // video start
        None,             // video duration missing
        Some(0.0),        // audio start
        Some(60.0),       // audio duration
    );
    
    let mut report = ValidationReport::new();
    sync::validate_sync(&media_info, &mut report);
    
    // Should have failed
    assert!(!report.passed);
    
    // Should have error about missing video duration
    let error_messages = report.errors();
    assert!(error_messages.len() > 0);
    assert!(error_messages[0].message.contains("Video duration not available"));
}

#[test]
fn test_sync_validation_within_threshold() {
    // Create media info with difference within threshold (50ms)
    let media_info = create_test_media_info(
        Some(0.0),        // video start
        Some(60.0),       // video duration
        Some(0.05),       // audio start (50ms difference, below threshold)
        Some(60.03),      // audio duration (30ms difference, below threshold)
    );
    
    let mut report = ValidationReport::new();
    sync::validate_sync(&media_info, &mut report);
    
    // Should pass
    assert!(report.passed);
    
    // Should have info message
    let info_messages = report.infos();
    assert!(info_messages.len() > 0);
    assert!(info_messages[0].message.contains("A/V sync OK"));
}