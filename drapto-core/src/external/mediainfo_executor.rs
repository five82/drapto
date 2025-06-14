//! MediaInfo integration for comprehensive media analysis and HDR detection
//!
//! This module provides functions for executing mediainfo commands to analyze
//! media files and extract comprehensive metadata including color space information,
//! HDR properties, and other advanced media characteristics.

use crate::error::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// MediaInfo video track information with color and HDR metadata
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct MediaInfoVideoTrack {
    #[serde(rename = "Format")]
    pub format: Option<String>,
    #[serde(rename = "Width")]
    pub width: Option<String>,
    #[serde(rename = "Height")]
    pub height: Option<String>,
    #[serde(rename = "Duration")]
    pub duration: Option<String>,
    #[serde(rename = "BitDepth")]
    pub bit_depth: Option<String>,
    #[serde(rename = "ColorSpace")]
    pub color_space: Option<String>,
    #[serde(rename = "ChromaSubsampling")]
    pub chroma_subsampling: Option<String>,
    #[serde(rename = "colour_range")]
    pub colour_range: Option<String>,
    #[serde(rename = "colour_primaries")]
    pub colour_primaries: Option<String>,
    #[serde(rename = "transfer_characteristics")]
    pub transfer_characteristics: Option<String>,
    #[serde(rename = "matrix_coefficients")]
    pub matrix_coefficients: Option<String>,
    #[serde(rename = "colour_description_present")]
    pub colour_description_present: Option<String>,
    #[serde(rename = "colour_description_present_Source")]
    pub colour_description_present_source: Option<String>,
}

/// MediaInfo audio track information
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct MediaInfoAudioTrack {
    #[serde(rename = "Format")]
    pub format: Option<String>,
    #[serde(rename = "Channels")]
    pub channels: Option<String>,
    #[serde(rename = "SamplingRate")]
    pub sampling_rate: Option<String>,
    #[serde(rename = "BitRate")]
    pub bit_rate: Option<String>,
}

/// MediaInfo track with type information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MediaInfoTrack {
    #[serde(rename = "@type")]
    pub track_type: String,
    #[serde(flatten)]
    pub video: Option<MediaInfoVideoTrack>,
    #[serde(flatten)]
    pub audio: Option<MediaInfoAudioTrack>,
}

/// MediaInfo media container
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MediaInfoMedia {
    pub track: Vec<MediaInfoTrack>,
}

/// Root MediaInfo response structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MediaInfoResponse {
    pub media: MediaInfoMedia,
}

/// HDR detection result
#[derive(Debug, Clone, Default)]
pub struct HdrInfo {
    pub is_hdr: bool,
    pub colour_primaries: Option<String>,
    pub transfer_characteristics: Option<String>,
    pub matrix_coefficients: Option<String>,
    pub bit_depth: Option<u8>,
}

