// ============================================================================
// drapto-cli/src/terminal.rs
// ============================================================================
//
// TERMINAL OUTPUT: Simplified UI Components and Styling
//
// This module provides a consistent terminal output styling system according
// to the Drapto CLI Design Guide. It uses a hierarchical system with minimal
// symbols and consistent spacing.
//
// KEY COMPONENTS:
// - Hierarchical output levels (Section, Subsection, Progress, Status, Detail)
// - Unified styling system with owo-colors
// - Progress bar integration with indicatif
// - Automatic color detection and NO_COLOR support
//
// AI-ASSISTANT-INFO: Simplified terminal UI components for the CLI

use console::{Term, style};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use log::{debug, error, info, warn};
use once_cell::sync::Lazy;
use owo_colors::OwoColorize;
use std::io::IsTerminal;
use std::sync::Mutex;
use std::time::Duration;
use supports_color::Stream;
use unicode_width::UnicodeWidthStr;

// Import format_bytes and format_duration_seconds from drapto_core
use drapto_core::{format_bytes, format_duration_seconds};

// ============================================================================
// OUTPUT HIERARCHY
// ============================================================================

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

// ============================================================================
// TERMINAL STATE
// ============================================================================

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

// Global terminal state
static TERMINAL_STATE: Lazy<Mutex<TerminalState>> = Lazy::new(|| Mutex::new(TerminalState::new()));

// ============================================================================
// COLOR CONTROL
// ============================================================================

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

// ============================================================================
// CORE OUTPUT FUNCTIONS
// ============================================================================

/// Print a section header for major workflow phases
pub fn print_section(title: &str) {
    let header = format!("===== {} =====", title.to_uppercase());

    // Add spacing before section
    info!("");

    // Print header with optional color
    if should_use_color() {
        info!("===== {} =====", title.to_uppercase().cyan().bold());
    } else {
        info!("{}", header);
    }

    // Add spacing after section
    info!("");
}

/// Print an item at the specified hierarchy level
pub fn print_item(level: OutputLevel, symbol: Option<&str>, text: &str, bold: bool) {
    let indent = level.indent();
    let formatted = if let Some(sym) = symbol {
        format!("{}{} {}", indent, sym, text)
    } else {
        format!("{}{}", indent, text)
    };

    // Apply formatting
    let output = if should_use_color() && bold {
        format!("{}{} {}", indent, symbol.unwrap_or(""), style(text).bold())
    } else {
        formatted
    };

    info!("{}", output);
}

/// Print a subsection or processing step
pub fn print_processing(message: &str) {
    // Add spacing before processing steps
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
        info!("  ✓ {}", message);
    }
}

