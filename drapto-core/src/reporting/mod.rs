use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::json;
use std::io::{self, Write};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Summary of host hardware for display.
#[derive(Clone, Debug)]
pub struct HardwareSummary {
    pub hostname: String,
}

/// Description of the current file before encoding begins.
#[derive(Clone, Debug)]
pub struct InitializationSummary {
    pub input_file: String,
    pub output_file: String,
    pub duration: String,
    pub resolution: String,
    pub category: String,
    pub dynamic_range: String,
    pub audio_description: String,
}

/// Result of crop detection.
#[derive(Clone, Debug)]
pub struct CropSummary {
    pub message: String,
    pub crop: Option<String>,
    pub required: bool,
    pub disabled: bool,
}

/// Encoding configuration that will be used for the current file.
#[derive(Clone, Debug)]
pub struct EncodingConfigSummary {
    pub encoder: String,
    pub preset: String,
    pub tune: String,
    pub quality: String,
    pub pixel_format: String,
    pub matrix_coefficients: String,
    pub audio_codec: String,
    pub audio_description: String,
    pub drapto_preset: String,
    pub drapto_preset_settings: Vec<(String, String)>,
    pub svtav1_params: String,
}

/// Snapshot of encoding progress.
#[derive(Clone, Debug)]
pub struct ProgressSnapshot {
    pub current_frame: u64,
    pub total_frames: u64,
    pub percent: f32,
    pub speed: f32,
    pub fps: f32,
    pub eta: Duration,
    pub bitrate: String,
}

/// Validation results after encode completes.
#[derive(Clone, Debug)]
pub struct ValidationSummary {
    pub passed: bool,
    pub steps: Vec<(String, bool, String)>,
}

/// Final encoding outcome.
#[derive(Clone, Debug)]
pub struct EncodingOutcome {
    pub input_file: String,
    pub output_file: String,
    pub original_size: u64,
    pub encoded_size: u64,
    pub video_stream: String,
    pub audio_stream: String,
    pub total_time: Duration,
    pub average_speed: f32,
    pub output_path: String,
}

/// High-level warning/error message.
#[derive(Clone, Debug)]
pub struct ReporterError {
    pub title: String,
    pub message: String,
    pub context: Option<String>,
    pub suggestion: Option<String>,
}

/// Batch start metadata.
#[derive(Clone, Debug)]
pub struct BatchStartInfo {
    pub total_files: usize,
    pub file_list: Vec<String>,
    pub output_dir: String,
}

/// Current file index within a batch.
#[derive(Clone, Debug)]
pub struct FileProgressContext {
    pub current_file: usize,
    pub total_files: usize,
}

/// Batch completion summary.
#[derive(Clone, Debug)]
pub struct BatchSummary {
    pub successful_count: usize,
    pub total_files: usize,
    pub total_original_size: u64,
    pub total_encoded_size: u64,
    pub total_duration: Duration,
    pub average_speed: f32,
    pub file_results: Vec<(String, f64)>,
    pub validation_passed_count: usize,
    pub validation_failed_count: usize,
}

/// Generic stage update (analysis, validation, etc.).
#[derive(Clone, Debug)]
pub struct StageProgress {
    pub stage: String,
    pub percent: f32,
    pub message: String,
    pub eta: Option<Duration>,
}

/// Reporter interface implemented by both human-readable and JSON reporters.
pub trait Reporter: Send + Sync {
    fn hardware(&self, _summary: &HardwareSummary) {}
    fn initialization(&self, _summary: &InitializationSummary) {}
    fn stage_progress(&self, _update: &StageProgress) {}
    fn crop_result(&self, _summary: &CropSummary) {}
    fn encoding_config(&self, _summary: &EncodingConfigSummary) {}
    fn encoding_started(&self, _total_frames: u64) {}
    fn encoding_progress(&self, _progress: &ProgressSnapshot) {}
    fn validation_complete(&self, _summary: &ValidationSummary) {}
    fn encoding_complete(&self, _summary: &EncodingOutcome) {}
    fn warning(&self, _message: &str) {}
    fn error(&self, _error: &ReporterError) {}
    fn operation_complete(&self, _message: &str) {}
    fn batch_started(&self, _info: &BatchStartInfo) {}
    fn file_progress(&self, _context: &FileProgressContext) {}
    fn batch_complete(&self, _summary: &BatchSummary) {}
}

