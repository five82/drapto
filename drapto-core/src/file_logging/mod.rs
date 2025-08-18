pub mod setup;

use crate::events::{Event, EventHandler};
use crate::utils::calculate_size_reduction;
use log::{info, warn, error, debug};
use std::time::{Duration, Instant};

pub struct FileLoggingHandler {
    last_logged_percent: std::sync::Mutex<u32>,
    last_log_time: std::sync::Mutex<Option<Instant>>,
}

impl Default for FileLoggingHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl FileLoggingHandler {
    pub fn new() -> Self {
        Self {
            last_logged_percent: std::sync::Mutex::new(0),
            last_log_time: std::sync::Mutex::new(None),
        }
    }
    
    pub fn reset_progress_state(&self) {
        *self.last_logged_percent.lock().unwrap() = 0;
        *self.last_log_time.lock().unwrap() = None;
    }
}

impl EventHandler for FileLoggingHandler {
    fn handle(&self, event: &Event) {
        match event {
            Event::HardwareInfo { hostname, os, cpu, memory, decoder } => {
                info!("Hardware information:");
                info!("  Hostname: {}", hostname);
                info!("  OS: {}", os);
                info!("  CPU: {}", cpu);
                info!("  Memory: {}", memory);
                info!("  Decoder: {}", decoder);
            }
            
            Event::InitializationStarted { 
                input_file, 
                output_file, 
                duration, 
                resolution,
                category,
                dynamic_range,
                audio_description 
            } => {
                info!("Starting drapto encoding process");
                info!("Input: {} (duration: {}, resolution: {} {})", input_file, duration, resolution, category);
                info!("Output: {}", output_file);
                info!("Dynamic range: {}, Audio: {}", dynamic_range, audio_description);
            }
            
            Event::VideoAnalysisStarted => {
                info!("Beginning video analysis");
            }
            
            Event::BlackBarDetectionStarted => {
                info!("Starting black bar detection");
            }
            
            Event::BlackBarDetectionProgress { current, total } => {
                let percent = *current as f64 / *total as f64 * 100.0;
                debug!("Black bar detection progress: {:.1}%", percent);
            }
            
            Event::BlackBarDetectionComplete { crop_required, crop_params } => {
                if *crop_required {
                    if let Some(params) = crop_params {
                        info!("Black bar detection complete. Crop detected: {}", params);
                    }
                } else {
                    info!("Black bar detection complete. No crop required");
                }
            }
            
            Event::ProcessingConfigurationStarted => {
                info!("Applying processing configuration");
            }
            
            Event::ProcessingConfigurationApplied { 
                denoising,
                denoising_params,
                film_grain,
                estimated_size,
                estimated_savings
            } => {
                info!("Processing configuration applied:");
                info!("  Denoising: {} ({})", denoising, denoising_params);
                info!("  Film grain: {}", film_grain);
                info!("  Estimated output size: {}", estimated_size);
                info!("  Estimated savings: {}", estimated_savings);
            }
            
            Event::EncodingConfigurationDisplayed {
                encoder,
                preset,
                tune,
                quality,
                denoising,
                film_grain,
                hardware_accel,
                pixel_format,
                matrix_coefficients,
                audio_codec,
                audio_description,
            } => {
                info!("Encoding configuration:");
                info!("  Encoder: {}", encoder);
                info!("  Preset: {}", preset);
                info!("  Tune: {}", tune);
                info!("  Quality (CRF): {}", quality);
                info!("  Denoising: {}", denoising);
                info!("  Film grain synthesis: {}", film_grain);
                if let Some(hw) = hardware_accel {
                    info!("  Hardware acceleration: {}", hw);
                }
                info!("  Pixel format: {}", pixel_format);
                info!("  Matrix: {}", matrix_coefficients);
                info!("  Audio codec: {}", audio_codec);
                info!("  Audio: {}", audio_description);
            }
            
            Event::EncodingStarted { total_frames } => {
                // Reset progress tracking for new encoding
                self.reset_progress_state();
                info!("Starting encoding process ({} frames)", total_frames);
            }
            
            Event::EncodingProgress { 
                current_frame,
                total_frames,
                percent,
                speed,
                fps,
                eta,
                bitrate,
            } => {
                // Only log meaningful progress (skip initial invalid data)
                if *percent > 0.5 && *speed > 0.1 {
                    let current_percent = *percent as u32;
                    let now = Instant::now();
                    
                    let mut last_logged = self.last_logged_percent.lock().unwrap();
                    let mut last_time = self.last_log_time.lock().unwrap();
                    
                    let should_log = if let Some(last_log_time) = *last_time {
                        // Primary: 3% milestones (3%, 6%, 9%, 12%, etc.)
                        let milestone_reached = current_percent >= *last_logged + 3;
                        
                        // Secondary: 5-minute time fallback (ensures regular updates)
                        let time_fallback = now.duration_since(last_log_time) >= Duration::from_secs(300);
                        
                        // Special: Major milestones (25%, 50%, 75%)
                        let major_milestone = [25, 50, 75].contains(&current_percent) && current_percent > *last_logged;
                        
                        milestone_reached || time_fallback || major_milestone
                    } else {
                        // First meaningful progress update
                        true
                    };
                    
                    if should_log {
                        let frame_info = if *total_frames > 0 {
                            format!("{}/{} frames", current_frame, total_frames)
                        } else {
                            format!("frame {}", current_frame)
                        };
                        
                        info!(
                            "Encoding progress: {:.1}% ({}), speed: {:.1}x, fps: {:.1}, bitrate: {}, ETA: {}",
                            percent,
                            frame_info,
                            speed,
                            fps,
                            bitrate,
                            format_duration(eta)
                        );
                        
                        *last_logged = current_percent;
                        *last_time = Some(now);
                    }
                }
            }
            
            Event::ValidationComplete { validation_passed, validation_steps } => {
                if *validation_passed {
                    info!("Post-encode validation completed successfully");
                } else {
                    warn!("Post-encode validation failed");
                }
                
                for (step_name, passed, details) in validation_steps {
                    if *passed {
                        info!("Validation - {}: Passed ({})", step_name, details);
                    } else {
                        warn!("Validation - {}: Failed ({})", step_name, details);
                    }
                }
            }

            Event::EncodingComplete {
                input_file,
                output_file,
                original_size,
                encoded_size,
                video_stream,
                audio_stream,
                total_time,
                average_speed,
                output_path,
            } => {
                let reduction = calculate_size_reduction(*original_size, *encoded_size) as f64;
                
                info!("Encoding completed successfully");
                info!("Input: {} ({} bytes)", input_file, original_size);
                info!("Output: {} ({} bytes)", output_file, encoded_size);
                info!("Size reduction: {:.1}%", reduction);
                info!("Video stream: {}", video_stream);
                info!("Audio stream: {}", audio_stream);
                info!("Total encoding time: {}", format_duration(total_time));
                info!("Average speed: {:.2}x", average_speed);
                info!("Output saved to: {}", output_path);
            }
            
            Event::Error { title, message, context, suggestion } => {
                error!("{}: {}", title, message);
                if let Some(ctx) = context {
                    error!("Context: {}", ctx);
                }
                if let Some(sug) = suggestion {
                    error!("Suggestion: {}", sug);
                }
            }
            
            Event::Warning { message } => {
                warn!("{}", message);
            }
            
            Event::StatusUpdate { label, value, .. } => {
                debug!("{}: {}", label, value);
            }
            
            Event::ProcessingStep { message } => {
                info!("{}", message);
            }
            
            Event::OperationComplete { message } => {
                info!("{}", message);
            }
            
            Event::BatchInitializationStarted { total_files, file_list, output_dir } => {
                info!("Starting batch encoding of {} files", total_files);
                for (i, filename) in file_list.iter().enumerate() {
                    info!("  {}. {}", i + 1, filename);
                }
                info!("Output directory: {}", output_dir);
            }
            
            Event::FileProgressContext { current_file, total_files } => {
                info!("Processing file {} of {}", current_file, total_files);
            }
            
            Event::BatchComplete { 
                successful_count, 
                total_files, 
                total_original_size, 
                total_encoded_size, 
                total_duration, 
                average_speed,
                file_results,
                validation_passed_count,
                validation_failed_count,
            } => {
                let total_reduction = if *total_original_size > 0 {
                    (*total_original_size - *total_encoded_size) as f64 / *total_original_size as f64 * 100.0
                } else {
                    0.0
                };
                
                info!("Batch encoding complete: {} of {} files successful", successful_count, total_files);
                info!("Validation summary: {} passed, {} failed", validation_passed_count, validation_failed_count);
                info!("Total original size: {} bytes", total_original_size);
                info!("Total encoded size: {} bytes", total_encoded_size);
                info!("Total reduction: {:.1}%", total_reduction);
                info!("Total encoding time: {}", format_duration(total_duration));
                info!("Average speed: {:.2}x", average_speed);
                
                for (filename, reduction) in file_results {
                    info!("  {} - {:.1}% reduction", filename, reduction);
                }
            }
            
            Event::NoiseAnalysisStarted => {
                info!("Analyzing video noise levels...");
            }
            
            Event::NoiseAnalysisComplete { 
                average_noise, 
                has_significant_noise, 
                recommended_params 
            } => {
                info!(
                    "Noise analysis complete: avg={:.4}, significant={}, recommended={}",
                    average_noise, has_significant_noise, recommended_params
                );
            }

            Event::StageProgress { stage, percent, message, .. } => {
                debug!("[{}] {:.1}% - {}", stage, percent, message);
            }
        }
    }
}

fn format_duration(duration: &Duration) -> String {
    let secs = duration.as_secs();
    format!("{:02}:{:02}:{:02}", secs / 3600, (secs % 3600) / 60, secs % 60)
}