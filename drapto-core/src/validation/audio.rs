//! Audio validation module
//!
//! Responsibilities:
//! - Validate audio codec compliance and compatibility
//! - Verify audio channel configuration
//! - Check audio sample rate and bit depth
//! - Validate audio stream duration
//! - Ensure audio quality meets required standards
//!
//! This module provides functions to validate various aspects of
//! audio streams to ensure they meet encoding specifications.

use crate::media::MediaInfo;
use crate::config::Config;
use super::report::ValidationReport;

/// Validate audio stream properties
pub fn validate_audio(media_info: &MediaInfo, report: &mut ValidationReport, config: Option<&Config>) {
    // Use default config if none provided
    let default_config = Config::default();
    let config = config.unwrap_or(&default_config);
    
    validate_audio_codec(media_info, report);
    validate_audio_channels(media_info, report);
    validate_audio_duration(media_info, report, config);
    validate_audio_sample_rate(media_info, report, config);
}

/// Validate audio codec
fn validate_audio_codec(media_info: &MediaInfo, report: &mut ValidationReport) {
    let audio_streams = media_info.audio_streams();
    
    if audio_streams.is_empty() {
        report.add_warning("No audio streams found", "Audio");
        return;
    }
    
    for (i, stream) in audio_streams.iter().enumerate() {
        let codec_name = &stream.codec_name;
        
        // Check if codec is Opus (preferred)
        if codec_name.contains("opus") {
            report.add_info(
                format!("Audio stream #{} has Opus codec", i),
                "Audio Codec"
            );
        } else if codec_name.contains("aac") || codec_name.contains("vorbis") {
            report.add_info(
                format!("Audio stream #{} has acceptable codec: {}", i, codec_name),
                "Audio Codec"
            );
        } else {
            report.add_warning(
                format!("Audio stream #{} has non-optimal codec: {}", i, codec_name),
                "Audio Codec"
            );
        }
    }
}

/// Validate audio channels
fn validate_audio_channels(media_info: &MediaInfo, report: &mut ValidationReport) {
    let audio_streams = media_info.audio_streams();
    
    if audio_streams.is_empty() {
        return; // Already warned in validate_audio_codec
    }
    
    for (i, stream) in audio_streams.iter().enumerate() {
        let channels = stream.properties.get("channels")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        
        if channels == 0 {
            report.add_warning(
                format!("Could not determine channel count for audio stream #{}", i),
                "Audio Channels"
            );
            continue;
        }
        
        report.add_info(
            format!("Audio stream #{} has {} channel(s)", i, channels),
            "Audio Channels"
        );
        
        // Check for common channel layouts
        match channels {
            1 => {
                report.add_info(
                    format!("Audio stream #{} is mono", i),
                    "Audio Channels"
                );
            },
            2 => {
                report.add_info(
                    format!("Audio stream #{} is stereo", i),
                    "Audio Channels"
                );
            },
            6 => {
                report.add_info(
                    format!("Audio stream #{} is 5.1 surround", i),
                    "Audio Channels"
                );
            },
            8 => {
                report.add_info(
                    format!("Audio stream #{} is 7.1 surround", i),
                    "Audio Channels"
                );
            },
            _ => {
                report.add_info(
                    format!("Audio stream #{} has uncommon channel count: {}", i, channels),
                    "Audio Channels"
                );
            }
        }
    }
}

/// Validate audio duration
fn validate_audio_duration(media_info: &MediaInfo, report: &mut ValidationReport, config: &Config) {
    let audio_streams = media_info.audio_streams();
    
    if audio_streams.is_empty() {
        return; // Already warned in validate_audio_codec
    }
    
    // Get short audio threshold from config
    let short_audio_threshold = config.validation.audio.short_audio_threshold;
    
    for (i, stream) in audio_streams.iter().enumerate() {
        // Try multiple methods to get duration, similar to the Python implementation
        
        // Method 1: Get duration directly from stream properties
        let stream_duration = stream.properties.get("duration")
            .and_then(|d| d.as_str())
            .and_then(|d| d.parse::<f64>().ok());
        
        // Method 2: Fall back to format duration if stream duration is not available
        let format_duration = if stream_duration.is_none() {
            media_info.duration()
        } else {
            None
        };
        
        if let Some(duration) = stream_duration.or(format_duration) {
            // Log the source of the duration for debugging
            if stream_duration.is_some() {
                report.add_info(
                    format!("Audio stream #{} duration: {:.3} seconds", i, duration),
                    "Audio Duration"
                );
            } else {
                report.add_info(
                    format!("Audio stream #{} duration: {:.3} seconds (using format duration)", i, duration),
                    "Audio Duration"
                );
            }
            
            // Check for very short audio using configurable threshold
            if duration < short_audio_threshold {
                report.add_warning(
                    format!("Audio stream #{} has very short duration: {:.3} seconds (threshold: {:.3}s)", 
                            i, duration, short_audio_threshold),
                    "Audio Duration"
                );
            }
        } else {
            report.add_warning(
                format!("Could not determine duration for audio stream #{}", i),
                "Audio Duration"
            );
        }
    }
}

/// Validate audio sample rate
fn validate_audio_sample_rate(media_info: &MediaInfo, report: &mut ValidationReport, config: &Config) {
    let audio_streams = media_info.audio_streams();
    
    if audio_streams.is_empty() {
        return; // Already warned in validate_audio_codec
    }
    
    for (i, stream) in audio_streams.iter().enumerate() {
        let sample_rate = stream.properties.get("sample_rate")
            .and_then(|v| v.as_str())
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        
        if sample_rate == 0 {
            report.add_warning(
                format!("Could not determine sample rate for audio stream #{}", i),
                "Audio Sample Rate"
            );
            continue;
        }
        
        report.add_info(
            format!("Audio stream #{} sample rate: {} Hz", i, sample_rate),
            "Audio Sample Rate"
        );
        
        // Check for common sample rates
        match sample_rate {
            44100 => {
                report.add_info(
                    format!("Audio stream #{} has CD-quality sample rate (44.1 kHz)", i),
                    "Audio Sample Rate"
                );
            },
            48000 => {
                report.add_info(
                    format!("Audio stream #{} has standard 48 kHz sample rate", i),
                    "Audio Sample Rate"
                );
            },
            96000 => {
                report.add_info(
                    format!("Audio stream #{} has high-quality 96 kHz sample rate", i),
                    "Audio Sample Rate"
                );
            },
            192000 => {
                report.add_info(
                    format!("Audio stream #{} has studio-quality 192 kHz sample rate", i),
                    "Audio Sample Rate"
                );
            },
            _ => {
                if sample_rate < 44100 {
                    report.add_warning(
                        format!("Audio stream #{} has low sample rate: {} Hz", i, sample_rate),
                        "Audio Sample Rate"
                    );
                } else if !is_standard_sample_rate(sample_rate, &config.validation.audio.standard_sample_rates) {
                    report.add_warning(
                        format!("Audio stream #{} has non-standard sample rate: {} Hz", i, sample_rate),
                        "Audio Sample Rate"
                    );
                }
            }
        }
    }
}

/// Check if a sample rate is a standard value
fn is_standard_sample_rate(rate: u32, standard_rates: &[u32]) -> bool {
    standard_rates.contains(&rate)
}