/// No-op reporter that discards all updates.
pub struct NullReporter;

impl Reporter for NullReporter {}

/// Reporter that fan-outs every event to a set of child reporters.
pub struct CompositeReporter {
    reporters: Vec<Box<dyn Reporter>>,
}

impl CompositeReporter {
    pub fn new(reporters: Vec<Box<dyn Reporter>>) -> Self {
        Self { reporters }
    }

    fn broadcast<F>(&self, f: F)
    where
        F: Fn(&Box<dyn Reporter>),
    {
        for reporter in &self.reporters {
            f(reporter);
        }
    }
}

impl Reporter for CompositeReporter {
    fn hardware(&self, summary: &HardwareSummary) {
        self.broadcast(|r| r.hardware(summary));
    }

    fn initialization(&self, summary: &InitializationSummary) {
        self.broadcast(|r| r.initialization(summary));
    }

    fn stage_progress(&self, update: &StageProgress) {
        self.broadcast(|r| r.stage_progress(update));
    }

    fn crop_result(&self, summary: &CropSummary) {
        self.broadcast(|r| r.crop_result(summary));
    }

    fn encoding_config(&self, summary: &EncodingConfigSummary) {
        self.broadcast(|r| r.encoding_config(summary));
    }

    fn encoding_started(&self, total_frames: u64) {
        self.broadcast(|r| r.encoding_started(total_frames));
    }

    fn encoding_progress(&self, progress: &ProgressSnapshot) {
        self.broadcast(|r| r.encoding_progress(progress));
    }

    fn validation_complete(&self, summary: &ValidationSummary) {
        self.broadcast(|r| r.validation_complete(summary));
    }

    fn encoding_complete(&self, summary: &EncodingOutcome) {
        self.broadcast(|r| r.encoding_complete(summary));
    }

    fn warning(&self, message: &str) {
        self.broadcast(|r| r.warning(message));
    }

    fn error(&self, error: &ReporterError) {
        self.broadcast(|r| r.error(error));
    }

    fn operation_complete(&self, message: &str) {
        self.broadcast(|r| r.operation_complete(message));
    }

    fn batch_started(&self, info: &BatchStartInfo) {
        self.broadcast(|r| r.batch_started(info));
    }

    fn file_progress(&self, context: &FileProgressContext) {
        self.broadcast(|r| r.file_progress(context));
    }

    fn batch_complete(&self, summary: &BatchSummary) {
        self.broadcast(|r| r.batch_complete(summary));
    }
}

/// Human-friendly reporter that prints concise text output.
pub struct TerminalReporter {
    progress: Mutex<Option<ProgressBar>>,
    max_percent: Mutex<f32>,
    last_stage: Mutex<Option<String>>,
}

impl TerminalReporter {
    pub fn new() -> Self {
        Self {
            progress: Mutex::new(None),
            max_percent: Mutex::new(0.0),
            last_stage: Mutex::new(None),
        }
    }

    fn finish_progress(&self) {
        if let Some(pb) = self.progress.lock().unwrap().take() {
            pb.finish_and_clear();
        }
        *self.max_percent.lock().unwrap() = 0.0;
    }

    fn update_progress_bar(&self, progress: &ProgressSnapshot) {
        let guard = self.progress.lock().unwrap();
        if guard.is_none() {
            return;
        }
        let pb = guard.as_ref().unwrap();
        let mut max_percent = self.max_percent.lock().unwrap();

        let clamped = progress.percent.clamp(0.0, 100.0);
        if clamped >= *max_percent {
            *max_percent = clamped;
            pb.set_position(clamped as u64);
        }

        let msg = format!(
            "speed {:.1}x, fps {:.1}, eta {}",
            progress.speed,
            progress.fps,
            format_duration(&progress.eta)
        );
        pb.set_message(msg);
    }
}

