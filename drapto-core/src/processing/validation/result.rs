//! Validation result structure and implementation

/// Validation results for an encoded video file
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the video codec is AV1
    pub is_av1: bool,
    /// Whether the video is 10-bit
    pub is_10_bit: bool,
    /// Whether crop was applied correctly (if crop was expected)
    pub is_crop_correct: bool,
    /// Whether duration matches the input duration
    pub is_duration_correct: bool,
    /// Whether HDR matches the input (both HDR or both SDR)
    pub is_hdr_correct: bool,
    /// Whether all audio tracks are Opus codec
    pub is_audio_opus: bool,
    /// The actual codec found
    pub codec_name: Option<String>,
    /// The actual pixel format found
    pub pixel_format: Option<String>,
    /// The actual bit depth found
    pub bit_depth: Option<u32>,
    /// The actual video dimensions found
    pub actual_dimensions: Option<(u32, u32)>,
    /// The expected dimensions after crop
    pub expected_dimensions: Option<(u32, u32)>,
    /// Crop validation message
    pub crop_message: Option<String>,
    /// The actual duration found (in seconds)
    pub actual_duration: Option<f64>,
    /// The expected duration (in seconds)
    pub expected_duration: Option<f64>,
    /// Duration validation message
    pub duration_message: Option<String>,
    /// Whether the input was HDR
    pub expected_hdr: Option<bool>,
    /// Whether the output is HDR
    pub actual_hdr: Option<bool>,
    /// HDR validation message
    pub hdr_message: Option<String>,
    /// List of audio codec names found in the output
    pub audio_codecs: Vec<String>,
    /// Audio validation message
    pub audio_message: Option<String>,
}

impl ValidationResult {
    /// Returns true if all validations pass
    pub fn is_valid(&self) -> bool {
        self.is_av1 && self.is_10_bit && self.is_crop_correct && self.is_duration_correct && self.is_hdr_correct && self.is_audio_opus
    }

    /// Returns a list of validation failures
    pub fn get_failures(&self) -> Vec<String> {
        let mut failures = Vec::new();
        
        if !self.is_av1 {
            let codec = self.codec_name.as_deref().unwrap_or("unknown");
            failures.push(format!("Expected AV1 codec, found: {}", codec));
        }
        
        if !self.is_10_bit {
            let pix_fmt = self.pixel_format.as_deref().unwrap_or("unknown");
            let bit_depth = self.bit_depth.map_or("unknown".to_string(), |d| d.to_string());
            failures.push(format!("Expected 10-bit depth, found: {} bit (pixel format: {})", bit_depth, pix_fmt));
        }
        
        if !self.is_crop_correct {
            if let Some(msg) = &self.crop_message {
                failures.push(format!("Crop validation failed: {}", msg));
            } else {
                failures.push("Crop validation failed".to_string());
            }
        }
        
        if !self.is_duration_correct {
            if let Some(msg) = &self.duration_message {
                failures.push(format!("Duration validation failed: {}", msg));
            } else {
                failures.push("Duration validation failed".to_string());
            }
        }
        
        if !self.is_hdr_correct {
            if let Some(msg) = &self.hdr_message {
                failures.push(format!("HDR validation failed: {}", msg));
            } else {
                failures.push("HDR validation failed".to_string());
            }
        }
        
        if !self.is_audio_opus {
            if let Some(msg) = &self.audio_message {
                failures.push(format!("Audio codec validation failed: {}", msg));
            } else {
                failures.push("Audio codec validation failed".to_string());
            }
        }
        
        failures
    }

    /// Returns individual validation step results
    pub fn get_validation_steps(&self) -> Vec<(String, bool, String)> {
        let mut steps = Vec::new();
        
        // AV1 codec validation
        let codec_name = self.codec_name.as_deref().unwrap_or("unknown");
        let codec_result = if self.is_av1 {
            format!("AV1 codec ({})", codec_name)
        } else {
            format!("Expected AV1, found: {}", codec_name)
        };
        steps.push(("Video codec".to_string(), self.is_av1, codec_result));
        
        // 10-bit depth validation
        let bit_depth_result = if self.is_10_bit {
            format!("{}-bit depth", self.bit_depth.unwrap_or(10))
        } else {
            let depth = self.bit_depth.map_or("unknown".to_string(), |d| d.to_string());
            format!("Expected 10-bit, found: {}-bit", depth)
        };
        steps.push(("Bit depth".to_string(), self.is_10_bit, bit_depth_result));
        
        // Crop validation
        let crop_result = if self.is_crop_correct {
            self.crop_message.as_ref()
                .map(|msg| msg.clone())
                .unwrap_or_else(|| "Crop applied correctly".to_string())
        } else {
            self.crop_message.as_ref()
                .map(|msg| msg.clone())
                .unwrap_or_else(|| "Crop validation failed".to_string())
        };
        steps.push(("Crop detection".to_string(), self.is_crop_correct, crop_result));
        
        // Duration validation
        let duration_result = if self.is_duration_correct {
            self.duration_message.as_ref()
                .map(|msg| msg.clone())
                .unwrap_or_else(|| "Duration matches input".to_string())
        } else {
            self.duration_message.as_ref()
                .map(|msg| msg.clone())
                .unwrap_or_else(|| "Duration validation failed".to_string())
        };
        steps.push(("Video duration".to_string(), self.is_duration_correct, duration_result));
        
        // HDR validation
        let hdr_result = if self.is_hdr_correct {
            self.hdr_message.as_ref()
                .map(|msg| msg.clone())
                .unwrap_or_else(|| "HDR status matches input".to_string())
        } else {
            self.hdr_message.as_ref()
                .map(|msg| msg.clone())
                .unwrap_or_else(|| "HDR validation failed".to_string())
        };
        steps.push(("HDR/SDR status".to_string(), self.is_hdr_correct, hdr_result));
        
        // Audio codec validation
        let audio_result = if self.is_audio_opus {
            self.audio_message.as_ref()
                .map(|msg| msg.clone())
                .unwrap_or_else(|| "All audio tracks are Opus".to_string())
        } else {
            self.audio_message.as_ref()
                .map(|msg| msg.clone())
                .unwrap_or_else(|| "Audio codec validation failed".to_string())
        };
        steps.push(("Audio codec".to_string(), self.is_audio_opus, audio_result));
        
        steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_failures() {
        let result = ValidationResult {
            is_av1: false,
            is_10_bit: true,
            is_crop_correct: true,
            is_duration_correct: true,
            is_hdr_correct: true,
            is_audio_opus: true,
            codec_name: Some("h264".to_string()),
            pixel_format: Some("yuv420p10le".to_string()),
            bit_depth: Some(10),
            actual_dimensions: Some((1920, 1080)),
            expected_dimensions: Some((1920, 1080)),
            crop_message: Some("No crop expected, dimensions: 1920x1080".to_string()),
            actual_duration: Some(6155.0),
            expected_duration: Some(6155.0),
            duration_message: Some("Duration matches input (6155.0s)".to_string()),
            expected_hdr: Some(false),
            actual_hdr: Some(false),
            hdr_message: Some("SDR preserved".to_string()),
            audio_codecs: vec!["opus".to_string()],
            audio_message: Some("Audio track is Opus".to_string()),
        };
        
        let failures = result.get_failures();
        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("Expected AV1 codec"));
        assert!(failures[0].contains("h264"));
    }
}