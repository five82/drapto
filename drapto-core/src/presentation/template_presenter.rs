use super::templates::{self, TemplateData, GroupData};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Template-based presenter that handles all terminal output through consistent templates
pub struct TemplatePresenter {
    progress_bar: Option<ProgressBar>,
    max_progress: u64,
}

impl Default for TemplatePresenter {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplatePresenter {
    pub fn new() -> Self {
        Self {
            progress_bar: None,
            max_progress: 0,
        }
    }

    /// Render hardware information section
    pub fn render_hardware_info(&self, hostname: &str, os: &str, cpu: &str, memory: &str, decoder: &str) {
        let items = vec![
            ("Hostname", hostname),
            ("OS", os),
            ("CPU", cpu),
            ("Memory", memory),
            ("Decoder", decoder),
        ];
        
        templates::render(TemplateData::HardwareHeader {
            title: "HARDWARE",
        });
        
        for (key, value) in items {
            println!("  {:<18} {}", format!("{}:", key), value);
        }
    }

    /// Render file analysis section with comprehensive file information
    pub fn render_file_analysis(&self, input_file: &str, duration: &str, resolution: &str, category: &str, dynamic_range: &str, audio_description: &str, hardware: Option<&str>) {
        let resolution_with_category = format!("{} ({})", resolution, category);
        
        let mut items = vec![
            ("File", input_file),
            ("Duration", duration),
            ("Resolution", &resolution_with_category),
            ("Dynamic range", dynamic_range),
            ("Audio", audio_description),
        ];
        
        if let Some(hw) = hardware {
            items.push(("Hardware", hw));
        }
        
        templates::render(TemplateData::KeyValueList {
            title: "VIDEO DETAILS",
            items,
        });
    }

    /// Start video analysis section (just header, spinner handled separately)
    pub fn start_video_analysis(&self) {
        templates::render(TemplateData::SectionHeader {
            title: "VIDEO ANALYSIS",
        });
    }

    /// Render video analysis results after spinner completes (no header)
    pub fn render_video_analysis_results(&self, success_message: &str, crop_required: bool, crop_params: Option<&str>) {
        let crop_value = if crop_required {
            crop_params.unwrap_or("crop parameters detected")
        } else {
            "None required"
        };
        
        // Just render the success message and result, no section header
        println!("  {} {}", console::style("✓").bold(), console::style(success_message).bold());
        println!("  {:<18} {}", "Detected crop:", crop_value);
    }

    /// Render encoding configuration with grouped settings
    pub fn render_encoding_configuration(
        &self,
        encoder: &str,
        preset: &str,
        tune: &str,
        quality: &str, 
        denoising: &str,
        film_grain: &str,
        hardware_accel: Option<&str>,
        pixel_format: &str,
        color_space: &str,
        audio_codec: &str,
        audio_description: &str,
    ) {
        // Format film grain value to be more concise
        let film_grain_formatted = format!("{} (synthesis)", film_grain);
        
        let mut groups = vec![
            GroupData {
                name: "Video",
                items: vec![
                    ("Encoder", encoder, false),
                    ("Preset", preset, false),
                    ("Tune", tune, false),
                    ("Quality", quality, false),
                    ("Denoising", denoising, false),
                    ("Film grain", &film_grain_formatted, false),
                ],
            },
            GroupData {
                name: "Audio",
                items: vec![
                    ("Audio codec", audio_codec, false),
                    ("Audio", audio_description, false),
                ],
            }
        ];
        
        if let Some(hw) = hardware_accel {
            groups.push(GroupData {
                name: "Hardware",
                items: vec![("Acceleration", hw, false)],
            });
        }
        
        groups.push(GroupData {
            name: "Advanced",
            items: vec![
                ("Pixel Format", pixel_format, false),
                ("Color Space", color_space, false),
            ],
        });
        
        templates::render(TemplateData::GroupedKeyValues {
            title: "ENCODING CONFIGURATION",
            groups,
        });
    }

    /// Start encoding progress section and return progress bar
    pub fn start_encoding_progress(&mut self) -> &ProgressBar {
        templates::render(TemplateData::SectionHeader {
            title: "ENCODING PROGRESS",
        });
        
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("  Encoding: {percent:>3}% [{bar:30}] ({elapsed} / {duration})\n{msg}")
                .unwrap()
                .progress_chars("##.")
        );
        pb.enable_steady_tick(Duration::from_millis(100));
        self.progress_bar = Some(pb);
        self.max_progress = 0; // Reset max progress for new encoding
        self.progress_bar.as_ref().unwrap()
    }