impl Reporter for TerminalReporter {
    fn hardware(&self, summary: &HardwareSummary) {
        println!("\n{}", style("HARDWARE").bold().cyan());
        println!("  {:<10} {}", style("Hostname:").bold(), summary.hostname);
    }

    fn initialization(&self, summary: &InitializationSummary) {
        println!("\n{}", style("VIDEO").bold().cyan());
        println!("  {:<10} {}", style("File:").bold(), summary.input_file);
        println!("  {:<10} {}", style("Output:").bold(), summary.output_file);
        println!("  {:<10} {}", style("Duration:").bold(), summary.duration);
        println!(
            "  {:<10} {} ({})",
            style("Resolution:").bold(),
            summary.resolution,
            summary.category
        );
        println!(
            "  {:<10} {}",
            style("Dynamic:").bold(),
            summary.dynamic_range
        );
        println!(
            "  {:<10} {}",
            style("Audio:").bold(),
            summary.audio_description
        );
    }

    fn stage_progress(&self, update: &StageProgress) {
        let mut last = self.last_stage.lock().unwrap();
        if last.as_deref() != Some(update.stage.as_str()) {
            println!("\n{}", style(update.stage.to_uppercase()).bold().cyan());
            *last = Some(update.stage.clone());
        }
        println!("  {}{}", style("› ").magenta(), update.message);
    }

    fn crop_result(&self, summary: &CropSummary) {
        let status = if summary.disabled {
            style("auto-crop disabled").dim().to_string()
        } else if summary.required {
            style(summary.crop.as_deref().unwrap_or("crop params unavailable"))
                .green()
                .to_string()
        } else {
            style("no crop needed").dim().to_string()
        };
        println!(
            "  {} {} ({})",
            style("Crop detection:").bold(),
            summary.message,
            status
        );
    }

    fn encoding_config(&self, summary: &EncodingConfigSummary) {
        println!("\n{}", style("ENCODING").bold().cyan());
        println!("  {:<13} {}", style("Encoder:").bold(), summary.encoder);
        println!("  {:<13} {}", style("Preset:").bold(), summary.preset);
        println!("  {:<13} {}", style("Tune:").bold(), summary.tune);
        println!("  {:<13} {}", style("Quality:").bold(), summary.quality);
        println!(
            "  {:<13} {}",
            style("Pixel format:").bold(),
            summary.pixel_format
        );
        println!(
            "  {:<13} {}",
            style("Matrix:").bold(),
            summary.matrix_coefficients
        );
        println!(
            "  {:<13} {}",
            style("Audio codec:").bold(),
            summary.audio_codec
        );
        println!(
            "  {:<13} {}",
            style("Audio:").bold(),
            summary.audio_description
        );
        println!(
            "  {:<13} {}",
            style("Drapto preset:").bold(),
            summary.drapto_preset
        );
        if !summary.drapto_preset_settings.is_empty() {
            let combined = summary
                .drapto_preset_settings
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(", ");
            println!("  {:<13} {}", style("Preset values:").bold(), combined);
        }
        if !summary.svtav1_params.is_empty() {
            println!(
                "  {:<13} {}",
                style("SVT params:").bold(),
                summary.svtav1_params
            );
        }
    }

    fn encoding_started(&self, _total_frames: u64) {
        self.finish_progress();
        let pb = ProgressBar::new(100);
        let style = ProgressStyle::default_bar()
            .template("Encoding [{bar:40}] {percent:>3}% | {msg}")
            .unwrap()
            .progress_chars("=> ");
        pb.set_style(style);
        pb.enable_steady_tick(Duration::from_millis(120));
        *self.progress.lock().unwrap() = Some(pb);
    }

