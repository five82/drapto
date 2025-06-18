//! Noise analysis using FFmpeg's bitplanenoise filter.
//!
//! This module analyzes video noise levels to help determine appropriate
//! denoising parameters that maximize file size reduction while maintaining
//! visual quality.

use crate::error::CoreResult;
use crate::processing::video_properties::VideoProperties;
use std::path::Path;

/// Noise analysis results from bitplanenoise filter
#[derive(Debug, Clone)]
pub struct NoiseAnalysis {
    /// Average noise level across all bit planes (0.0 - 1.0)
    pub average_noise: f64,
    /// Maximum noise level found in any bit plane (0.0 - 1.0)
    pub max_noise: f64,
    /// Noise levels for each bit plane
    pub bit_plane_noise: Vec<f64>,
    /// Recommended hqdn3d parameters based on noise analysis
    pub recommended_hqdn3d: String,
    /// Recommended film grain synthesis level (4-16)
    pub recommended_film_grain: u8,
    /// Whether significant noise was detected
    pub has_significant_noise: bool,
}

impl NoiseAnalysis {
    /// Determine if noise level warrants denoising
    fn has_significant_noise(average_noise: f64) -> bool {
        // Noise threshold based on bitplanenoise output (0.0-1.0 range)
        // Values above 0.6 typically indicate visible noise requiring denoising
        average_noise > 0.6
    }

    /// Calculate recommended hqdn3d parameters based on noise levels
    fn calculate_hqdn3d_params(average_noise: f64, is_hdr: bool) -> String {
        // Base parameters for different noise levels
        // Format: spatial_luma:spatial_chroma:temporal_luma:temporal_chroma
        
        if !Self::has_significant_noise(average_noise) {
            // Very low noise - use minimal denoising
            if is_hdr {
                "0.5:0.4:1.5:1.5".to_string()
            } else {
                "1:0.8:2:2".to_string()
            }
        } else if average_noise < 0.7 {
            // Low noise - light denoising
            if is_hdr {
                "1:0.8:2.5:2".to_string()
            } else {
                "2:1.5:3:2.5".to_string()
            }
        } else if average_noise < 0.8 {
            // Moderate noise - medium denoising
            if is_hdr {
                "2:1.5:3.5:3".to_string()
            } else {
                "3:2.5:4:3.5".to_string()
            }
        } else {
            // High noise - stronger denoising (but still conservative)
            if is_hdr {
                "3:2.5:4.5:4".to_string()
            } else {
                "4:3.5:5:4.5".to_string()
            }
        }
    }

    /// Calculate recommended film grain synthesis level based on denoising strength
    /// Returns a value between MIN_FILM_GRAIN_VALUE (4) and MAX_FILM_GRAIN_VALUE (16)
    fn calculate_film_grain_level(average_noise: f64, is_hdr: bool) -> u8 {
        use crate::config::{MIN_FILM_GRAIN_VALUE, MAX_FILM_GRAIN_VALUE};
        
        if !Self::has_significant_noise(average_noise) {
            // Very low noise - minimal film grain
            MIN_FILM_GRAIN_VALUE
        } else if average_noise < 0.7 {
            // Low noise - slightly more film grain
            if is_hdr { 5 } else { 6 }
        } else if average_noise < 0.8 {
            // Moderate noise - moderate film grain
            if is_hdr { 8 } else { 10 }
        } else {
            // High noise - film grain to compensate for denoising
            // HDR gets less aggressive denoising, so needs less film grain compensation
            if is_hdr { 12 } else { MAX_FILM_GRAIN_VALUE }
        }
    }
}

