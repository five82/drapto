//! Noise analysis using FFmpeg's bitplanenoise filter.
//!
//! This module analyzes video noise levels to help determine appropriate
//! denoising parameters that maximize file size reduction while maintaining
//! visual quality.

use crate::error::CoreResult;
use crate::events::{Event, EventDispatcher};
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
    /// Film grain level is proportional to the spatial denoising strength applied
    fn calculate_film_grain_level(average_noise: f64, is_hdr: bool) -> u8 {
        use crate::config::{MIN_FILM_GRAIN_VALUE, MAX_FILM_GRAIN_VALUE};
        
        if !Self::has_significant_noise(average_noise) {
            // Very low noise - minimal film grain compensation
            MIN_FILM_GRAIN_VALUE
        } else {
            // Calculate film grain proportional to spatial denoising strength
            // Extract spatial luma parameter from the denoising settings
            let hqdn3d_params = Self::calculate_hqdn3d_params(average_noise, is_hdr);
            let spatial_luma = Self::extract_spatial_luma_param(&hqdn3d_params);
            
            // Film grain level scales with spatial denoising strength
            // Stronger denoising removes more natural grain, requiring more synthesis
            let base_grain = MIN_FILM_GRAIN_VALUE as f64;
            let max_grain = MAX_FILM_GRAIN_VALUE as f64;
            
            // Scale film grain based on spatial luma denoising (typical range 0.5-4.0)
            // Conservative scaling: grain increases gradually with denoising strength
            let grain_scaling = (spatial_luma - 0.5).max(0.0).min(3.5) / 3.5; // Normalize to 0.0-1.0
            let calculated_grain = base_grain + (max_grain - base_grain) * grain_scaling;
            
            // Ensure result is within valid range
            (calculated_grain.round() as u8).clamp(MIN_FILM_GRAIN_VALUE, MAX_FILM_GRAIN_VALUE)
        }
    }
    
    /// Extract the spatial luma parameter from hqdn3d parameter string
    /// Format: "spatial_luma:spatial_chroma:temporal_luma:temporal_chroma"
    fn extract_spatial_luma_param(hqdn3d_params: &str) -> f64 {
        hqdn3d_params
            .split(':')
            .next()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(1.0) // Default fallback
    }
}