    fn encoding_progress(&self, progress: &ProgressSnapshot) {
        self.update_progress_bar(progress);
    }

    fn validation_complete(&self, summary: &ValidationSummary) {
        self.finish_progress();
        println!("\n{}", style("VALIDATION").bold().cyan());
        if summary.passed {
            println!("  {}", style("All checks passed").green().bold());
        } else {
            println!("  {}", style("Validation failed").red().bold());
        }
        for (name, passed, details) in &summary.steps {
            let styled_status = if *passed {
                style("ok").green()
            } else {
                style("failed").red().bold()
            };
            println!("  - {}: {} ({})", name, styled_status, details);
        }
    }

    fn encoding_complete(&self, summary: &EncodingOutcome) {
        println!("\n{}", style("RESULTS").bold().cyan());
        println!(
            "  {} {}",
            style("Output:").bold(),
            style(&summary.output_file).bold()
        );
        println!(
            "  Size: {} -> {}",
            format_size_readable(summary.original_size),
            format_size_readable(summary.encoded_size)
        );
        println!(
            "  Reduction: {}",
            style(format!(
                "{:.1}%",
                calculate_reduction(summary.original_size, summary.encoded_size)
            ))
            .bold()
        );
        println!("  {:<8} {}", style("Video:").bold(), summary.video_stream);
        println!("  {:<8} {}", style("Audio:").bold(), summary.audio_stream);
        println!(
            "  Time: {} (avg speed {:.1}x)",
            format_duration(&summary.total_time),
            summary.average_speed
        );
        println!(
            "  {} {}",
            style("Saved to").bold(),
            style(&summary.output_path).green()
        );
    }

    fn warning(&self, message: &str) {
        println!("\n{}", style(format!("WARN: {}", message)).yellow().bold());
    }

    fn error(&self, error: &ReporterError) {
        eprintln!(
            "\n{} {}",
            style("ERROR").red().bold(),
            style(&error.title).red().bold()
        );
        eprintln!("  {}", error.message);
        if let Some(ctx) = &error.context {
            eprintln!("  Context: {}", ctx);
        }
        if let Some(suggestion) = &error.suggestion {
            eprintln!("  Suggestion: {}", suggestion);
        }
    }

    fn operation_complete(&self, message: &str) {
        println!("\n{} {}", style("✓").green().bold(), style(message).bold());
    }

    fn batch_started(&self, info: &BatchStartInfo) {
        println!("\n{}", style("BATCH").bold().cyan());
        println!(
            "  Processing {} files -> {}",
            info.total_files,
            style(&info.output_dir).bold()
        );
        for (idx, name) in info.file_list.iter().enumerate() {
            println!("  {}. {}", idx + 1, name);
        }
    }

    fn file_progress(&self, context: &FileProgressContext) {
        println!(
            "\nFile {} of {}",
            style(context.current_file.to_string()).bold(),
            context.total_files
        );
    }

    fn batch_complete(&self, summary: &BatchSummary) {
        println!("\n{}", style("BATCH SUMMARY").bold().cyan());
        println!(
            "  {}",
            style(format!(
                "{} of {} succeeded",
                summary.successful_count, summary.total_files
            ))
            .bold()
        );
        println!(
            "  Validation: {} passed, {} failed",
            style(summary.validation_passed_count.to_string()).green(),
            style(summary.validation_failed_count.to_string()).red()
        );
        println!(
            "  Size: {} -> {} bytes ({:.1}% reduction)",
            summary.total_original_size,
            summary.total_encoded_size,
            calculate_reduction(summary.total_original_size, summary.total_encoded_size)
        );
        println!(
            "  Time: {} (avg speed {:.1}x)",
            format_duration(&summary.total_duration),
            summary.average_speed
        );
        for (file, reduction) in &summary.file_results {
            println!("  - {} ({:.1}% reduction)", file, reduction);
        }
    }
}

