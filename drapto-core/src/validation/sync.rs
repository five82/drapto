use crate::media::MediaInfo;
use super::report::ValidationReport;

/// Validate A/V synchronization
pub fn validate_sync(media_info: &MediaInfo, report: &mut ValidationReport) {
    // Get primary video stream information
    let video_stream = media_info.primary_video_stream();
    
    // Try to get video duration from the stream first, then fallback to format duration
    let video_duration = video_stream
        .and_then(|s| s.properties.get("duration")
            .and_then(|d| d.as_str())
            .and_then(|d| d.parse::<f64>().ok())
        )
        .or_else(|| media_info.duration());
    
    let video_start = video_stream
        .and_then(|s| s.properties.get("start_time")
            .and_then(|t| t.as_str())
            .and_then(|t| t.parse::<f64>().ok())
        )
        .unwrap_or(0.0);
    
    // Get primary audio stream information
    let audio_streams = media_info.audio_streams();
    let audio_stream = audio_streams.first();
    
    // Try to get audio duration from the stream first, then use the format duration as fallback
    let audio_duration = audio_stream
        .and_then(|s| s.properties.get("duration")
            .and_then(|d| d.as_str())
            .and_then(|d| d.parse::<f64>().ok())
        )
        .or_else(|| media_info.duration());
    
    let audio_start = audio_stream
        .and_then(|s| s.properties.get("start_time")
            .and_then(|t| t.as_str())
            .and_then(|t| t.parse::<f64>().ok())
        )
        .unwrap_or(0.0);
    
    // Validate streams exist
    if video_stream.is_none() {
        report.add_error("No video stream found for A/V sync validation", "AV Sync");
        return;
    }
    
    if audio_stream.is_none() {
        report.add_warning("No audio stream found for A/V sync validation", "AV Sync");
        return;
    }
    
    // Threshold in milliseconds (100ms = 0.1s as in Python version)
    const THRESHOLD_MS: i64 = 100;
    
    // Calculate start time difference in milliseconds
    let start_diff_ms = ((video_start - audio_start).abs() * 1000.0).round() as i64;
    
    // Check for start time mismatch
    if start_diff_ms > THRESHOLD_MS {
        report.add_error(
            format!("A/V sync start time error: video_start={:.2}s, audio_start={:.2}s, difference={}ms exceeds threshold", 
                    video_start, audio_start, start_diff_ms),
            "AV Sync"
        );
        return;
    }
    
    // Check for duration mismatch
    match (video_duration, audio_duration) {
        (Some(vdur), Some(adur)) => {
            // Convert to milliseconds for more precise comparison
            let video_dur_ms = (vdur * 1000.0).round() as i64;
            let audio_dur_ms = (adur * 1000.0).round() as i64;
            
            // Calculate difference in milliseconds
            let dur_diff_ms = (video_dur_ms - audio_dur_ms).abs();
            
            if dur_diff_ms > THRESHOLD_MS {
                report.add_error(
                    format!("A/V sync duration error: video_dur={:.2}s, audio_dur={:.2}s, difference={}ms exceeds threshold", 
                            vdur, adur, dur_diff_ms),
                    "AV Sync"
                );
            } else {
                report.add_info(
                    format!("A/V sync OK. Start time diff: {}ms, Duration diff: {}ms", 
                            start_diff_ms, dur_diff_ms),
                    "AV Sync"
                );
            }
        },
        (None, Some(_)) => {
            report.add_error("Video duration not available for A/V sync check", "AV Sync");
        },
        (Some(_), None) => {
            report.add_error("Audio duration not available for A/V sync check", "AV Sync");
        },
        (None, None) => {
            report.add_error("Neither video nor audio duration available for A/V sync check", "AV Sync");
        }
    }
}