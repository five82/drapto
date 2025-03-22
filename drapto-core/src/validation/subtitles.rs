use std::path::Path;
use crate::media::MediaInfo;
use crate::media::StreamInfo;
use crate::error::Result;
use super::ValidationReport;

/// Validate subtitle streams
pub fn validate_subtitles(media_info: &MediaInfo, report: &mut ValidationReport) {
    let subtitle_streams = media_info.subtitle_streams();
    
    if subtitle_streams.is_empty() {
        report.add_info(
            format!("No subtitle streams found"),
            "Subtitles"
        );
        return;
    }
    
    report.add_info(
        format!("Found {} subtitle stream(s)", subtitle_streams.len()),
        "Subtitles"
    );
    
    for (i, stream) in subtitle_streams.iter().enumerate() {
        validate_subtitle_stream(i, stream, report);
    }
}

/// Validate a single subtitle stream
fn validate_subtitle_stream(index: usize, stream: &StreamInfo, report: &mut ValidationReport) {
    // Check codec
    let codec_name = &stream.codec_name;
    report.add_info(
        format!("Subtitle stream #{} codec: {}", index, codec_name),
        "Subtitles"
    );
    
    // Check language if available
    if let Some(lang) = stream.tags.get("language") {
        report.add_info(
            format!("Subtitle stream #{} language: {}", index, lang),
            "Subtitles"
        );
    } else {
        report.add_warning(
            format!("Subtitle stream #{} has no language tag", index),
            "Subtitles"
        );
    }
    
    // Check if it's a forced subtitle
    let is_forced = stream.tags.get("DISPOSITION:forced")
        .map(|v| v == "1")
        .unwrap_or(false);
    
    if is_forced {
        report.add_info(
            format!("Subtitle stream #{} is forced", index),
            "Subtitles"
        );
    }
    
    // Check title if available
    if let Some(title) = stream.tags.get("title") {
        report.add_info(
            format!("Subtitle stream #{} title: {}", index, title),
            "Subtitles"
        );
    }
    
    // Validate specific codec properties
    match codec_name.as_str() {
        "subrip" | "srt" => {
            report.add_info(
                format!("Subtitle stream #{} is text-based (SRT)", index),
                "Subtitles"
            );
        },
        "ass" | "ssa" => {
            report.add_info(
                format!("Subtitle stream #{} is styled text (ASS/SSA)", index),
                "Subtitles"
            );
        },
        "dvd_subtitle" | "dvdsub" => {
            report.add_info(
                format!("Subtitle stream #{} is bitmap-based (DVD)", index),
                "Subtitles"
            );
        },
        "hdmv_pgs_subtitle" | "pgssub" => {
            report.add_info(
                format!("Subtitle stream #{} is bitmap-based (PGS)", index),
                "Subtitles"
            );
        },
        "dvb_subtitle" | "dvbsub" => {
            report.add_info(
                format!("Subtitle stream #{} is bitmap-based (DVB)", index),
                "Subtitles"
            );
        },
        _ => {
            report.add_info(
                format!("Subtitle stream #{} uses codec: {}", index, codec_name),
                "Subtitles"
            );
        }
    }
}

/// Compare subtitle tracks between original and encoded files
pub fn compare_subtitles<P1, P2>(
    input_path: P1,
    output_path: P2,
    report: &mut ValidationReport
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    // Get media info for both files
    let input_media = MediaInfo::from_path(input_path.as_ref())?;
    let output_media = MediaInfo::from_path(output_path.as_ref())?;
    
    // Get subtitle streams
    let input_subs = input_media.subtitle_streams();
    let output_subs = output_media.subtitle_streams();
    
    // Check if subtitle tracks were preserved
    if !input_subs.is_empty() && output_subs.is_empty() {
        if input_subs.len() == 1 {
            report.add_error(
                "Subtitle track was lost during encoding",
                "Subtitles"
            );
        } else {
            report.add_error(
                format!("All {} subtitle tracks were lost during encoding", input_subs.len()),
                "Subtitles"
            );
        }
        return Ok(());
    }
    
    // Compare number of subtitle tracks
    if input_subs.len() != output_subs.len() {
        report.add_warning(
            format!(
                "Subtitle track count changed: {} → {}",
                input_subs.len(),
                output_subs.len()
            ),
            "Subtitles"
        );
    } else {
        report.add_info(
            format!("{} subtitle tracks preserved", input_subs.len()),
            "Subtitles"
        );
    }
    
    // Compare language tags
    let mut input_langs = Vec::new();
    let mut output_langs = Vec::new();
    
    for stream in &input_subs {
        if let Some(lang) = stream.tags.get("language") {
            input_langs.push(lang.clone());
        }
    }
    
    for stream in &output_subs {
        if let Some(lang) = stream.tags.get("language") {
            output_langs.push(lang.clone());
        }
    }
    
    // Check if all languages are preserved
    let mut missing_langs = Vec::new();
    for lang in &input_langs {
        if !output_langs.contains(lang) {
            missing_langs.push(lang.clone());
        }
    }
    
    if !missing_langs.is_empty() {
        report.add_warning(
            format!("Missing subtitle language(s): {}", missing_langs.join(", ")),
            "Subtitles"
        );
    }
    
    // Check for forced subtitles
    let input_has_forced = input_subs.iter().any(|s| {
        s.tags.get("DISPOSITION:forced").map(|v| v == "1").unwrap_or(false)
    });
    
    let output_has_forced = output_subs.iter().any(|s| {
        s.tags.get("DISPOSITION:forced").map(|v| v == "1").unwrap_or(false)
    });
    
    if input_has_forced && !output_has_forced {
        report.add_warning(
            "Forced subtitle flag was lost during encoding",
            "Subtitles"
        );
    } else if input_has_forced && output_has_forced {
        report.add_info(
            "Forced subtitle flag preserved",
            "Subtitles"
        );
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::media::{MediaInfo, StreamInfo, StreamType};

    #[test]
    fn test_validate_subtitles_empty() {
        let media_info = MediaInfo {
            format: None,
            streams: vec![],
            chapters: vec![],
        };
        
        let mut report = ValidationReport::new();
        validate_subtitles(&media_info, &mut report);
        
        assert_eq!(report.messages.len(), 1);
        assert!(report.messages[0].message.contains("No subtitle streams found"));
    }
    
    #[test]
    fn test_validate_subtitles_with_stream() {
        let mut tags = HashMap::new();
        tags.insert("language".to_string(), "eng".to_string());
        tags.insert("title".to_string(), "English".to_string());
        
        let srt_stream = StreamInfo {
            index: 2,
            codec_type: StreamType::Subtitle,
            codec_name: "subrip".to_string(),
            codec_long_name: Some("SubRip subtitle".to_string()),
            properties: HashMap::new(),
            tags,
        };
        
        let media_info = MediaInfo {
            format: None,
            streams: vec![srt_stream],
            chapters: vec![],
        };
        
        let mut report = ValidationReport::new();
        validate_subtitles(&media_info, &mut report);
        
        assert!(report.messages.len() >= 4); // Should have multiple info messages
        
        // Verify presence of expected messages
        let message_texts: Vec<&str> = report.messages.iter()
            .map(|m| m.message.as_str())
            .collect();
            
        assert!(message_texts.iter().any(|m| m.contains("1 subtitle stream")));
        assert!(message_texts.iter().any(|m| m.contains("language: eng")));
        assert!(message_texts.iter().any(|m| m.contains("title: English")));
        assert!(message_texts.iter().any(|m| m.contains("text-based (SRT)")));
    }
}