/// Reporter that mirrors the terminal narrative into the structured log output.
pub struct LogReporter {
    last_stage: Mutex<Option<String>>,
    last_progress_bucket: Mutex<i32>,
}

impl LogReporter {
    pub fn new() -> Self {
        Self {
            last_stage: Mutex::new(None),
            last_progress_bucket: Mutex::new(-1),
        }
    }

    fn log_section(&self, heading: &str, lines: &[String]) {
        log::info!("");
        log::info!("{}", heading);
        for line in lines {
            log::info!("{}", line);
        }
    }

    fn log_plain_block(&self, heading: &str, lines: &[String]) {
        if !heading.is_empty() {
            self.log_section(heading, lines);
        } else {
            log::info!("");
            for line in lines {
                log::info!("{}", line);
            }
        }
    }
}

impl Reporter for LogReporter {
    fn hardware(&self, summary: &HardwareSummary) {
        self.log_section("HARDWARE", &hardware_lines(summary));
    }

    fn initialization(&self, summary: &InitializationSummary) {
        self.log_section("VIDEO", &video_lines(summary));
    }

    fn stage_progress(&self, update: &StageProgress) {
        let mut last = self.last_stage.lock().unwrap();
        if last.as_deref() != Some(update.stage.as_str()) {
            log::info!("");
            log::info!("{}", update.stage.to_uppercase());
            *last = Some(update.stage.clone());
        }
        log::info!("  {}{}", "\u{203A} ", update.message);
    }

    fn crop_result(&self, summary: &CropSummary) {
        let line = crop_line(summary);
        log::info!("{}", line);
    }

    fn encoding_config(&self, summary: &EncodingConfigSummary) {
        self.log_section("ENCODING", &encoding_config_lines(summary));
    }

    fn encoding_started(&self, _total_frames: u64) {
        *self.last_progress_bucket.lock().unwrap() = -1;
    }

    fn encoding_progress(&self, progress: &ProgressSnapshot) {
        let bucket = (progress.percent as i32) / 5;
        let mut guard = self.last_progress_bucket.lock().unwrap();
        if bucket <= *guard && progress.percent < 99.0 {
            return;
        }
        *guard = bucket;
        log::info!(
            "Encoding progress: {:>5.1}% | speed {:.1}x | fps {:.1} | eta {}",
            progress.percent,
            progress.speed,
            progress.fps,
            format_duration(&progress.eta)
        );
    }

    fn validation_complete(&self, summary: &ValidationSummary) {
        self.log_section("VALIDATION", &validation_lines(summary));
    }

    fn encoding_complete(&self, summary: &EncodingOutcome) {
        self.log_section("RESULTS", &result_lines(summary));
    }

    fn warning(&self, message: &str) {
        self.log_plain_block("", &[format!("WARN: {}", message)]);
    }

    fn error(&self, error: &ReporterError) {
        log::error!("");
        log::error!("ERROR {}", error.title);
        log::error!("  {}", error.message);
        if let Some(ctx) = &error.context {
            log::error!("  Context: {}", ctx);
        }
        if let Some(suggestion) = &error.suggestion {
            log::error!("  Suggestion: {}", suggestion);
        }
    }

    fn operation_complete(&self, message: &str) {
        self.log_plain_block("", &[format!("✓ {}", message)]);
    }

    fn batch_started(&self, info: &BatchStartInfo) {
        self.log_section("BATCH", &batch_start_lines(info));
    }

    fn file_progress(&self, context: &FileProgressContext) {
        log::info!("");
        log::info!("File {} of {}", context.current_file, context.total_files);
    }

    fn batch_complete(&self, summary: &BatchSummary) {
        self.log_section("BATCH SUMMARY", &batch_complete_lines(summary));
    }
}

/// JSON reporter compatible with Spindle's expectations.
pub struct JsonReporter {
    writer: Mutex<Box<dyn Write + Send>>,
    last_progress_bucket: Mutex<i32>,
}

impl JsonReporter {
    pub fn new() -> Self {
        Self::with_writer(Box::new(io::stdout()))
    }

