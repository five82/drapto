use super::template_presenter::{
    BatchCompleteParams, EncodingCompleteParams, EncodingConfigParams, FileAnalysisParams,
    TemplatePresenter,
};
use super::templates;
use crate::events::{Event, EventHandler};
use crate::utils::calculate_size_reduction;
use console;
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
            Event::HardwareInfo {
                hostname,
                os,
                cpu,
                memory,
                decoder,
            } => {
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
            } => {
                presenter.render_file_analysis(FileAnalysisParams {
                    input_file,
                    duration,
                    resolution,
                    category,
                    dynamic_range,
                    audio_description,
                });
            }

            Event::VideoAnalysisStarted => {
                presenter.start_video_analysis();
            }

            Event::BlackBarDetectionStarted => {
                presenter.start_spinner("Detecting black bars...");
            }

            Event::BlackBarDetectionProgress {
                current: _,
                total: _,
            } => {
                // Spinner handles animation automatically
            }

            Event::BlackBarDetectionComplete {
                crop_required,
                crop_params,
            } => {
                presenter.finish_spinner();
                let message = if crop_params.as_deref() == Some("disabled") {
                    "Crop detection disabled"
                } else {
                    "Crop detection complete"
                };
                presenter.render_video_analysis_results(
                    message,
                    *crop_required,
                    crop_params.as_deref(),
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
                estimated_savings: _,
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
                matrix_coefficients,
                audio_codec,
                audio_description,
            } => {
                presenter.render_encoding_configuration(EncodingConfigParams {
                    encoder,
                    preset,
                    tune,
                    quality,
                    denoising,
                    film_grain,
                    hardware_accel: hardware_accel.as_deref(),
                    pixel_format,
                    matrix_coefficients,
                    audio_codec,
                    audio_description,
                });
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
                    let eta_str = format!(
                        "{:02}:{:02}:{:02}",
                        eta.as_secs() / 3600,
                        (eta.as_secs() % 3600) / 60,
                        eta.as_secs() % 60
                    );

                    let message = format!(
                        "Speed: {}, ETA: {}",
                        templates::format_speed(*speed),
                        eta_str
                    );
                    presenter.update_encoding_progress(progress_pos, Some(&message));
                } else {
                    presenter.update_encoding_progress(progress_pos, None);
                }
            }

            Event::ValidationComplete {
                validation_passed,
                validation_steps,
            } => {
                presenter.finish_encoding_progress();
                presenter.render_validation_complete(*validation_passed, validation_steps);
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
                let reduction = calculate_size_reduction(*original_size, *encoded_size) as f64;
                let original_size_str = format_bytes(*original_size);
                let encoded_size_str = format_bytes(*encoded_size);
                let reduction_str = templates::format_reduction(reduction);

                let total_time_str = format!(
                    "{:02}:{:02}:{:02}",
                    total_time.as_secs() / 3600,
                    (total_time.as_secs() % 3600) / 60,
                    total_time.as_secs() % 60
                );

                presenter.render_encoding_complete(EncodingCompleteParams {
                    input_file,
                    original_size: &original_size_str,
                    encoded_size: &encoded_size_str,
                    reduction: &reduction_str,
                    video_stream,
                    audio_stream,
                    total_time: &total_time_str,
                    average_speed: &templates::format_speed(*average_speed),
                    output_path,
                    emphasize_reduction: false, // color formatting handled by format_reduction
                });
            }

            Event::Error {
                title,
                message,
                context,
                suggestion,
            } => {
                presenter.render_error(title, message, context.as_deref(), suggestion.as_deref());
            }

            Event::Warning { message } => {
                presenter.render_warning(message);
            }

            Event::StatusUpdate {
                label: _,
                value: _,
                emphasize: _,
            } => {
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

            Event::BatchInitializationStarted {
                total_files,
                file_list,
                output_dir,
            } => {
                presenter.render_batch_initialization(*total_files, file_list, output_dir);
            }

            Event::FileProgressContext {
                current_file,
                total_files,
            } => {
                presenter.render_file_progress_context(*current_file, *total_files);
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
                    (*total_original_size - *total_encoded_size) as f64
                        / *total_original_size as f64
                        * 100.0
                } else {
                    0.0
                };

                presenter.render_batch_complete(BatchCompleteParams {
                    successful_count: *successful_count,
                    total_files: *total_files,
                    total_original_size: &format_bytes(*total_original_size),
                    total_encoded_size: &format_bytes(*total_encoded_size),
                    total_reduction_percent: total_reduction,
                    total_time: &format!(
                        "{:02}:{:02}:{:02}",
                        total_duration.as_secs() / 3600,
                        (total_duration.as_secs() % 3600) / 60,
                        total_duration.as_secs() % 60
                    ),
                    average_speed: &templates::format_speed(*average_speed),
                    file_results,
                    validation_passed_count: *validation_passed_count,
                    validation_failed_count: *validation_failed_count,
                });
            }

            Event::NoiseAnalysisStarted => {
                presenter.render_template(&templates::ProcessingStep {
                    message: "Analyzing video noise levels...",
                });
            }

            Event::NoiseAnalysisComplete {
                average_noise,
                has_significant_noise: _,
                recommended_params,
            } => {
                let noise_level_desc = if *average_noise >= 0.8 {
                    "noisy"
                } else if *average_noise >= 0.7 {
                    "somewhat noisy"
                } else if *average_noise >= 0.6 {
                    "slightly noisy"
                } else {
                    "very clean"
                };

                let denoising_strength = if recommended_params.starts_with("4:3.5:5:4.5")
                    || recommended_params.starts_with("3:2.5:4.5:4")
                {
                    "moderate denoising"
                } else if recommended_params.starts_with("3:2.5:4:3.5")
                    || recommended_params.starts_with("2:1.5:3.5:3")
                {
                    "light denoising"
                } else if recommended_params.starts_with("2:1.5:3:2.5")
                    || recommended_params.starts_with("1:0.8:2.5:2")
                {
                    "very light denoising"
                } else {
                    "minimal denoising"
                };

                // Level 2: Success message (subsection level)
                println!(
                    "  {} {}",
                    console::style("✓").dim(),
                    console::style("Noise analysis complete").dim()
                );

                // Level 4: Primary findings (key-value level)
                println!(
                    "      Video quality:    {} ({:.0}%)",
                    noise_level_desc,
                    average_noise * 100.0
                );
                println!(
                    "      Denoising:        {} ({})",
                    denoising_strength, recommended_params
                );
            }

            Event::StageProgress { .. } => {
                // Don't show stage progress in interactive mode - it clutters the output
                // The JSON progress handler will still emit these events for spindle integration
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