/// Analyzes noise levels in a video using FFmpeg's bitplanenoise filter
pub fn analyze_noise(
    input_file: &Path,
    video_props: &VideoProperties,
) -> CoreResult<NoiseAnalysis> {
    log::debug!("Analyzing noise levels for {}", input_file.display());
    
    // Sample at multiple points in the video for more accurate analysis
    let sample_points = vec![0.2, 0.4, 0.5, 0.6, 0.8];
    let mut all_noise_values: Vec<Vec<f64>> = Vec::new();
    
    for position in sample_points {
        let start_time = video_props.duration_secs * position;
        let noise_values = sample_noise_at_position(input_file, start_time)?;
        
        if !noise_values.is_empty() {
            all_noise_values.push(noise_values);
        }
    }
    
    if all_noise_values.is_empty() {
        return Err(crate::error::CoreError::Analysis(
            "Failed to analyze noise levels".to_string()
        ));
    }
    
    // Calculate average noise across all samples and bit planes
    let mut avg_per_plane = vec![0.0; all_noise_values[0].len()];
    
    for sample in &all_noise_values {
        for (i, &value) in sample.iter().enumerate() {
            avg_per_plane[i] += value;
        }
    }
    
    for value in &mut avg_per_plane {
        *value /= all_noise_values.len() as f64;
    }
    
    let average_noise = avg_per_plane.iter().sum::<f64>() / avg_per_plane.len() as f64;
    let max_noise = avg_per_plane.iter().cloned().fold(0.0, f64::max);
    
    let is_hdr = video_props.hdr_info.is_hdr;
    let has_significant_noise = NoiseAnalysis::has_significant_noise(average_noise);
    let recommended_hqdn3d = NoiseAnalysis::calculate_hqdn3d_params(average_noise, is_hdr);
    let recommended_film_grain = NoiseAnalysis::calculate_film_grain_level(average_noise, is_hdr);
    
    log::info!(
        "Noise analysis: avg={:.4}, max={:.4}, significant={}, recommended=hqdn3d={}, film_grain={}",
        average_noise, max_noise, has_significant_noise, recommended_hqdn3d, recommended_film_grain
    );
    
    Ok(NoiseAnalysis {
        average_noise,
        max_noise,
        bit_plane_noise: avg_per_plane,
        recommended_hqdn3d,
        recommended_film_grain,
        has_significant_noise,
    })
}