    pub fn with_writer(writer: Box<dyn Write + Send>) -> Self {
        Self {
            writer: Mutex::new(writer),
            last_progress_bucket: Mutex::new(-1),
        }
    }

    fn timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn write_value(&self, value: serde_json::Value) {
        if let Ok(mut writer) = self.writer.lock() {
            let _ = writeln!(writer, "{}", value);
            let _ = writer.flush();
        }
    }
}

impl Reporter for JsonReporter {
    fn hardware(&self, summary: &HardwareSummary) {
        let value = json!({
            "type": "hardware",
            "hostname": summary.hostname,
            "timestamp": Self::timestamp(),
        });
        self.write_value(value);
    }

    fn initialization(&self, summary: &InitializationSummary) {
        let value = json!({
            "type": "initialization",
            "input_file": summary.input_file,
            "output_file": summary.output_file,
            "duration": summary.duration,
            "resolution": summary.resolution,
            "category": summary.category,
            "dynamic_range": summary.dynamic_range,
            "audio_description": summary.audio_description,
            "timestamp": Self::timestamp(),
        });
        self.write_value(value);
    }

    fn stage_progress(&self, update: &StageProgress) {
        let value = json!({
            "type": "stage_progress",
            "stage": update.stage,
            "percent": update.percent,
            "message": update.message,
            "eta_seconds": update.eta.map(|d| d.as_secs()),
            "timestamp": Self::timestamp(),
        });
        self.write_value(value);
    }

    fn crop_result(&self, summary: &CropSummary) {
        let value = json!({
            "type": "crop_result",
            "message": summary.message,
            "crop": summary.crop,
            "required": summary.required,
            "disabled": summary.disabled,
            "timestamp": Self::timestamp(),
        });
        self.write_value(value);
    }

    fn encoding_config(&self, summary: &EncodingConfigSummary) {
        let preset_settings: Vec<_> = summary
            .drapto_preset_settings
            .iter()
            .map(|(key, value)| json!({"key": key, "value": value}))
            .collect();
        let value = json!({
            "type": "encoding_config",
            "encoder": summary.encoder,
            "preset": summary.preset,
            "tune": summary.tune,
            "quality": summary.quality,
            "pixel_format": summary.pixel_format,
            "matrix_coefficients": summary.matrix_coefficients,
            "audio_codec": summary.audio_codec,
            "audio_description": summary.audio_description,
            "drapto_preset": summary.drapto_preset,
            "drapto_preset_settings": preset_settings,
            "svtav1_params": summary.svtav1_params,
            "timestamp": Self::timestamp(),
        });
        self.write_value(value);
    }

    fn encoding_started(&self, total_frames: u64) {
        *self.last_progress_bucket.lock().unwrap() = -1;
        let value = json!({
            "type": "encoding_started",
            "total_frames": total_frames,
            "timestamp": Self::timestamp(),
        });
        self.write_value(value);
    }

    fn encoding_progress(&self, progress: &ProgressSnapshot) {
        let bucket = (progress.percent as i32) / 5;
        let mut guard = self.last_progress_bucket.lock().unwrap();
        if bucket <= *guard && progress.percent < 99.0 {
            return;
        }
        *guard = bucket;

        let value = json!({
            "type": "encoding_progress",
            "stage": "encoding",
            "current_frame": progress.current_frame,
            "total_frames": progress.total_frames,
            "percent": progress.percent,
            "speed": progress.speed,
            "fps": progress.fps,
            "eta_seconds": progress.eta.as_secs(),
            "bitrate": progress.bitrate,
            "timestamp": Self::timestamp(),
        });
        self.write_value(value);
    }

    fn validation_complete(&self, summary: &ValidationSummary) {
        let steps: Vec<_> = summary
            .steps
            .iter()
            .map(|(step, passed, details)| {
                json!({
                    "step": step,
                    "passed": passed,
                    "details": details
                })
            })
            .collect();

        let value = json!({
            "type": "validation_complete",
            "validation_passed": summary.passed,
            "validation_steps": steps,
            "timestamp": Self::timestamp(),
        });
        self.write_value(value);
    }

