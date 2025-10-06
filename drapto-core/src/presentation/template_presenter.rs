use super::templates::{self, GroupData, TemplateData};
use console;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Parameters for file analysis rendering
pub struct FileAnalysisParams<'a> {
    pub input_file: &'a str,
    pub duration: &'a str,
    pub resolution: &'a str,
    pub category: &'a str,
    pub dynamic_range: &'a str,
    pub audio_description: &'a str,
}

/// Parameters for encoding configuration rendering
pub struct EncodingConfigParams<'a> {
    pub encoder: &'a str,
    pub preset: &'a str,
    pub tune: &'a str,
    pub quality: &'a str,
    pub denoising: &'a str,
    pub film_grain: &'a str,
    pub hardware_accel: Option<&'a str>,
    pub pixel_format: &'a str,
    pub matrix_coefficients: &'a str,
    pub audio_codec: &'a str,
    pub audio_description: &'a str,
}

/// Parameters for encoding completion rendering
pub struct EncodingCompleteParams<'a> {
    pub input_file: &'a str,
    pub original_size: &'a str,
    pub encoded_size: &'a str,
    pub reduction: &'a str,
    pub video_stream: &'a str,
    pub audio_stream: &'a str,
    pub total_time: &'a str,
    pub average_speed: &'a str,
    pub output_path: &'a str,
    pub emphasize_reduction: bool,
}

