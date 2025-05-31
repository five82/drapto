//! Terminal UI components and styling.
//!
//! This module provides consistent terminal output styling according to the
//! Drapto CLI Design Guide, using a hierarchical system with minimal symbols
//! and consistent spacing.

use console::{Term, style};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use log::{debug, error, info, warn};
use std::sync::LazyLock;
use owo_colors::OwoColorize;
use std::io::IsTerminal;
use std::sync::Mutex;
use std::time::Duration;
use supports_color::Stream;
use unicode_width::UnicodeWidthStr;

use drapto_core::{format_bytes, format_duration_seconds};

/// Represents the visual hierarchy levels in the CLI output
#[derive(Debug, Clone, Copy)]
pub enum OutputLevel {
    /// Level 1: Main sections (===== SECTION =====)
    Section,
    /// Level 2: Subsections and major operations (» Operation)
    Subsection,
    /// Level 3: Progress items and sub-operations
    Progress,
    /// Level 4: Key-value status information
    Status,
    /// Level 5: Additional details and metrics
    Detail,
}

impl OutputLevel {
    /// Get the indentation for this output level
    fn indent(&self) -> &'static str {
        match self {
            OutputLevel::Section => "",
            OutputLevel::Subsection => "  ",
            OutputLevel::Progress => "    ",
            OutputLevel::Status => "      ",
            OutputLevel::Detail => "        ",
        }
    }
}

/// Terminal state management
struct TerminalState {
    /// Whether we've printed the encoding section
    encoding_section_shown: bool,
    /// Current progress bar if any
    current_progress: Option<ProgressBar>,
    /// Whether color output is enabled
    use_color: bool,
}

impl TerminalState {
    fn new() -> Self {
        // Detect color support using multiple methods
        let use_color = if std::env::var("NO_COLOR").is_ok() || !std::io::stderr().is_terminal() {
            false
        } else {
            supports_color::on(Stream::Stderr).is_some()
        };

        Self {
            encoding_section_shown: false,
            current_progress: None,
            use_color,
        }
    }
}

static TERMINAL_STATE: LazyLock<Mutex<TerminalState>> = LazyLock::new(|| Mutex::new(TerminalState::new()));

/// Set whether to use color in terminal output
pub fn set_color(enable: bool) {
    if let Ok(mut state) = TERMINAL_STATE.lock() {
        state.use_color = enable;
    }
}

/// Check if color should be used
fn should_use_color() -> bool {
    TERMINAL_STATE
        .lock()
        .map(|state| state.use_color)
        .unwrap_or(false)
}

/// Print a section header for major workflow phases
pub fn print_section(title: &str) {
    let header = format!("===== {} =====", title.to_uppercase());

    info!("");
    if should_use_color() {
        info!("===== {} =====", title.to_uppercase().cyan().bold());
    } else {
        info!("{header}");
    }
    info!("");
}

/// Print an item at the specified hierarchy level
pub fn print_item(level: OutputLevel, symbol: Option<&str>, text: &str, bold: bool) {
    let indent = level.indent();
    let formatted = if let Some(sym) = symbol {
        format!("{indent}{sym} {text}")
    } else {
        format!("{indent}{text}")
    };

    let output = if should_use_color() && bold {
        format!("{}{} {}", indent, symbol.unwrap_or(""), style(text).bold())
    } else {
        formatted
    };

    info!("{output}");
}

/// Print a subsection or processing step
pub fn print_processing(message: &str) {
    info!("");
    print_item(OutputLevel::Subsection, Some("»"), message, true);
}

/// Print a subsection without preceding blank line
pub fn print_processing_no_spacing(message: &str) {
    print_item(OutputLevel::Subsection, Some("»"), message, true);
}

/// Print a subsection header
pub fn print_subsection(title: &str) {
    print_item(OutputLevel::Subsection, None, title, true);
}

/// Print a success message
pub fn print_success(message: &str) {
    info!("");
    if should_use_color() {
        info!("  ✓ {}", message.green());
    } else {
        info!("  ✓ {message}");
    }
}

/// Print a status line (key-value pair)
pub fn print_status(label: &str, value: &str, highlight: bool) {
    let label_width = 15;
    let padding = if label.width() < label_width {
        label_width - label.width()
    } else {
        1
    };

    let formatted = format!(
        "{}{}:{} {}",
        OutputLevel::Status.indent(),
        label,
        " ".repeat(padding),
        value
    );

    if should_use_color() {
        let colored_value = match () {
            () if label.contains("Speed") && value.ends_with('x') => {
                if let Some(speed_str) = value.strip_suffix('x') {
                    if let Ok(speed) = speed_str.trim().parse::<f32>() {
                        if speed >= 2.0 {
                            value.green().to_string()
                        } else if speed < 1.0 {
                            value.yellow().to_string()
                        } else {
                            value.to_string()
                        }
                    } else {
                        value.to_string()
                    }
                } else {
                    value.to_string()
                }
            }
            () if label.contains("Acceleration") && value.contains("None available") => {
                value.yellow().to_string()
            }
            () if highlight => value.bold().to_string(),
            () => value.to_string(),
        };

        info!(
            "{}{}:{} {}",
            OutputLevel::Status.indent(),
            label,
            " ".repeat(padding),
            colored_value
        );
    } else {
        info!("{formatted}");
    }
}