    fn encoding_complete(&self, summary: &EncodingOutcome) {
        let value = json!({
            "type": "encoding_complete",
            "input_file": summary.input_file,
            "output_file": summary.output_file,
            "original_size": summary.original_size,
            "encoded_size": summary.encoded_size,
            "video_stream": summary.video_stream,
            "audio_stream": summary.audio_stream,
            "average_speed": summary.average_speed,
            "output_path": summary.output_path,
            "duration_seconds": summary.total_time.as_secs(),
            "size_reduction_percent": calculate_reduction(summary.original_size, summary.encoded_size),
            "timestamp": Self::timestamp(),
        });
        self.write_value(value);
    }

    fn warning(&self, message: &str) {
        let value = json!({
            "type": "warning",
            "message": message,
            "timestamp": Self::timestamp(),
        });
        self.write_value(value);
    }

    fn error(&self, error: &ReporterError) {
        let value = json!({
            "type": "error",
            "title": error.title,
            "message": error.message,
            "context": error.context,
            "suggestion": error.suggestion,
            "timestamp": Self::timestamp(),
        });
        self.write_value(value);
    }

    fn operation_complete(&self, message: &str) {
        let value = json!({
            "type": "operation_complete",
            "message": message,
            "timestamp": Self::timestamp(),
        });
        self.write_value(value);
    }

    fn batch_started(&self, info: &BatchStartInfo) {
        let value = json!({
            "type": "batch_started",
            "total_files": info.total_files,
            "file_list": info.file_list,
            "output_dir": info.output_dir,
            "timestamp": Self::timestamp(),
        });
        self.write_value(value);
    }

    fn file_progress(&self, context: &FileProgressContext) {
        let value = json!({
            "type": "file_progress",
            "current_file": context.current_file,
            "total_files": context.total_files,
            "timestamp": Self::timestamp(),
        });
        self.write_value(value);
    }

    fn batch_complete(&self, summary: &BatchSummary) {
        let value = json!({
            "type": "batch_complete",
            "successful_count": summary.successful_count,
            "total_files": summary.total_files,
            "total_original_size": summary.total_original_size,
            "total_encoded_size": summary.total_encoded_size,
            "total_duration_seconds": summary.total_duration.as_secs(),
            "total_size_reduction_percent": calculate_reduction(summary.total_original_size, summary.total_encoded_size),
            "timestamp": Self::timestamp(),
        });
        self.write_value(value);
    }
}

fn hardware_lines(summary: &HardwareSummary) -> Vec<String> {
    vec![format!("  {:<10} {}", "Hostname:", summary.hostname)]
}

fn video_lines(summary: &InitializationSummary) -> Vec<String> {
    vec![
        format!("  {:<10} {}", "File:", summary.input_file),
        format!("  {:<10} {}", "Output:", summary.output_file),
        format!("  {:<10} {}", "Duration:", summary.duration),
        format!(
            "  {:<10} {} ({})",
            "Resolution:", summary.resolution, summary.category
        ),
        format!("  {:<10} {}", "Dynamic:", summary.dynamic_range),
        format!("  {:<10} {}", "Audio:", summary.audio_description),
    ]
}

fn crop_line(summary: &CropSummary) -> String {
    let status = if summary.disabled {
        "auto-crop disabled".to_string()
    } else if summary.required {
        summary
            .crop
            .clone()
            .unwrap_or_else(|| "crop params unavailable".to_string())
    } else {
        "no crop needed".to_string()
    };
    format!("  {} {} ({})", "Crop detection:", summary.message, status)
}

