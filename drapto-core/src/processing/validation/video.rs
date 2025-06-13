//! Video codec and bit depth validation

use ffprobe::Stream;

/// Validates the video codec (AV1) and bit depth (10-bit)
pub fn validate_video_codec_and_depth(stream: &Stream) -> (bool, bool, Option<String>, Option<String>, Option<u32>) {
    let codec_name = stream.codec_name.clone();
    let pixel_format = stream.pix_fmt.clone();
    
    // Check if codec is AV1
    let is_av1 = codec_name
        .as_deref()
        .map(|c| c.eq_ignore_ascii_case("av01") || c.eq_ignore_ascii_case("av1"))
        .unwrap_or(false);

    // Check bit depth - try multiple methods
    let bit_depth = get_bit_depth_from_stream(stream);
    let is_10_bit = bit_depth.map_or(false, |depth| depth == 10);

    (is_av1, is_10_bit, codec_name, pixel_format, bit_depth)
}

/// Extract bit depth from video stream using multiple methods
fn get_bit_depth_from_stream(stream: &Stream) -> Option<u32> {
    // Method 1: Check bits_per_raw_sample field
    if let Some(bits_str) = &stream.bits_per_raw_sample {
        if let Ok(bits) = bits_str.parse::<u32>() {
            if bits > 0 {
                return Some(bits);
            }
        }
    }

    // Method 2: Infer from pixel format
    if let Some(pix_fmt) = &stream.pix_fmt {
        return infer_bit_depth_from_pixel_format(pix_fmt);
    }

    // Method 3: Check profile for additional hints
    if let Some(profile) = &stream.profile {
        if profile.contains("10") {
            return Some(10);
        }
    }

    None
}

/// Infer bit depth from pixel format string
fn infer_bit_depth_from_pixel_format(pix_fmt: &str) -> Option<u32> {
    match pix_fmt {
        // 10-bit formats
        s if s.contains("10le") || s.contains("10be") => Some(10),
        s if s.contains("p010") || s.contains("p016") => Some(10),
        s if s.contains("yuv420p10") || s.contains("yuv422p10") || s.contains("yuv444p10") => Some(10),
        
        // 12-bit formats
        s if s.contains("12le") || s.contains("12be") => Some(12),
        s if s.contains("yuv420p12") || s.contains("yuv422p12") || s.contains("yuv444p12") => Some(12),
        
        // 8-bit formats (default)
        s if s.contains("yuv420p") || s.contains("yuv422p") || s.contains("yuv444p") => Some(8),
        s if s.contains("nv12") || s.contains("nv21") => Some(8),
        
        // If we can't determine, return None
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_format_bit_depth_inference() {
        assert_eq!(infer_bit_depth_from_pixel_format("yuv420p10le"), Some(10));
        assert_eq!(infer_bit_depth_from_pixel_format("yuv420p"), Some(8));
        assert_eq!(infer_bit_depth_from_pixel_format("yuv422p12le"), Some(12));
        assert_eq!(infer_bit_depth_from_pixel_format("p010le"), Some(10));
        assert_eq!(infer_bit_depth_from_pixel_format("unknown_format"), None);
    }
}