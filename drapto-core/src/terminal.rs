//! Terminal UI components and styling for drapto.
//!
//! This module provides consistent terminal output styling using a hierarchical
//! system with minimal symbols and consistent spacing. It consolidates functionality
//! from both CLI and core components.

use console::{Term, style};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use log::info;
use std::sync::LazyLock;
use owo_colors::OwoColorize;
use std::io::IsTerminal;
use std::sync::Mutex;
use std::time::Duration;
use unicode_width::UnicodeWidthStr;

use crate::{format_bytes, format_duration};

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
    /// Current progress bar if any
    current_progress: Option<ProgressBar>,
}

impl TerminalState {
    fn new() -> Self {
        Self {
            current_progress: None,
        }
    }
}

static TERMINAL_STATE: LazyLock<Mutex<TerminalState>> = LazyLock::new(|| Mutex::new(TerminalState::new()));

/// Check if color should be used (respects NO_COLOR environment variable)
fn should_use_color() -> bool {
    std::env::var("NO_COLOR").is_err()
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

/// Print a subsection header at Level 3 (4 spaces indentation - for use within sections)
pub fn print_subsection_level3(title: &str) {
    if should_use_color() {
        info!("    {}", style(title).bold());
    } else {
        info!("    {}", title);
    }
}

/// Print a subsection header at Level 3 with preceding blank line
pub fn print_subsection_level3_with_spacing(title: &str) {
    info!("");
    if should_use_color() {
        info!("    {}", style(title).bold());
    } else {
        info!("    {}", title);
    }
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

/// Print a sub-item with preceding blank line (Level 3 - Operations with spacing)
pub fn print_sub_item_with_spacing(message: &str) {
    info!("");
    info!("    {}", message);
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
        format_duration(duration.as_secs_f64())
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