/// Parameters for batch completion rendering
pub struct BatchCompleteParams<'a> {
    pub successful_count: usize,
    pub total_files: usize,
    pub total_original_size: &'a str,
    pub total_encoded_size: &'a str,
    pub total_reduction_percent: f64,
    pub total_time: &'a str,
    pub average_speed: &'a str,
    pub file_results: &'a [(String, f64)],
    pub validation_passed_count: usize,
    pub validation_failed_count: usize,
}

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
    pub fn render_hardware_info(
        &self,
        hostname: &str,
        os: &str,
        cpu: &str,
        memory: &str,
        decoder: &str,
    ) {
        let items = vec![
            ("Hostname", hostname),
            ("OS", os),
            ("CPU", cpu),
            ("Memory", memory),
            ("Decoder", decoder),
        ];

        templates::render(TemplateData::HardwareHeader { title: "HARDWARE" });

        for (key, value) in items {
            println!("  {:<18} {}", format!("{}:", key), value);
        }
    }

    /// Render file analysis section with comprehensive file information
    pub fn render_file_analysis(&self, params: FileAnalysisParams) {
        let resolution_with_category = templates::format_technical_info(&format!(
            "{} ({})",
            params.resolution, params.category
        ));
        let formatted_dynamic_range = templates::format_technical_info(params.dynamic_range);
        let formatted_audio = templates::format_technical_info(params.audio_description);

        let items = vec![
            ("File", params.input_file),
            ("Duration", params.duration),
            ("Resolution", &resolution_with_category),
            ("Dynamic range", &formatted_dynamic_range),
            ("Audio", &formatted_audio),
        ];

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
    pub fn render_video_analysis_results(
        &self,
        success_message: &str,
        crop_required: bool,
        crop_params: Option<&str>,
    ) {
        let crop_value = if let Some(params) = crop_params {
            if params == "disabled" {
                "Disabled"
            } else if crop_required {
                params
            } else {
                "None required"
            }
        } else {
            "None required"
        };

        // Just render the success message and result, no section header - dimmed for minor status
        println!(
            "  {} {}",
            console::style("✓").dim(),
            console::style(success_message).dim()
        );
        println!("  {:<18} {}", "Detected crop:", crop_value);
    }

    /// Render encoding configuration with grouped settings
    pub fn render_encoding_configuration(&self, params: EncodingConfigParams) {
        // Format film grain value to be more concise
        let film_grain_formatted = format!("{} (synthesis)", params.film_grain);

        let mut groups = vec![
            GroupData {
                name: "Video",
                items: vec![
                    ("Encoder", params.encoder, false),
                    ("Preset", params.preset, false),
                    ("Tune", params.tune, false),
                    ("Quality", params.quality, false),
                    ("Denoising", params.denoising, false),
                    ("Film grain", &film_grain_formatted, false),
                ],
            },
            GroupData {
                name: "Audio",
                items: vec![
                    ("Audio codec", params.audio_codec, false),
                    ("Audio", params.audio_description, false),
                ],
            },
        ];

        if let Some(hw) = params.hardware_accel {
            groups.push(GroupData {
                name: "Hardware",
                items: vec![("Acceleration", hw, false)],
            });
        }

        let formatted_matrix_coefficients =
            templates::format_technical_info(params.matrix_coefficients);

        groups.push(GroupData {
            name: "Advanced",
            items: vec![
                ("Pixel Format", params.pixel_format, false),
                ("Matrix", &formatted_matrix_coefficients, false),
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
                .progress_chars("##."),
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
                log::debug!(
                    "Ignoring backward progress: {} < {}",
                    pos,
                    self.max_progress
                );
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

    /// Render validation completion summary
    pub fn render_validation_complete(
        &self,
        validation_passed: bool,
        validation_steps: &[(String, bool, String)],
    ) {
        // Add validation group with individual steps using proper colors per design guide
        let validation_results: Vec<String> = validation_steps
            .iter()
            .map(|(_, passed, details)| {
                if *passed {
                    // Green checkmark for successful validation (major milestone)
                    format!("{} {}", console::style("✓").green().bold(), details)
                } else {
                    // Red X for failed validation (critical error)
                    format!("{} {}", console::style("✗").red().bold(), details)
                }
            })
            .collect();

        let validation_items: Vec<(&str, &str, bool)> = validation_steps
            .iter()
            .zip(validation_results.iter())
            .map(|((step_name, _, _), formatted_result)| {
                (step_name.as_str(), formatted_result.as_str(), false)
            })
            .collect();

        let groups = vec![GroupData {
            name: "Post-encode validation",
            items: validation_items,
        }];

        let success_message = if validation_passed {
            "Validation passed"
        } else {
            "Validation failed"
        };

        templates::render(TemplateData::CompletionSummary {
            title: "VALIDATION RESULTS",
            success_message,
            groups,
        });
    }

    /// Render encoding completion summary
    pub fn render_encoding_complete(&self, params: EncodingCompleteParams) {
        let formatted_video_stream = templates::format_technical_info(params.video_stream);
        let formatted_audio_stream = templates::format_technical_info(params.audio_stream);

        let groups = vec![
            GroupData {
                name: "Results",
                items: vec![
                    ("File", params.input_file, false),
                    ("Original size", params.original_size, false),
                    ("Encoded size", params.encoded_size, false),
                    ("Reduction", params.reduction, params.emphasize_reduction),
                ],
            },
            GroupData {
                name: "Streams",
                items: vec![
                    ("Video stream", &formatted_video_stream, false),
                    ("Audio stream", &formatted_audio_stream, false),
                ],
            },
            GroupData {
                name: "Performance",
                items: vec![
                    ("Total time", params.total_time, false),
                    ("Average speed", params.average_speed, false),
                ],
            },
        ];

        templates::render(TemplateData::CompletionSummary {
            title: "ENCODING RESULTS",
            success_message: "Encoding finished successfully",
            groups,
        });

        println!("\n  The encoded file is ready at: {}", params.output_path);
    }

    /// Start a spinner for short operations
    pub fn start_spinner(&mut self, message: &str) -> &ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("  {spinner} {msg}")
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
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
    pub fn render_error(
        &self,
        title: &str,
        message: &str,
        context: Option<&str>,
        suggestion: Option<&str>,
    ) {
        println!(
            "\n  {} {}",
            console::style("✗").bold().red(),
            console::style(title).bold().red()
        );
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
        println!(
            "  {} {}",
            console::style("⚠").bold().yellow(),
            console::style(message).bold().yellow()
        );
    }

    /// Render single file operation complete
    pub fn render_operation_complete(&self, message: &str) {
        println!("\n  ✓ {}", message);
    }

    /// Render batch initialization
    pub fn render_batch_initialization(
        &self,
        total_files: usize,
        file_list: &[String],
        output_dir: &str,
    ) {
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
    pub fn render_batch_complete(&self, params: BatchCompleteParams) {
        templates::render(TemplateData::BatchHeader {
            title: "BATCH COMPLETE",
        });

        println!(
            "  {} Successfully encoded {} files",
            console::style("✓").green().bold(),
            params.successful_count
        );

        // Display validation summary
        if params.validation_failed_count == 0 {
            println!(
                "  {} All {} files passed validation",
                console::style("✓").green().bold(),
                params.validation_passed_count
            );
        } else {
            println!(
                "  {} {} files passed validation, {} failed",
                console::style("⚠").yellow().bold(),
                params.validation_passed_count,
                params.validation_failed_count
            );
        }

        println!();
        println!("  Total original size:   {}", params.total_original_size);
        println!("  Total encoded size:    {}", params.total_encoded_size);
        println!(
            "  Total reduction:       {}",
            templates::format_reduction(params.total_reduction_percent)
        );
        println!("  Total encoding time:   {}", params.total_time);
        println!("  Average speed:         {}", params.average_speed);
        println!();
        println!("  Files processed:");
        for (filename, reduction) in params.file_results {
            println!(
                "    {} {} ({} reduction)",
                console::style("✓").dim(),
                filename,
                templates::format_reduction(*reduction)
            );
        }
        println!();
    }

    /// Render a simple processing step message
    pub fn render_template(&self, step: &templates::ProcessingStep) {
        println!("  {}", step.message);
    }
}