/// Print completion with associated status
pub fn print_completion_with_status(success_message: &str, status_label: &str, status_value: &str) {
    print_success(success_message);
    print_status(status_label, status_value, false);
}

/// Print an error message
pub fn print_error(title: &str, message: &str, suggestion: Option<&str>) {
    if should_use_color() {
        info!("✗ {}", title.red().bold());
    } else {
        info!("✗ {title}");
    }

    info!("");
    info!("  Message:  {message}");

    if let Some(suggestion_text) = suggestion {
        info!("");
        info!("  Suggestion: {suggestion_text}");
    }

    info!("");
}

/// Print a warning message
pub fn print_warning(message: &str) {
    if should_use_color() {
        info!("  ⚠ {}", message.yellow());
    } else {
        info!("  ⚠ {message}");
    }
}

/// Print a sub-item under a processing step
pub fn print_sub_item(message: &str) {
    print_item(OutputLevel::Progress, None, message, false);
}

/// Print a progress indicator
pub fn print_progress_indicator(message: &str) {
    print_item(
        OutputLevel::Progress,
        None,
        &format!("Progress: {message}"),
        false,
    );
}

/// Print section separator (empty line)
pub fn print_section_separator() {
    info!("");
}

/// Initialize a progress bar with indicatif
fn init_progress_bar(total_secs: f64) -> ProgressBar {
    let pb = ProgressBar::new((total_secs * 1000.0) as u64);

    let term = Term::stderr();
    let term_width = term.size().1 as usize;
    let style = if term_width >= 100 {
        ProgressStyle::default_bar()
            .template("Encoding: {percent:>5.1}% [{bar:30}] ({elapsed_precise} / {eta_precise})")
            .unwrap()
            .progress_chars("##.")
    } else if term_width >= 60 {
        ProgressStyle::default_bar()
            .template("Encoding: {percent:>5.1}% [{bar:20}]\n  {eta_precise}")
            .unwrap()
            .progress_chars("##.")
    } else {
        ProgressStyle::default_bar()
            .template("{percent:>5.1}% [{bar:10}]")
            .unwrap()
            .progress_chars("##.")
    };

    pb.set_style(style);
    pb.set_message("Encoding");

    if !std::io::stderr().is_terminal() {
        pb.set_draw_target(ProgressDrawTarget::hidden());
    }

    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Print a progress bar
pub fn print_progress_bar(
    _percent: f32,
    elapsed_secs: f64,
    total_secs: f64,
    speed: Option<f32>,
    fps: Option<f32>,
    _eta: Option<Duration>,
) {
    if std::io::stderr().is_terminal() {
        let mut state = TERMINAL_STATE.lock().unwrap();

        if state.current_progress.is_none() {
            state.current_progress = Some(init_progress_bar(total_secs));
        }

        if let Some(pb) = state.current_progress.as_ref() {
            pb.set_position((elapsed_secs * 1000.0) as u64);

            let term_width = Term::stderr().size().1 as usize;
            if let (Some(speed_val), Some(fps_val)) = (speed, fps) {
                if term_width >= 100 {
                    let msg = format!(
                        "Encoding - Speed: {speed_val:.2}x, Avg FPS: {fps_val:.2}"
                    );
                    pb.set_message(msg);
                }
            }

            pb.tick();
        }
    }
}

/// Clear the current progress bar
pub fn clear_progress_bar() {
    if let Ok(mut state) = TERMINAL_STATE.lock() {
        if let Some(pb) = state.current_progress.take() {
            pb.finish_and_clear();
        }
    }
}

/// Implementation of the `ProgressReporter` trait for the CLI interface
pub struct CliProgressReporter;

/// Register the CLI progress reporter with the core library
pub fn register_cli_reporter() {
    let reporter = Box::new(CliProgressReporter);
    drapto_core::progress_reporting::set_progress_reporter(reporter);
}

impl drapto_core::progress_reporting::ProgressReporter for CliProgressReporter {
    fn output(&self, level: drapto_core::progress_reporting::OutputLevel, text: &str) {
        use drapto_core::progress_reporting::OutputLevel as CoreLevel;

        match level {
            CoreLevel::Section => print_section(text),
            CoreLevel::Subsection => print_subsection(text),
            CoreLevel::Processing => {
                let is_encoding_start = text.starts_with("Encoding:");
                if is_encoding_start {
                    let mut state = TERMINAL_STATE.lock().unwrap();
                    if state.encoding_section_shown {
                        drop(state);
                        print_processing(text);
                    } else {
                        state.encoding_section_shown = true;
                        drop(state);
                        print_section("ENCODING PROGRESS");
                        print_processing_no_spacing(text);
                    }
                } else {
                    print_processing(text);
                }
            }
            CoreLevel::Success => print_success(text),
            CoreLevel::Error => print_error(text, "", None),
            CoreLevel::Warning => print_warning(text),
            CoreLevel::Debug => debug!("{text}"),
            CoreLevel::Info => print_sub_item(text),
            _ => info!("{text}"),
        }
    }

    fn output_status(&self, label: &str, value: &str, highlight: bool) {
        print_status(label, value, highlight);
    }

    fn progress_bar(&self, percent: f32, elapsed_secs: f64, total_secs: f64) {
        print_progress_bar(percent, elapsed_secs, total_secs, None, None, None);
    }

    fn clear_progress_bar(&self) {
        clear_progress_bar();
    }

    fn log(&self, level: drapto_core::progress_reporting::LogLevel, message: &str) {
        match level {
            drapto_core::progress_reporting::LogLevel::Info => info!("{message}"),
            drapto_core::progress_reporting::LogLevel::Warning => warn!("{message}"),
            drapto_core::progress_reporting::LogLevel::Error => error!("{message}"),
            drapto_core::progress_reporting::LogLevel::Debug => debug!("{message}"),
        }
    }

    fn ffmpeg_command(&self, cmd_data: &str) {
        debug!("    FFmpeg command:");
        if let Ok(args) = serde_json::from_str::<Vec<String>>(cmd_data) {
            debug!("      {}", format_ffmpeg_simple(&args));
        } else {
            debug!("      {cmd_data}");
        }
    }
}

/// Simple `FFmpeg` command formatter
fn format_ffmpeg_simple(args: &[String]) -> String {
    if args.is_empty() {
        return String::new();
    }

    let mut lines = vec![args[0].clone()];
    let mut current_line = String::new();

    for arg in args.iter().skip(1) {
        let needs_new_line = matches!(
            arg.as_str(),
            "-i" | "-hwaccel"
                | "-vf"
                | "-af"
                | "-filter_complex"
                | "-c:v"
                | "-c:a"
                | "-preset"
                | "-crf"
                | "-svtav1-params"
                | "-map"
                | "-y"
        );

        if needs_new_line && !current_line.is_empty() {
            lines.push(format!("  {current_line}"));
            current_line.clear();
        }

        if !current_line.is_empty() {
            current_line.push(' ');
        }

        if arg.contains(' ') || arg.contains('=') {
            current_line.push_str(&format!("\"{arg}\""));
        } else {
            current_line.push_str(arg);
        }
    }

    if !current_line.is_empty() {
        lines.push(format!("  {current_line}"));
    }

    lines.join("\n      ")
}

/// Print encoding summary
pub fn print_encoding_summary(
    filename: &str,
    duration: std::time::Duration,
    input_size: u64,
    output_size: u64,
) {
    clear_progress_bar();

    let reduction = if input_size > 0 {
        100 - ((output_size * 100) / input_size)
    } else {
        0
    };

    info!("");
    info!("{filename}");
    info!(
        "  {:<13} {}",
        "Encode time:",
        format_duration_seconds(duration.as_secs_f64())
    );
    info!("  {:<13} {}", "Input size:", format_bytes(input_size));
    info!("  {:<13} {}", "Output size:", format_bytes(output_size));

    let reduction_str = format!("{reduction}%");
    if should_use_color() && reduction >= 50 {
        info!("  {:<13} {}", "Reduced by:", reduction_str.green());
    } else {
        info!("  {:<13} {}", "Reduced by:", reduction_str);
    }

    info!("");
}

/// Print file list
pub fn print_file_list(header: &str, files: &[std::path::PathBuf]) {
    if files.is_empty() {
        info!("No files found to process.");
        return;
    }

    info!("{header}");
    for file in files {
        info!("  - {}", file.display());
    }
}

/// Print daemon file list (pre-daemonization)
pub fn print_daemon_file_list(files: &[std::path::PathBuf]) {
    if files.is_empty() {
        eprintln!("No .mkv files found to encode in the specified input.");
        return;
    }

    eprintln!("Will encode the following files:");
    for file in files {
        eprintln!("  - {}", file.display());
    }
}

/// Print daemon log info
pub fn print_daemon_log_info(log_path: &std::path::Path) {
    eprintln!("Log file: {}", log_path.display());
}

/// Print daemon starting message
pub fn print_daemon_starting() {
    eprintln!("Starting Drapto daemon in the background...");
}


/// Data structure for encoding summary table
pub struct EncodingSummaryRow {
    pub file: String,
    pub input: String,
    pub output: String,
    pub reduction: String,
    pub time: String,
}

/// Print encoding summary table
pub fn print_encoding_summary_table(summaries: &[EncodingSummaryRow]) {
    if summaries.is_empty() {
        return;
    }

    print_section("ENCODING SUMMARY");

    for summary in summaries {
        info!(
            "      {}: {} → {} ({}) in {}",
            summary.file, summary.input, summary.output, summary.reduction, summary.time
        );
    }
}