/// Print a status line (key-value pair)
pub fn print_status(label: &str, value: &str, highlight: bool) {
    // Calculate padding for alignment
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

    // Apply value formatting based on content
    if should_use_color() {
        let colored_value = match () {
            _ if label.contains("Speed") && value.ends_with('x') => {
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
            _ if label.contains("Acceleration") && value.contains("None available") => {
                value.yellow().to_string()
            }
            _ if highlight => value.bold().to_string(),
            _ => value.to_string(),
        };

        info!(
            "{}{}:{} {}",
            OutputLevel::Status.indent(),
            label,
            " ".repeat(padding),
            colored_value
        );
    } else {
        info!("{}", formatted);
    }
}

/// Print completion with associated status
pub fn print_completion_with_status(success_message: &str, status_label: &str, status_value: &str) {
    print_success(success_message);

    // Special handling for grain detection results
    if should_use_color() && status_label.contains("grain") && status_value.contains(" - applying")
    {
        if let Some(dash_pos) = status_value.find(" - applying") {
            let grain_level = &status_value[..dash_pos];
            let rest = &status_value[dash_pos..];
            let colored_value = format!("{}{}", grain_level.green(), rest);
            print_status(status_label, &colored_value, false);
        } else {
            print_status(status_label, status_value, false);
        }
    } else {
        print_status(status_label, status_value, false);
    }
}

/// Print an error message
pub fn print_error(title: &str, message: &str, suggestion: Option<&str>) {
    if should_use_color() {
        info!("✗ {}", title.red().bold());
    } else {
        info!("✗ {}", title);
    }

    info!("");
    info!("  Message:  {}", message);

    if let Some(suggestion_text) = suggestion {
        info!("");
        info!("  Suggestion: {}", suggestion_text);
    }

    info!("");
}

/// Print a warning message
pub fn print_warning(message: &str) {
    if should_use_color() {
        info!("  ⚠ {}", message.yellow());
    } else {
        info!("  ⚠ {}", message);
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
        &format!("Progress: {}", message),
        false,
    );
}

/// Print section separator (empty line)
pub fn print_section_separator() {
    info!("");
}

// ============================================================================
// PROGRESS BAR MANAGEMENT
// ============================================================================

/// Initialize a progress bar with indicatif
fn init_progress_bar(total_secs: f64) -> ProgressBar {
    let pb = ProgressBar::new((total_secs * 1000.0) as u64);

    // Get terminal width
    let term = Term::stderr();
    let term_width = term.size().1 as usize;

    // Create adaptive style
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

    // Only show in interactive terminals
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

        // Initialize progress bar if needed
        if state.current_progress.is_none() {
            state.current_progress = Some(init_progress_bar(total_secs));
        }

        if let Some(pb) = state.current_progress.as_ref() {
            pb.set_position((elapsed_secs * 1000.0) as u64);

            // Update message with speed/fps if available
            let term_width = Term::stderr().size().1 as usize;
            if let (Some(speed_val), Some(fps_val)) = (speed, fps) {
                if term_width >= 100 {
                    let msg = format!(
                        "Encoding - Speed: {:.2}x, Avg FPS: {:.2}",
                        speed_val, fps_val
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

// ============================================================================
// CLI PROGRESS REPORTER IMPLEMENTATION
// ============================================================================

/// Implementation of the ProgressReporter trait for the CLI interface
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
                // Check if this is the first encoding message
                let is_encoding_start = text.starts_with("Encoding:");
                if is_encoding_start {
                    let mut state = TERMINAL_STATE.lock().unwrap();
                    if !state.encoding_section_shown {
                        state.encoding_section_shown = true;
                        drop(state);
                        print_section("ENCODING PROGRESS");
                        print_processing_no_spacing(text);
                    } else {
                        drop(state);
                        print_processing(text);
                    }
                } else {
                    print_processing(text);
                }
            }
            CoreLevel::Success => print_success(text),
            CoreLevel::Error => print_error(text, "", None),
            CoreLevel::Warning => print_warning(text),
            CoreLevel::Debug => debug!("{}", text),
            CoreLevel::Info => print_sub_item(text),
            _ => info!("{}", text),
        }
    }

    fn output_status(&self, label: &str, value: &str, highlight: bool) {
        print_status(label, value, highlight);
    }

    fn progress_bar(&self, percent: f32, elapsed_secs: f64, total_secs: f64) {
        // For now, we'll use default values for speed/fps/eta
        // These can be calculated from the progress data if needed
        print_progress_bar(percent, elapsed_secs, total_secs, None, None, None);
    }

    fn clear_progress_bar(&self) {
        clear_progress_bar();
    }

    fn log(&self, level: drapto_core::progress_reporting::LogLevel, message: &str) {
        match level {
            drapto_core::progress_reporting::LogLevel::Info => info!("{}", message),
            drapto_core::progress_reporting::LogLevel::Warning => warn!("{}", message),
            drapto_core::progress_reporting::LogLevel::Error => error!("{}", message),
            drapto_core::progress_reporting::LogLevel::Debug => debug!("{}", message),
        }
    }

    fn ffmpeg_command(&self, cmd_data: &str, is_sample: bool) {
        if is_sample {
            debug!("FFmpeg command (grain sample): {}", cmd_data);
            return;
        }

        // Simple FFmpeg command display
        debug!("    FFmpeg command:");

        // Try to parse as JSON array first
        if let Ok(args) = serde_json::from_str::<Vec<String>>(cmd_data) {
            debug!("      {}", format_ffmpeg_simple(&args));
        } else {
            // Fallback to showing raw command
            debug!("      {}", cmd_data);
        }
    }
}

/// Simple FFmpeg command formatter
fn format_ffmpeg_simple(args: &[String]) -> String {
    if args.is_empty() {
        return String::new();
    }

    let mut lines = vec![args[0].clone()];
    let mut current_line = String::new();

    for arg in args.iter().skip(1) {
        // Start new line for major argument groups
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
            lines.push(format!("  {}", current_line));
            current_line.clear();
        }

        if !current_line.is_empty() {
            current_line.push(' ');
        }

        // Quote arguments that need it
        if arg.contains(' ') || arg.contains('=') {
            current_line.push_str(&format!("\"{}\"", arg));
        } else {
            current_line.push_str(arg);
        }
    }

    if !current_line.is_empty() {
        lines.push(format!("  {}", current_line));
    }

    lines.join("\n      ")
}

// ============================================================================
// ADDITIONAL OUTPUT FUNCTIONS
// ============================================================================

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
    info!("{}", filename);
    info!(
        "  {:<13} {}",
        "Encode time:",
        format_duration_seconds(duration.as_secs_f64())
    );
    info!("  {:<13} {}", "Input size:", format_bytes(input_size));
    info!("  {:<13} {}", "Output size:", format_bytes(output_size));

    let reduction_str = format!("{}%", reduction);
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

    info!("{}", header);
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

/// Data structure for grain analysis table
pub struct GrainAnalysisRow {
    pub sample: String,
    pub time: String,
    pub size_mb: String,
    pub quality: String,
    pub selection: String,
}

/// Print grain analysis results
pub fn print_grain_analysis_table(results: &[GrainAnalysisRow]) {
    if results.is_empty() {
        return;
    }

    info!("    Sample  Time      Size (MB)  Quality  Selection");
    info!("    ------  --------  ---------  -------  ---------");

    for result in results {
        info!(
            "    {:<6}  {:<8}  {:<9}  {:<7}  {}",
            result.sample, result.time, result.size_mb, result.quality, result.selection
        );
    }
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