/// Sample noise levels at a specific position in the video
fn sample_noise_at_position(
    input_file: &Path,
    start_time: f64,
) -> CoreResult<Vec<f64>> {
    let mut cmd = crate::external::FfmpegCommandBuilder::new()
        .with_hardware_accel(true)
        .build();
    
    // Start at the specified time
    cmd.args(["-ss", &format!("{:.2}", start_time)]);
    
    // Input file
    cmd.input(input_file.to_string_lossy());
    
    // Analyze 30 frames with bitplanenoise filter and metadata output
    cmd.args([
        "-vframes", "30",
        "-vf", "bitplanenoise,metadata=mode=print",
        "-f", "null",
        "-"
    ]);
    
    // Spawn and collect output
    let mut child = cmd
        .spawn()
        .map_err(|e| crate::error::command_start_error("ffmpeg", e))?;
    
    let mut noise_values = Vec::new();
    let mut found_values = false;
    
    for event in child.iter().map_err(|e| {
        crate::error::command_failed_error(
            "ffmpeg",
            std::process::ExitStatus::default(),
            e.to_string(),
        )
    })? {
        if let ffmpeg_sidecar::event::FfmpegEvent::Log(_, line) = event {
            // Parse bitplanenoise metadata output
            // Format: [Parsed_metadata_1 @ 0x...] lavfi.bitplanenoise.0.1=0.827098
            if line.contains("lavfi.bitplanenoise") && line.contains("=") {
                // Extract just the luma (0) channel noise value for primary measurement
                if line.contains("lavfi.bitplanenoise.0.1=") {
                    if let Some(noise_part) = line.split("lavfi.bitplanenoise.0.1=").nth(1) {
                        if let Ok(noise_value) = noise_part.trim().parse::<f64>() {
                            noise_values.push(noise_value);
                            found_values = true;
                        }
                    }
                }
            }
        }
    }
    
    if !found_values {
        log::warn!("No noise values found at position {:.1}s", start_time);
    }
    
    Ok(noise_values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_significant_noise() {
        assert!(!NoiseAnalysis::has_significant_noise(0.4));
        assert!(!NoiseAnalysis::has_significant_noise(0.6));
        assert!(NoiseAnalysis::has_significant_noise(0.61));
        assert!(NoiseAnalysis::has_significant_noise(0.7));
        assert!(NoiseAnalysis::has_significant_noise(0.8));
    }

    #[test]
    fn test_calculate_hqdn3d_params_sdr() {
        // Very low noise
        assert_eq!(
            NoiseAnalysis::calculate_hqdn3d_params(0.5, false),
            "1:0.8:2:2"
        );
        
        // Low noise
        assert_eq!(
            NoiseAnalysis::calculate_hqdn3d_params(0.65, false),
            "2:1.5:3:2.5"
        );
        
        // Moderate noise
        assert_eq!(
            NoiseAnalysis::calculate_hqdn3d_params(0.75, false),
            "3:2.5:4:3.5"
        );
        
        // High noise
        assert_eq!(
            NoiseAnalysis::calculate_hqdn3d_params(0.85, false),
            "4:3.5:5:4.5"
        );
    }

    #[test]
    fn test_calculate_hqdn3d_params_hdr() {
        // Very low noise
        assert_eq!(
            NoiseAnalysis::calculate_hqdn3d_params(0.5, true),
            "0.5:0.4:1.5:1.5"
        );
        
        // Low noise
        assert_eq!(
            NoiseAnalysis::calculate_hqdn3d_params(0.65, true),
            "1:0.8:2.5:2"
        );
        
        // Moderate noise
        assert_eq!(
            NoiseAnalysis::calculate_hqdn3d_params(0.75, true),
            "2:1.5:3.5:3"
        );
        
        // High noise
        assert_eq!(
            NoiseAnalysis::calculate_hqdn3d_params(0.85, true),
            "3:2.5:4.5:4"
        );
    }

    #[test]
    fn test_calculate_film_grain_level_sdr() {
        use crate::config::{MIN_FILM_GRAIN_VALUE, MAX_FILM_GRAIN_VALUE};
        
        // Very low noise - minimum film grain (SDR)
        assert_eq!(
            NoiseAnalysis::calculate_film_grain_level(0.5, false),
            MIN_FILM_GRAIN_VALUE
        );
        
        // Low noise (SDR)
        assert_eq!(
            NoiseAnalysis::calculate_film_grain_level(0.65, false),
            6
        );
        
        // Moderate noise (SDR)
        assert_eq!(
            NoiseAnalysis::calculate_film_grain_level(0.75, false),
            10
        );
        
        // High noise - maximum film grain (SDR)
        assert_eq!(
            NoiseAnalysis::calculate_film_grain_level(0.85, false),
            MAX_FILM_GRAIN_VALUE
        );
    }
    
    #[test]
    fn test_calculate_film_grain_level_hdr() {
        use crate::config::MIN_FILM_GRAIN_VALUE;
        
        // Very low noise - minimum film grain (HDR)
        assert_eq!(
            NoiseAnalysis::calculate_film_grain_level(0.5, true),
            MIN_FILM_GRAIN_VALUE
        );
        
        // Low noise (HDR) - less than SDR
        assert_eq!(
            NoiseAnalysis::calculate_film_grain_level(0.65, true),
            5
        );
        
        // Moderate noise (HDR) - less than SDR
        assert_eq!(
            NoiseAnalysis::calculate_film_grain_level(0.75, true),
            8
        );
        
        // High noise (HDR) - less than SDR max
        assert_eq!(
            NoiseAnalysis::calculate_film_grain_level(0.85, true),
            12
        );
    }
}