/// Gets comprehensive media information using MediaInfo
pub fn get_media_info(input_path: &Path) -> CoreResult<MediaInfoResponse> {
    log::debug!(
        "Running mediainfo for comprehensive analysis on: {}",
        input_path.display()
    );

    let output = Command::new("mediainfo")
        .arg("--Output=JSON")
        .arg(input_path)
        .output()
        .map_err(|e| {
            crate::error::command_start_error("mediainfo", e)
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::error::command_failed_error(
            "mediainfo",
            output.status,
            stderr.to_string()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).map_err(|e| {
        CoreError::JsonParseError(format!(
            "Failed to parse mediainfo JSON output for {}: {}",
            input_path.display(),
            e
        ))
    })
}

/// Detects HDR content from MediaInfo data
pub fn detect_hdr_from_mediainfo(media_info: &MediaInfoResponse) -> HdrInfo {
    // Find the video track
    let video_track = media_info
        .media
        .track
        .iter()
        .find(|track| track.track_type == "Video");

    let Some(video_track) = video_track else {
        log::warn!("No video track found in MediaInfo data");
        return HdrInfo {
            is_hdr: false,
            colour_primaries: None,
            transfer_characteristics: None,
            matrix_coefficients: None,
            bit_depth: None,
        };
    };

    // Extract color information from the video track
    // Note: MediaInfo uses flattened structure, so we need to check all fields
    let colour_primaries = get_field_value(video_track, "colour_primaries");
    let transfer_characteristics = get_field_value(video_track, "transfer_characteristics");
    let matrix_coefficients = get_field_value(video_track, "matrix_coefficients");
    let bit_depth_str = get_field_value(video_track, "BitDepth");

    let bit_depth = bit_depth_str
        .and_then(|s| s.parse::<u8>().ok());

    // HDR detection logic based on color metadata
    let is_hdr = detect_hdr_from_color_metadata(
        colour_primaries.as_deref(),
        transfer_characteristics.as_deref(),
        matrix_coefficients.as_deref(),
    );

    log::debug!(
        "HDR detection result: is_hdr={}, primaries={:?}, transfer={:?}, matrix={:?}, bit_depth={:?}",
        is_hdr,
        colour_primaries,
        transfer_characteristics,
        matrix_coefficients,
        bit_depth
    );

    HdrInfo {
        is_hdr,
        colour_primaries,
        transfer_characteristics,
        matrix_coefficients,
        bit_depth,
    }
}

/// Helper function to extract field values from MediaInfo track
fn get_field_value(track: &MediaInfoTrack, field_name: &str) -> Option<String> {
    // This is a simplified approach - in reality, MediaInfo JSON structure
    // can vary, so we might need more sophisticated field extraction
    match field_name {
        "colour_primaries" => track.video.as_ref()?.colour_primaries.clone(),
        "transfer_characteristics" => track.video.as_ref()?.transfer_characteristics.clone(),
        "matrix_coefficients" => track.video.as_ref()?.matrix_coefficients.clone(),
        "BitDepth" => track.video.as_ref()?.bit_depth.clone(),
        "ColorSpace" => track.video.as_ref()?.color_space.clone(),
        _ => None,
    }
}

/// Detects HDR content based on color metadata
fn detect_hdr_from_color_metadata(
    colour_primaries: Option<&str>,
    transfer_characteristics: Option<&str>,
    matrix_coefficients: Option<&str>,
) -> bool {
    // Check for HDR primaries (BT.2020 color gamut)
    if let Some(primaries) = colour_primaries {
        if primaries.contains("BT.2020") || primaries.contains("BT.2100") {
            log::debug!("HDR detected via colour_primaries: {}", primaries);
            return true;
        }
    }

    // Check for HDR transfer characteristics
    if let Some(transfer) = transfer_characteristics {
        if transfer.contains("PQ") || transfer.contains("HLG") || transfer.contains("SMPTE 2084") {
            log::debug!("HDR detected via transfer_characteristics: {}", transfer);
            return true;
        }
    }

    // Check for HDR matrix coefficients
    if let Some(matrix) = matrix_coefficients {
        if matrix.contains("BT.2020") {
            log::debug!("HDR detected via matrix_coefficients: {}", matrix);
            return true;
        }
    }

    false
}

/// Gets audio channel information from MediaInfo
pub fn get_audio_channels_from_mediainfo(media_info: &MediaInfoResponse) -> Vec<u32> {
    media_info
        .media
        .track
        .iter()
        .filter(|track| track.track_type == "Audio")
        .filter_map(|track| {
            track.audio.as_ref()?.channels.as_ref()?.parse::<u32>().ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hdr_detection_bt2020_primaries() {
        assert!(detect_hdr_from_color_metadata(
            Some("BT.2020"),
            None,
            None
        ));
    }

    #[test]
    fn test_hdr_detection_pq_transfer() {
        assert!(detect_hdr_from_color_metadata(
            None,
            Some("PQ"),
            None
        ));
    }

    #[test]
    fn test_sdr_detection_bt709() {
        assert!(!detect_hdr_from_color_metadata(
            Some("BT.709"),
            Some("BT.709"),
            Some("BT.709")
        ));
    }

    #[test]
    fn test_sdr_detection_no_metadata() {
        assert!(!detect_hdr_from_color_metadata(None, None, None));
    }
}