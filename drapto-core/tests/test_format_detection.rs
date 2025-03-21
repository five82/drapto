use std::collections::HashMap;
use serde_json::Value;
use drapto_core::detection::format::{has_hdr, has_dolby_vision};
use drapto_core::media::{MediaInfo, StreamInfo, StreamType, FormatInfo};

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_stream(codec_type: StreamType, properties: HashMap<String, Value>, tags: HashMap<String, String>) -> StreamInfo {
        StreamInfo {
            index: 0,
            codec_type,
            codec_name: String::from("test_codec"),
            codec_long_name: Some(String::from("Test Codec")),
            properties,
            tags,
        }
    }
    
    fn create_test_media_info(streams: Vec<StreamInfo>, format_tags: Option<HashMap<String, String>>) -> MediaInfo {
        let format = if let Some(tags) = format_tags {
            Some(FormatInfo {
                format_name: String::from("test_format"),
                format_long_name: Some(String::from("Test Format")),
                duration: Some(60.0),
                bit_rate: Some(1000000),
                size: Some(7500000),
                tags,
            })
        } else {
            None
        };
        
        MediaInfo {
            streams,
            format,
            chapters: Vec::new(),
        }
    }
    
    #[test]
    fn test_has_hdr() {
        // Test with HDR transfer function (smpte2084/PQ)
        let mut props = HashMap::new();
        props.insert(String::from("color_transfer"), Value::String(String::from("smpte2084")));
        
        let stream = create_test_stream(StreamType::Video, props, HashMap::new());
        let media_info = create_test_media_info(vec![stream], None);
        
        assert!(has_hdr(&media_info));
        
        // Test with HDR color primaries (bt2020)
        let mut props = HashMap::new();
        props.insert(String::from("color_primaries"), Value::String(String::from("bt2020")));
        
        let stream = create_test_stream(StreamType::Video, props, HashMap::new());
        let media_info = create_test_media_info(vec![stream], None);
        
        assert!(has_hdr(&media_info));
        
        // Test with HDR color space (bt2020nc)
        let mut props = HashMap::new();
        props.insert(String::from("color_space"), Value::String(String::from("bt2020nc")));
        
        let stream = create_test_stream(StreamType::Video, props, HashMap::new());
        let media_info = create_test_media_info(vec![stream], None);
        
        assert!(has_hdr(&media_info));
        
        // Test with non-HDR
        let mut props = HashMap::new();
        props.insert(String::from("color_transfer"), Value::String(String::from("bt709")));
        props.insert(String::from("color_primaries"), Value::String(String::from("bt709")));
        props.insert(String::from("color_space"), Value::String(String::from("bt709")));
        
        let stream = create_test_stream(StreamType::Video, props, HashMap::new());
        let media_info = create_test_media_info(vec![stream], None);
        
        assert!(!has_hdr(&media_info));
    }
    
    #[test]
    fn test_has_dolby_vision() {
        // Test with Dolby Vision in format tags
        let mut format_tags = HashMap::new();
        format_tags.insert(String::from("hdr_format"), String::from("Dolby Vision"));
        
        let stream = create_test_stream(StreamType::Video, HashMap::new(), HashMap::new());
        let media_info = create_test_media_info(vec![stream], Some(format_tags));
        
        assert!(has_dolby_vision(&media_info));
        
        // Test with Dolby Vision in stream tags
        let mut stream_tags = HashMap::new();
        stream_tags.insert(String::from("DOLBY_VISION"), String::from("1"));
        
        let stream = create_test_stream(StreamType::Video, HashMap::new(), stream_tags);
        let media_info = create_test_media_info(vec![stream], None);
        
        assert!(has_dolby_vision(&media_info));
        
        // Test with Dolby Vision codec tag
        let mut props = HashMap::new();
        props.insert(String::from("codec_tag_string"), Value::String(String::from("dovi")));
        
        let stream = create_test_stream(StreamType::Video, props, HashMap::new());
        let media_info = create_test_media_info(vec![stream], None);
        
        assert!(has_dolby_vision(&media_info));
        
        // Test without Dolby Vision
        let stream = create_test_stream(StreamType::Video, HashMap::new(), HashMap::new());
        let media_info = create_test_media_info(vec![stream], None);
        
        assert!(!has_dolby_vision(&media_info));
    }
}