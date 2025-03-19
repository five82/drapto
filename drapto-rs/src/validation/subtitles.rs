use crate::ffprobe::MediaInfo;
use crate::ffprobe::media::StreamInfo;
use super::ValidationReport;

/// Validate subtitle streams
pub fn validate_subtitles(media_info: &MediaInfo, report: &mut ValidationReport) {
    let subtitle_streams = media_info.subtitle_streams();
    
    if subtitle_streams.is_empty() {
        report.add_info("No subtitle streams found", "Subtitles");
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