fn encoding_config_lines(summary: &EncodingConfigSummary) -> Vec<String> {
    let mut lines = vec![
        format!("  {:<13} {}", "Encoder:", summary.encoder),
        format!("  {:<13} {}", "Preset:", summary.preset),
        format!("  {:<13} {}", "Tune:", summary.tune),
        format!("  {:<13} {}", "Quality:", summary.quality),
        format!("  {:<13} {}", "Pixel format:", summary.pixel_format),
        format!("  {:<13} {}", "Matrix:", summary.matrix_coefficients),
        format!("  {:<13} {}", "Audio codec:", summary.audio_codec),
        format!("  {:<13} {}", "Audio:", summary.audio_description),
        format!("  {:<13} {}", "Drapto preset:", summary.drapto_preset),
    ];

    if !summary.drapto_preset_settings.is_empty() {
        let combined = summary
            .drapto_preset_settings
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("  {:<13} {}", "Preset values:", combined));
    }

    if !summary.svtav1_params.is_empty() {
        lines.push(format!(
            "  {:<13} {}",
            "SVT params:",
            summary.svtav1_params.clone()
        ));
    }

    lines
}

fn validation_lines(summary: &ValidationSummary) -> Vec<String> {
    let mut lines = Vec::new();
    if summary.passed {
        lines.push("  All checks passed".to_string());
    } else {
        lines.push("  Validation failed".to_string());
    }

    for (name, passed, details) in &summary.steps {
        let status = if *passed { "ok" } else { "failed" };
        lines.push(format!("  - {}: {} ({})", name, status, details));
    }
    lines
}

fn result_lines(summary: &EncodingOutcome) -> Vec<String> {
    vec![
        format!("  {} {}", "Output:", summary.output_file),
        format!(
            "  Size: {} -> {}",
            format_size_readable(summary.original_size),
            format_size_readable(summary.encoded_size)
        ),
        format!(
            "  Reduction: {:.1}%",
            calculate_reduction(summary.original_size, summary.encoded_size)
        ),
        format!("  {:<8} {}", "Video:", summary.video_stream),
        format!("  {:<8} {}", "Audio:", summary.audio_stream),
        format!(
            "  Time: {} (avg speed {:.1}x)",
            format_duration(&summary.total_time),
            summary.average_speed
        ),
        format!("  {} {}", "Saved to", summary.output_path.clone()),
    ]
}

fn batch_start_lines(info: &BatchStartInfo) -> Vec<String> {
    let mut lines = vec![format!(
        "  Processing {} files -> {}",
        info.total_files, info.output_dir
    )];
    for (idx, name) in info.file_list.iter().enumerate() {
        lines.push(format!("  {}. {}", idx + 1, name));
    }
    lines
}

fn batch_complete_lines(summary: &BatchSummary) -> Vec<String> {
    let mut lines = vec![
        format!(
            "  {}",
            format!(
                "{} of {} succeeded",
                summary.successful_count, summary.total_files
            )
        ),
        format!(
            "  Validation: {} passed, {} failed",
            summary.validation_passed_count, summary.validation_failed_count
        ),
        format!(
            "  Size: {} -> {} bytes ({:.1}% reduction)",
            summary.total_original_size,
            summary.total_encoded_size,
            calculate_reduction(summary.total_original_size, summary.total_encoded_size)
        ),
        format!(
            "  Time: {} (avg speed {:.1}x)",
            format_duration(&summary.total_duration),
            summary.average_speed
        ),
    ];
    for (file, reduction) in &summary.file_results {
        lines.push(format!("  - {} ({:.1}% reduction)", file, reduction));
    }
    lines
}

fn calculate_reduction(original: u64, encoded: u64) -> f64 {
    if original == 0 {
        0.0
    } else {
        ((original as f64 - encoded as f64) / original as f64 * 100.0).round()
    }
}

fn format_duration(duration: &Duration) -> String {
    let secs = duration.as_secs();
    format!(
        "{:02}:{:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

fn format_size_readable(bytes: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = MB * 1024.0;
    let mb = bytes as f64 / MB;
    let gb = bytes as f64 / GB;
    format!("{:.2} MB ({:.2} GB)", mb, gb)
}