    /// Update encoding progress with additional details (prevents backward progress)
    pub fn update_encoding_progress(&mut self, pos: u64, message: Option<&str>) {
        if let Some(pb) = &self.progress_bar {
            // Only update if the new position is greater than or equal to current max
            // This prevents the progress bar from going backwards due to FFmpeg reporting issues
            if pos >= self.max_progress {
                self.max_progress = pos;
                pb.set_position(pos);
            } else {
                // FFmpeg reported progress going backwards - ignore it
                log::debug!("Ignoring backward progress: {} < {}", pos, self.max_progress);
            }
            
            if let Some(msg) = message {
                pb.set_message(format!("  {}", msg));
            }
        }
    }

    /// Finish encoding progress
    pub fn finish_encoding_progress(&mut self) {
        if let Some(pb) = self.progress_bar.take() {
            pb.finish();
            println!();
        }
    }

    /// Render encoding completion summary
    pub fn render_encoding_complete(
        &self,
        input_file: &str,
        original_size: &str,
        encoded_size: &str,
        reduction: &str,
        video_stream: &str,
        audio_stream: &str,
        total_time: &str,
        average_speed: &str,
        output_path: &str,
        emphasize_reduction: bool,
    ) {
        let groups = vec![
            GroupData {
                name: "Results",
                items: vec![
                    ("File", input_file, false),
                    ("Original size", original_size, false),
                    ("Encoded size", encoded_size, false),
                    ("Reduction", reduction, emphasize_reduction),
                ],
            },
            GroupData {
                name: "Streams",
                items: vec![
                    ("Video stream", video_stream, false),
                    ("Audio stream", audio_stream, false),
                ],
            },
            GroupData {
                name: "Performance",
                items: vec![
                    ("Total time", total_time, false),
                    ("Average speed", average_speed, false),
                ],
            },
        ];
        
        templates::render(TemplateData::CompletionSummary {
            title: "ENCODING RESULTS",
            success_message: "Encoding finished successfully",
            groups,
        });
        
        println!("\n  The encoded file is ready at: {}", output_path);
    }

    /// Start a spinner for short operations
    pub fn start_spinner(&mut self, message: &str) -> &ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("  {spinner} {msg}")
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
        );
        pb.set_message(message.to_string());
        pb.enable_steady_tick(Duration::from_millis(120));
        self.progress_bar = Some(pb);
        self.progress_bar.as_ref().unwrap()
    }

    /// Finish spinner
    pub fn finish_spinner(&mut self) {
        if let Some(pb) = self.progress_bar.take() {
            pb.finish_and_clear();
        }
    }

    /// Render error using template
    pub fn render_error(&self, title: &str, message: &str, context: Option<&str>, suggestion: Option<&str>) {
        println!("\n  {} {}", console::style("✗").bold().red(), console::style(title).bold().red());
        println!();
        println!("  {:<18} {}", "Message:", message);
        if let Some(ctx) = context {
            println!("  {:<18} {}", "Context:", ctx);
        }
        if let Some(sug) = suggestion {
            println!();
            println!("  {:<18} {}", "Suggestion:", sug);
        }
    }

    /// Render warning
    pub fn render_warning(&self, message: &str) {
        println!("  {} {}", console::style("⚠").bold().yellow(), console::style(message).bold().yellow());
    }
    
    /// Render single file operation complete
    pub fn render_operation_complete(&self, message: &str) {
        println!("\n  ✓ {}", message);
    }
    
    /// Render batch initialization
    pub fn render_batch_initialization(&self, total_files: usize, file_list: &[String], output_dir: &str) {
        templates::render(TemplateData::BatchHeader {
            title: "BATCH ENCODING",
        });
        
        println!("  Processing {} files:", total_files);
        for (i, filename) in file_list.iter().enumerate() {
            println!("    {}. {}", i + 1, filename);
        }
        println!();
        println!("  Output directory: {}", output_dir);
    }
    
    /// Render file progress context
    pub fn render_file_progress_context(&self, current_file: usize, total_files: usize) {
        templates::render(TemplateData::FileProgressHeader {
            current: current_file,
            total: total_files,
        });
    }
    
    /// Render batch complete summary
    pub fn render_batch_complete(
        &self,
        successful_count: usize,
        _total_files: usize,
        total_original_size: &str,
        total_encoded_size: &str,
        total_reduction_percent: f64,
        total_time: &str,
        average_speed: &str,
        file_results: &[(String, f64)],
    ) {
        templates::render(TemplateData::BatchHeader {
            title: "BATCH COMPLETE",
        });
        
        println!("  ✓ Successfully encoded {} files", successful_count);
        println!();
        println!("  Total original size:   {}", total_original_size);
        println!("  Total encoded size:    {}", total_encoded_size);
        println!("  Total reduction:       {}", templates::format_reduction(total_reduction_percent));
        println!("  Total encoding time:   {}", total_time);
        println!("  Average speed:         {}", average_speed);
        println!();
        println!("  Files processed:");
        for (filename, reduction) in file_results {
            println!("    ✓ {} ({} reduction)", filename, templates::format_reduction(*reduction));
        }
        println!();
    }
}