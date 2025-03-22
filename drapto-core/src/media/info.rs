use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;
use crate::media::probe::FFprobe;

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

impl fmt::Display for StreamType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamType::Video => write!(f, "Video"),
            StreamType::Audio => write!(f, "Audio"),
            StreamType::Subtitle => write!(f, "Subtitle"),
            StreamType::Attachment => write!(f, "Attachment"),
            StreamType::Data => write!(f, "Data"),
            StreamType::Unknown => write!(f, "Unknown"),
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
    pub format: Option<FormatInfo>,
    
    /// Media chapters
    pub chapters: Vec<ChapterInfo>,
}

impl MediaInfo {
    /// Create a new MediaInfo from a file path
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let json = FFprobe::execute(path)?;
        
        let mut media_info = Self {
            streams: Vec::new(),
            format: None,
            chapters: Vec::new(),
        };
        
        // Parse streams
        if let Some(streams) = json.get("streams").and_then(|s| s.as_array()) {
            for stream in streams {
                let index = stream.get("index")
                    .and_then(|i| i.as_u64())
                    .unwrap_or(0) as usize;
                
                let codec_type = stream.get("codec_type")
                    .and_then(|t| t.as_str())
                    .map(StreamType::from)
                    .unwrap_or(StreamType::Unknown);
                
                let codec_name = stream.get("codec_name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let codec_long_name = stream.get("codec_long_name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string());
                
                // Parse tags
                let mut tags = HashMap::new();
                if let Some(stream_tags) = stream.get("tags").and_then(|t| t.as_object()) {
                    for (key, value) in stream_tags {
                        if let Some(value_str) = value.as_str() {
                            tags.insert(key.clone(), value_str.to_string());
                        }
                    }
                }
                
                // Parse other properties
                let mut properties = HashMap::new();
                if let Some(obj) = stream.as_object() {
                    for (key, value) in obj {
                        if key != "tags" && key != "index" && key != "codec_type" && 
                           key != "codec_name" && key != "codec_long_name" {
                            properties.insert(key.clone(), value.clone());
                        }
                    }
                }
                
                media_info.streams.push(StreamInfo {
                    index,
                    codec_type,
                    codec_name,
                    codec_long_name,
                    tags,
                    properties,
                });
            }
        }
        
        // Parse format
        if let Some(format) = json.get("format").and_then(|f| f.as_object()) {
            let format_name = format.get("format_name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();
            
            let format_long_name = format.get("format_long_name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string());
            
            let duration = format.get("duration")
                .and_then(|d| d.as_str())
                .and_then(|d| d.parse::<f64>().ok());
            
            let bit_rate = format.get("bit_rate")
                .and_then(|b| b.as_str())
                .and_then(|b| b.parse::<u64>().ok());
            
            let size = format.get("size")
                .and_then(|s| s.as_str())
                .and_then(|s| s.parse::<u64>().ok());
            
            // Parse tags
            let mut tags = HashMap::new();
            if let Some(format_tags) = format.get("tags").and_then(|t| t.as_object()) {
                for (key, value) in format_tags {
                    if let Some(value_str) = value.as_str() {
                        tags.insert(key.clone(), value_str.to_string());
                    }
                }
            }
            
            media_info.format = Some(FormatInfo {
                format_name,
                format_long_name,
                duration,
                bit_rate,
                size,
                tags,
            });
        }
        
        // Parse chapters
        if let Some(chapters) = json.get("chapters").and_then(|c| c.as_array()) {
            for chapter in chapters {
                let id = chapter.get("id")
                    .and_then(|i| i.as_u64())
                    .unwrap_or(0);
                
                let start_time = chapter.get("start_time")
                    .and_then(|t| t.as_str())
                    .and_then(|t| t.parse::<f64>().ok())
                    .unwrap_or(0.0);
                
                let end_time = chapter.get("end_time")
                    .and_then(|t| t.as_str())
                    .and_then(|t| t.parse::<f64>().ok())
                    .unwrap_or(0.0);
                
                // Parse tags
                let mut tags = HashMap::new();
                if let Some(chapter_tags) = chapter.get("tags").and_then(|t| t.as_object()) {
                    for (key, value) in chapter_tags {
                        if let Some(value_str) = value.as_str() {
                            tags.insert(key.clone(), value_str.to_string());
                        }
                    }
                }
                
                media_info.chapters.push(ChapterInfo {
                    id,
                    start_time,
                    end_time,
                    tags,
                });
            }
        }
        
        Ok(media_info)
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
        self.format.as_ref().and_then(|f| f.duration)
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
        // Check format tags first
        if let Some(format) = &self.format {
            if format.tags.iter().any(|(k, v)| {
                let k_lower = k.to_lowercase();
                let v_lower = v.to_lowercase();
                k_lower.contains("dolby_vision") || 
                k_lower.contains("dovi") ||
                v_lower.contains("dolby vision") ||
                v_lower.contains("dv")
            }) {
                return true;
            }
        }
        
        // Then check video stream
        if let Some(stream) = self.primary_video_stream() {
            // Check codec names
            let codec_name = stream.codec_name.to_lowercase();
            if codec_name.contains("dvh") || 
               codec_name.contains("dolby") || 
               codec_name.contains("dovi") {
                return true;
            }
            
            // Check stream tags
            if stream.tags.iter().any(|(k, v)| {
                let k_lower = k.to_lowercase();
                let v_lower = v.to_lowercase();
                k_lower.contains("dolby_vision") || 
                k_lower.contains("dovi") ||
                k_lower.contains("dv_profile") ||
                k_lower.contains("dv_level") ||
                v_lower.contains("dolby vision") ||
                v_lower.contains("dv")
            }) {
                return true;
            }
            
            // Check codec tag
            if let Some(codec_tag) = stream.properties.get("codec_tag_string")
                .and_then(|v| v.as_str()) {
                let codec_tag_lower = codec_tag.to_lowercase();
                if codec_tag_lower == "dovi" || 
                   codec_tag_lower.contains("dvh") {
                    return true;
                }
            }
            
            // Check for specialized metadata
            if let Some(side_data_list) = stream.properties.get("side_data_list")
                .and_then(|v| v.as_array()) {
                for side_data in side_data_list {
                    if let Some(side_data_type) = side_data.get("side_data_type")
                        .and_then(|v| v.as_str()) {
                        if side_data_type.contains("DOVI") || 
                           side_data_type.contains("Dolby Vision") {
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }

    /// Get the bitrate in bits per second
    pub fn bitrate(&self) -> Option<u64> {
        // First try to get from format info
        if let Some(bitrate) = self.format.as_ref().and_then(|f| f.bit_rate) {
            return Some(bitrate);
        }
        
        // If not available at format level, sum stream bitrates
        let mut total_bitrate = 0;
        let mut found_bitrate = false;
        
        for stream in &self.streams {
            if let Some(bit_rate) = stream.properties.get("bit_rate")
                .and_then(|v| v.as_str())
                .and_then(|v| v.parse::<u64>().ok()) {
                total_bitrate += bit_rate;
                found_bitrate = true;
            }
        }
        
        if found_bitrate {
            Some(total_bitrate)
        } else {
            // As a fallback, estimate from file size and duration
            if let (Some(size), Some(duration)) = (
                self.format.as_ref().and_then(|f| f.size),
                self.duration()
            ) {
                if duration > 0.0 {
                    // Convert bytes to bits and divide by duration
                    let bits = size * 8;
                    let bitrate = (bits as f64 / duration) as u64;
                    return Some(bitrate);
                }
            }
            
            None
        }
    }
}