/// Analyzes noise levels in a video using FFmpeg's bitplanenoise filter
pub fn analyze_noise(
    input_file: &Path,
    video_props: &VideoProperties,
    event_dispatcher: Option<&EventDispatcher>,
) -> CoreResult<NoiseAnalysis> {
    log::debug!("Analyzing noise levels for {}", input_file.display());
    
    // Emit start event
    if let Some(dispatcher) = event_dispatcher {
        dispatcher.emit(Event::StageProgress {
            stage: "noise_analysis".to_string(),
            percent: 0.0,
            message: "Starting noise analysis".to_string(),
            eta: None,
        });
    }
    
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
    
    // Handle cases with insufficient noise analysis data
    let (average_noise, max_noise, bit_plane_noise) = if all_noise_values.is_empty() {
        log::warn!("No noise analysis data collected, using conservative fallback estimate");
        // Conservative fallback: assume moderate noise for safety
        let fallback_noise = 0.5; // Below significant threshold but not zero
        (fallback_noise, fallback_noise, vec![fallback_noise])
    } else if all_noise_values.len() < 3 {
        log::warn!("Limited noise analysis data ({} samples), results may be less accurate", all_noise_values.len());
        // Use available data but note it's limited
        let mut avg_per_plane = vec![0.0; all_noise_values[0].len()];
        
        for sample in &all_noise_values {
            for (i, &value) in sample.iter().enumerate() {
                avg_per_plane[i] += value;
            }
        }
        
        for value in &mut avg_per_plane {
            *value /= all_noise_values.len() as f64;
        }
        
        let avg = avg_per_plane.iter().sum::<f64>() / avg_per_plane.len() as f64;
        let max = avg_per_plane.iter().cloned().fold(0.0, f64::max);
        (avg, max, avg_per_plane)
    } else {
        // Normal case: sufficient data available
        let mut avg_per_plane = vec![0.0; all_noise_values[0].len()];
        
        for sample in &all_noise_values {
            for (i, &value) in sample.iter().enumerate() {
                avg_per_plane[i] += value;
            }
        }
        
        for value in &mut avg_per_plane {
            *value /= all_noise_values.len() as f64;
        }
        
        let avg = avg_per_plane.iter().sum::<f64>() / avg_per_plane.len() as f64;
        let max = avg_per_plane.iter().cloned().fold(0.0, f64::max);
        (avg, max, avg_per_plane)
    };
    
    let is_hdr = video_props.hdr_info.is_hdr;
    let has_significant_noise = NoiseAnalysis::has_significant_noise(average_noise);
    let recommended_hqdn3d = NoiseAnalysis::calculate_hqdn3d_params(average_noise, is_hdr);
    let recommended_film_grain = NoiseAnalysis::calculate_film_grain_level(average_noise, is_hdr);
    
    log::info!(
        "Noise analysis: avg={:.4}, max={:.4}, significant={}, recommended=hqdn3d={}, film_grain={}",
        average_noise, max_noise, has_significant_noise, recommended_hqdn3d, recommended_film_grain
    );
    
    // Emit completion event
    if let Some(dispatcher) = event_dispatcher {
        let noise_level = if has_significant_noise {
            "significant noise detected"
        } else {
            "minimal noise detected"
        };
        
        dispatcher.emit(Event::StageProgress {
            stage: "noise_analysis".to_string(),
            percent: 100.0,
            message: format!("Noise analysis complete: {}", noise_level),
            eta: None,
        });
    }
    
    Ok(NoiseAnalysis {
        average_noise,
        max_noise,
        bit_plane_noise,
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
                // Extract luma (0.1) channel noise value for primary measurement
                // Focus on luma since film grain is primarily a luminance phenomenon
                if line.contains("lavfi.bitplanenoise.0.1=") {
                    if let Some(noise_part) = line.split("lavfi.bitplanenoise.0.1=").nth(1) {
                        // Parse the noise value, handling potential trailing content
                        let noise_str = noise_part.split_whitespace().next().unwrap_or("").trim();
                        if let Ok(noise_value) = noise_str.parse::<f64>() {
                            // Validate noise value is within expected range (0.0-1.0)
                            if (0.0..=1.0).contains(&noise_value) {
                                noise_values.push(noise_value);
                                found_values = true;
                                log::trace!("Captured luma noise value: {:.6}", noise_value);
                            } else {
                                log::warn!("Invalid noise value out of range: {:.6}", noise_value);
                            }
                        } else {
                            log::warn!("Failed to parse noise value: '{}'", noise_str);
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
        
        // Low noise (SDR) - proportional to spatial denoising (2.0)
        let low_noise_grain = NoiseAnalysis::calculate_film_grain_level(0.65, false);
        assert!(low_noise_grain >= 7 && low_noise_grain <= 10, 
                "Low noise grain should be between 7 and 10, got {}", low_noise_grain);
        
        // Moderate noise (SDR) - proportional to spatial denoising (3.0)
        let moderate_noise_grain = NoiseAnalysis::calculate_film_grain_level(0.75, false);
        assert!(moderate_noise_grain >= 10 && moderate_noise_grain <= 14, 
                "Moderate noise grain should be between 10 and 14, got {}", moderate_noise_grain);
        
        // High noise (SDR) - proportional to spatial denoising (4.0)
        let high_noise_grain = NoiseAnalysis::calculate_film_grain_level(0.85, false);
        assert!(high_noise_grain >= 14 && high_noise_grain <= MAX_FILM_GRAIN_VALUE, 
                "High noise grain should be between 14 and {}, got {}", MAX_FILM_GRAIN_VALUE, high_noise_grain);
    }
    
    #[test]
    fn test_calculate_film_grain_level_hdr() {
        use crate::config::{MIN_FILM_GRAIN_VALUE, MAX_FILM_GRAIN_VALUE};
        
        // Very low noise - minimum film grain (HDR)
        assert_eq!(
            NoiseAnalysis::calculate_film_grain_level(0.5, true),
            MIN_FILM_GRAIN_VALUE
        );
        
        // Low noise (HDR) - proportional to spatial denoising (1.0) - lighter than SDR
        let low_noise_grain = NoiseAnalysis::calculate_film_grain_level(0.65, true);
        assert!(low_noise_grain >= MIN_FILM_GRAIN_VALUE && low_noise_grain <= 7, 
                "HDR low noise grain should be between {} and 7, got {}", MIN_FILM_GRAIN_VALUE, low_noise_grain);
        
        // Moderate noise (HDR) - proportional to spatial denoising (2.0) - lighter than SDR
        let moderate_noise_grain = NoiseAnalysis::calculate_film_grain_level(0.75, true);
        assert!(moderate_noise_grain >= 7 && moderate_noise_grain <= 11, 
                "HDR moderate noise grain should be between 7 and 11, got {}", moderate_noise_grain);
        
        // High noise (HDR) - proportional to spatial denoising (3.0) - lighter than SDR
        let high_noise_grain = NoiseAnalysis::calculate_film_grain_level(0.85, true);
        assert!(high_noise_grain >= 11 && high_noise_grain <= MAX_FILM_GRAIN_VALUE, 
                "HDR high noise grain should be between 11 and {}, got {}", MAX_FILM_GRAIN_VALUE, high_noise_grain);
    }
    
    #[test]
    fn test_extract_spatial_luma_param() {
        // Test normal hqdn3d parameter extraction
        assert_eq!(NoiseAnalysis::extract_spatial_luma_param("2:1.5:3:2.5"), 2.0);
        assert_eq!(NoiseAnalysis::extract_spatial_luma_param("0.5:0.4:1.5:1.5"), 0.5);
        assert_eq!(NoiseAnalysis::extract_spatial_luma_param("4:3.5:5:4.5"), 4.0);
        
        // Test edge cases
        assert_eq!(NoiseAnalysis::extract_spatial_luma_param(""), 1.0); // Fallback
        assert_eq!(NoiseAnalysis::extract_spatial_luma_param("invalid:params"), 1.0); // Fallback
        assert_eq!(NoiseAnalysis::extract_spatial_luma_param("3.5"), 3.5); // Single value
    }
    
    #[test]
    fn test_proportional_film_grain_scaling() {
        // Test that film grain scales appropriately with denoising strength
        
        // Compare SDR low vs high noise - high noise should have more film grain
        let low_noise_grain = NoiseAnalysis::calculate_film_grain_level(0.65, false);
        let high_noise_grain = NoiseAnalysis::calculate_film_grain_level(0.85, false);
        assert!(high_noise_grain > low_noise_grain, 
                "High noise should produce more film grain than low noise: {} vs {}", 
                high_noise_grain, low_noise_grain);
        
        // Compare HDR vs SDR for same noise level - HDR should have less film grain
        let sdr_grain = NoiseAnalysis::calculate_film_grain_level(0.75, false);
        let hdr_grain = NoiseAnalysis::calculate_film_grain_level(0.75, true);
        assert!(hdr_grain <= sdr_grain, 
                "HDR should have less or equal film grain than SDR: {} vs {}", 
                hdr_grain, sdr_grain);
    }
}