use crate::ffprobe::MediaInfo;
use super::ValidationReport;

/// Validate video stream properties
pub fn validate_video(media_info: &MediaInfo, report: &mut ValidationReport) {
    validate_video_codec(media_info, report);
    validate_video_dimensions(media_info, report);
    validate_video_framerate(media_info, report);
    validate_video_duration(media_info, report);
}

/// Validate video codec
fn validate_video_codec(media_info: &MediaInfo, report: &mut ValidationReport) {
    let video_streams = media_info.video_streams();
    
    if video_streams.is_empty() {
        report.add_error("No video streams found", "Video");
        return;
    }
    
    for (i, stream) in video_streams.iter().enumerate() {
        let codec_name = &stream.codec_name;
        
        // Check if codec is AV1
        if codec_name.contains("av1") {
            report.add_info(
                format!("Video stream #{} has AV1 codec: {}", i, codec_name),
                "Video Codec"
            );
        } else {
            report.add_warning(
                format!("Video stream #{} has non-AV1 codec: {}", i, codec_name),
                "Video Codec"
            );
        }
    }
}

/// Validate video dimensions
fn validate_video_dimensions(media_info: &MediaInfo, report: &mut ValidationReport) {
    if let Some(dimensions) = media_info.video_dimensions() {
        let (width, height) = dimensions;
        
        report.add_info(
            format!("Video dimensions: {}x{}", width, height),
            "Video Dimensions"
        );
        
        // Check if dimensions are valid
        if width < 16 || height < 16 {
            report.add_error(
                format!("Video dimensions too small: {}x{}", width, height),
                "Video Dimensions"
            );
        }
        
        // Check if dimensions are even (required by many codecs)
        if width % 2 != 0 || height % 2 != 0 {
            report.add_warning(
                format!("Video dimensions not divisible by 2: {}x{}", width, height),
                "Video Dimensions"
            );
        }
    } else {
        report.add_error(
            "Could not determine video dimensions",
            "Video Dimensions"
        );
    }
}

/// Validate video framerate
fn validate_video_framerate(media_info: &MediaInfo, report: &mut ValidationReport) {
    if let Some(stream) = media_info.primary_video_stream() {
        // Try to get avg_frame_rate first, then fall back to r_frame_rate
        let framerate_str = stream.properties.get("avg_frame_rate")
            .or_else(|| stream.properties.get("r_frame_rate"))
            .and_then(|v| v.as_str());
        
        if let Some(framerate) = framerate_str {
            // Parse framerate in the form "num/den"
            let parts: Vec<&str> = framerate.split('/').collect();
            if parts.len() == 2 {
                if let (Ok(num), Ok(den)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                    if den > 0.0 {
                        let fps = num / den;
                        report.add_info(
                            format!("Video framerate: {:.3} fps", fps),
                            "Video Framerate"
                        );
                        
                        // Check for very low or high framerates
                        if fps < 10.0 {
                            report.add_warning(
                                format!("Low video framerate: {:.3} fps", fps),
                                "Video Framerate"
                            );
                        } else if fps > 120.0 {
                            report.add_warning(
                                format!("High video framerate: {:.3} fps", fps),
                                "Video Framerate"
                            );
                        }
                        
                        return;
                    }
                }
            }
            
            report.add_warning(
                format!("Could not parse video framerate: {}", framerate),
                "Video Framerate"
            );
        } else {
            report.add_warning(
                "Video framerate information not available",
                "Video Framerate"
            );
        }
    } else {
        report.add_error(
            "No primary video stream found for framerate validation",
            "Video Framerate"
        );
    }
}

/// Validate video duration
fn validate_video_duration(media_info: &MediaInfo, report: &mut ValidationReport) {
    if let Some(duration) = media_info.duration() {
        report.add_info(
            format!("Video duration: {:.3} seconds", duration),
            "Video Duration"
        );
        
        // Check for very short videos
        if duration < 0.5 {
            report.add_warning(
                format!("Very short video duration: {:.3} seconds", duration),
                "Video Duration"
            );
        }
    } else {
        report.add_warning(
            "Video duration information not available",
            "Video Duration"
        );
    }
}