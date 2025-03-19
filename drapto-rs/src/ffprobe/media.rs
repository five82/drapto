use std::path::Path;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;
use super::exec::FFprobe;

/// Media stream types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamType {
    Video,
    Audio,
    Subtitle,
    Attachment,
    Data,
    Unknown,
}

impl From<&str> for StreamType {
    fn from(s: &str) -> Self {
        match s {
            "video" => StreamType::Video,
            "audio" => StreamType::Audio,
            "subtitle" => StreamType::Subtitle,
            "attachment" => StreamType::Attachment,
            "data" => StreamType::Data,
            _ => StreamType::Unknown,
        }
    }
}

/// Stream information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    /// Stream index
    pub index: usize,
    
    /// Stream type
    pub codec_type: StreamType,
    
    /// Codec name
    pub codec_name: String,
    
    /// Codec long name
    pub codec_long_name: Option<String>,
    
    /// Stream tags
    pub tags: HashMap<String, String>,
    
    /// Stream-specific properties
    pub properties: HashMap<String, Value>,
}

/// Media format information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatInfo {
    /// Format name
    pub format_name: String,
    
    /// Format long name
    pub format_long_name: Option<String>,
    
    /// Duration in seconds
    pub duration: Option<f64>,
    
    /// Bitrate in bits per second
    pub bit_rate: Option<u64>,
    
    /// Size in bytes
    pub size: Option<u64>,
    
    /// Format tags
    pub tags: HashMap<String, String>,
}

/// Chapter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterInfo {
    /// Chapter ID
    pub id: u64,
    
    /// Start time in seconds
    pub start_time: f64,
    
    /// End time in seconds
    pub end_time: f64,
    
    /// Chapter tags
    pub tags: HashMap<String, String>,
}

/// Complete media information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInfo {
    /// Media streams
    pub streams: Vec<StreamInfo>,
    
    /// Media format
    pub format: FormatInfo,
    
    /// Media chapters
    pub chapters: Vec<ChapterInfo>,
}

impl MediaInfo {
    /// Create MediaInfo from a file path
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let json = FFprobe::execute(path)?;
        Self::from_json(json)
    }
    
    /// Create MediaInfo from ffprobe JSON output
    pub fn from_json(json: Value) -> Result<Self> {
        let streams = json["streams"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|stream| {
                let index = stream["index"].as_u64().unwrap_or(0) as usize;
                let codec_type = stream["codec_type"]
                    .as_str()
                    .map(StreamType::from)
                    .unwrap_or(StreamType::Unknown);
                let codec_name = stream["codec_name"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();
                let codec_long_name = stream["codec_long_name"]
                    .as_str()
                    .map(ToString::to_string);
                
                // Extract tags
                let tags = if let Some(tags) = stream["tags"].as_object() {
                    tags.iter()
                        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                        .collect()
                } else {
                    HashMap::new()
                };
                
                // Extract remaining properties
                let mut properties = HashMap::new();
                if let Some(obj) = stream.as_object() {
                    for (k, v) in obj {
                        if k != "index" && k != "codec_type" && k != "codec_name" 
                            && k != "codec_long_name" && k != "tags" {
                            properties.insert(k.clone(), v.clone());
                        }
                    }
                }
                
                StreamInfo {
                    index,
                    codec_type,
                    codec_name,
                    codec_long_name,
                    tags,
                    properties,
                }
            })
            .collect();
        
        let format = {
            let format = &json["format"];
            let format_name = format["format_name"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            let format_long_name = format["format_long_name"]
                .as_str()
                .map(ToString::to_string);
            let duration = format["duration"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok());
            let bit_rate = format["bit_rate"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok());
            let size = format["size"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok());
            
            // Extract tags
            let tags = if let Some(tags) = format["tags"].as_object() {
                tags.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                    .collect()
            } else {
                HashMap::new()
            };
            
            FormatInfo {
                format_name,
                format_long_name,
                duration,
                bit_rate,
                size,
                tags,
            }
        };
        
        let chapters = json["chapters"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|chapter| {
                let id = chapter["id"].as_u64().unwrap_or(0);
                let start_time = chapter["start_time"]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let end_time = chapter["end_time"]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                
                // Extract tags
                let tags = if let Some(tags) = chapter["tags"].as_object() {
                    tags.iter()
                        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                        .collect()
                } else {
                    HashMap::new()
                };
                
                ChapterInfo {
                    id,
                    start_time,
                    end_time,
                    tags,
                }
            })
            .collect();
        
        Ok(MediaInfo {
            streams,
            format,
            chapters,
        })
    }
    
    /// Get video streams
    pub fn video_streams(&self) -> Vec<&StreamInfo> {
        self.streams
            .iter()
            .filter(|s| s.codec_type == StreamType::Video)
            .collect()
    }
    
    /// Get audio streams
    pub fn audio_streams(&self) -> Vec<&StreamInfo> {
        self.streams
            .iter()
            .filter(|s| s.codec_type == StreamType::Audio)
            .collect()
    }
    
    /// Get subtitle streams
    pub fn subtitle_streams(&self) -> Vec<&StreamInfo> {
        self.streams
            .iter()
            .filter(|s| s.codec_type == StreamType::Subtitle)
            .collect()
    }
    
    /// Get total duration in seconds
    pub fn duration(&self) -> Option<f64> {
        self.format.duration
    }
    
    /// Get primary video stream
    pub fn primary_video_stream(&self) -> Option<&StreamInfo> {
        self.video_streams().first().copied()
    }
    
    /// Get video width and height if available
    pub fn video_dimensions(&self) -> Option<(u32, u32)> {
        self.primary_video_stream().and_then(|stream| {
            let width = stream.properties.get("width")
                .and_then(|w| w.as_u64())
                .map(|w| w as u32);
            let height = stream.properties.get("height")
                .and_then(|h| h.as_u64())
                .map(|h| h as u32);
            
            match (width, height) {
                (Some(w), Some(h)) => Some((w, h)),
                _ => None,
            }
        })
    }
    
    /// Check if the media contains HDR content
    pub fn is_hdr(&self) -> bool {
        self.primary_video_stream()
            .and_then(|stream| {
                // Check color primaries, transfer characteristics, and bit depth
                let color_primaries = stream.properties.get("color_primaries")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                    
                let color_transfer = stream.properties.get("color_transfer")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                    
                let bits_per_raw_sample = stream.properties.get("bits_per_raw_sample")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(8);
                
                // Common HDR indicators
                let hdr_primaries = color_primaries == "bt2020";
                let hdr_transfer = color_transfer == "smpte2084" || color_transfer == "arib-std-b67";
                let high_bit_depth = bits_per_raw_sample >= 10;
                
                if hdr_primaries && hdr_transfer && high_bit_depth {
                    Some(true)
                } else {
                    None
                }
            })
            .unwrap_or(false)
    }
    
    /// Check if the media contains Dolby Vision content
    pub fn is_dolby_vision(&self) -> bool {
        self.primary_video_stream()
            .map(|stream| {
                // Check codec and/or Dolby Vision metadata
                let is_dv_codec = stream.codec_name.contains("dvh1") 
                    || stream.codec_name.contains("dolby");
                
                // Check for specific Dolby Vision tags
                let has_dv_tag = stream.tags.iter().any(|(k, v)| {
                    k.to_lowercase().contains("dolby_vision") || 
                    v.to_lowercase().contains("dolby vision")
                });
                
                is_dv_codec || has_dv_tag
            })
            .unwrap_or(false)
    }
}