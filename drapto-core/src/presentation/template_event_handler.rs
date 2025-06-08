use crate::events::{Event, EventHandler};
use super::template_presenter::TemplatePresenter;
use super::templates;
use std::sync::Mutex;

pub struct TemplateEventHandler {
    presenter: Mutex<TemplatePresenter>,
}

impl Default for TemplateEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateEventHandler {
    pub fn new() -> Self {
        Self {
            presenter: Mutex::new(TemplatePresenter::new()),
        }
    }
}

impl EventHandler for TemplateEventHandler {
    fn handle(&self, event: &Event) {
        let mut presenter = self.presenter.lock().unwrap();
        
        match event {
            Event::HardwareInfo { hostname, os, cpu, memory, decoder } => {
                presenter.render_hardware_info(hostname, os, cpu, memory, decoder);
            }
            
            Event::InitializationStarted { 
                input_file, 
                output_file: _, 
                duration, 
                resolution,
                category,
                dynamic_range,
                audio_description,
                hardware 
            } => {
                presenter.render_file_analysis(
                    input_file, 
                    duration, 
                    resolution, 
                    category,
                    dynamic_range,
                    audio_description,
                    hardware.as_deref()
                );
            }
            
            Event::VideoAnalysisStarted => {
                presenter.start_video_analysis();
            }
            
            Event::BlackBarDetectionStarted => {
                presenter.start_spinner("Detecting black bars...");
            }
            
            Event::BlackBarDetectionProgress { current: _, total: _ } => {
                // Spinner handles animation automatically
            }
            
            Event::BlackBarDetectionComplete { crop_required, crop_params } => {
                presenter.finish_spinner();
                presenter.render_video_analysis_results(
                    "Crop detection complete",
                    *crop_required,
                    crop_params.as_deref()
                );
            }
            
            Event::ProcessingConfigurationStarted => {
                // This event can be removed or ignored in template system
                // Configuration is shown when applied
            }
            
            Event::ProcessingConfigurationApplied { 
                denoising: _,
                denoising_params: _,
                film_grain: _,
                estimated_size: _,
                estimated_savings: _
            } => {
                // Template system doesn't show intermediate configuration
                // All configuration shown in EncodingConfigurationDisplayed
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
                color_space,
                audio_codec,
                audio_description,
            } => {
                presenter.render_encoding_configuration(
                    encoder,
                    preset,
                    tune,
                    quality,
                    denoising,
                    film_grain,
                    hardware_accel.as_deref(),
                    pixel_format,
                    color_space,
                    audio_codec,
                    audio_description,
                );
            }
            
            Event::EncodingStarted { total_frames: _ } => {
                presenter.start_encoding_progress();
            }
            
            Event::EncodingProgress { 
                current_frame: _,
                total_frames: _,
                percent,
                speed,
                fps: _,
                eta,
                bitrate: _,
            } => {
                let progress_pos = (*percent as u64).min(100);
                
                // Progressive disclosure per design guide
                if *percent >= 5.0 {
                    let eta_str = format!("{:02}:{:02}:{:02}", 
                        eta.as_secs() / 3600,
                        (eta.as_secs() % 3600) / 60,
                        eta.as_secs() % 60
                    );
                    
                    let message = format!("Speed: {}, ETA: {}", templates::format_speed(*speed), eta_str);
                    presenter.update_encoding_progress(progress_pos, Some(&message));
                } else {
                    presenter.update_encoding_progress(progress_pos, None);
                }
            }
            
            Event::EncodingComplete {
                input_file,
                output_file: _,
                original_size,
                encoded_size,
                video_stream,
                audio_stream,
                total_time,
                average_speed,
                output_path,
            } => {
                presenter.finish_encoding_progress();
                
                let reduction = (*original_size - *encoded_size) as f64 / *original_size as f64 * 100.0;
                let original_size_str = format_bytes(*original_size);
                let encoded_size_str = format_bytes(*encoded_size);
                let reduction_str = format!("{:.1}%", reduction);
                
                let total_time_str = format!("{:02}:{:02}:{:02}",
                    total_time.as_secs() / 3600,
                    (total_time.as_secs() % 3600) / 60,
                    total_time.as_secs() % 60
                );
                
                presenter.render_encoding_complete(
                    input_file,
                    &original_size_str,
                    &encoded_size_str,
                    &reduction_str,
                    video_stream,
                    audio_stream,
                    &total_time_str,
                    &templates::format_speed(*average_speed),
                    output_path,
                    reduction > 50.0 // emphasize significant reductions
                );
            }
            
            Event::Error { title, message, context, suggestion } => {
                presenter.render_error(title, message, context.as_deref(), suggestion.as_deref());
            }
            
            Event::Warning { message } => {
                presenter.render_warning(message);
            }
            
            Event::StatusUpdate { label: _, value: _, emphasize: _ } => {
                // Template system handles status via structured data
                // Individual status updates are not used
            }
            
            Event::ProcessingStep { message: _ } => {
                // Template system groups processing steps
                // Individual steps are not displayed
            }
            
            Event::OperationComplete { message } => {
                // Single file completion - show simple message
                presenter.render_operation_complete(message);
            }
            
            Event::BatchInitializationStarted { total_files, file_list, output_dir } => {
                presenter.render_batch_initialization(*total_files, file_list, output_dir);
            }
            
            Event::FileProgressContext { current_file, total_files } => {
                presenter.render_file_progress_context(*current_file, *total_files);
            }
            
            Event::BatchComplete { 
                successful_count, 
                total_files, 
                total_original_size, 
                total_encoded_size, 
                total_duration, 
                average_speed,
                file_results 
            } => {
                let total_reduction = if *total_original_size > 0 {
                    (*total_original_size - *total_encoded_size) as f64 / *total_original_size as f64 * 100.0
                } else {
                    0.0
                };
                
                presenter.render_batch_complete(
                    *successful_count,
                    *total_files,
                    &format_bytes(*total_original_size),
                    &format_bytes(*total_encoded_size),
                    &format!("{:.1}%", total_reduction),
                    &format!("{:02}:{:02}:{:02}", 
                        total_duration.as_secs() / 3600,
                        (total_duration.as_secs() % 3600) / 60,
                        total_duration.as_secs() % 60
                    ),
                    &templates::format_speed(*average_speed),
                    file_results
                );
            }
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    format!("{:.2} {}", size, UNITS[unit_index])
}