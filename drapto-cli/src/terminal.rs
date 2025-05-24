// ============================================================================
// drapto-cli/src/terminal.rs
// ============================================================================
//
// TERMINAL OUTPUT: UI Components and Styling
//
// This module provides a consistent terminal output styling system according
// to the Drapto CLI Design Guide. It includes functions for sections, status
// lines, progress bars, and other UI components with consistent styling.
//
// KEY COMPONENTS:
// - styling: Constants for colors, symbols, and formatting
// - UI component functions: print_section, print_status, print_progress, etc.
// - Verbosity control: Functions to control output verbosity
//
// ARCHITECTURAL DESIGN:
// This module is the primary UI formatting layer for the CLI. It defines
// the visual language and styling that will be used throughout the application.
// The core library uses a parallel API in progress_reporting.rs that follows
// the same styling conventions but remains independent of CLI-specific code.
//
// When the CLI sets verbosity levels or other output-related settings, these
// are propagated to the core library to maintain consistent styling across
// all output, regardless of which layer generates it.
//
// DESIGN PHILOSOPHY:
// The terminal module follows the "Human-first design" principle from the design
// guide, with consistent visual hierarchy, strategic color use, and clear typography.
//
// AI-ASSISTANT-INFO: Terminal UI components and styling for the CLI

// ---- External crate imports ----
use colored::*;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use log::{debug, error, info, warn};
use once_cell::sync::Lazy;
use serde_json;
use std::io::{self, IsTerminal};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tabled::Table;

// Import format_bytes from drapto_core
use drapto_core::format_bytes;

// ============================================================================
// STYLING CONSTANTS
// ============================================================================

/// Styling constants for terminal output
pub mod styling {
    // Symbols (monochrome as per design guide)
    pub const SUCCESS_SYMBOL: &str = "✓";
    pub const PROGRESS_SYMBOL: &str = "⧖";
    pub const PROCESSING_SYMBOL: &str = "»";
    pub const PHASE_SYMBOL: &str = "◎";
    pub const SAMPLE_SYMBOL: &str = "◆";
    pub const ERROR_SYMBOL: &str = "✗";
    pub const WARNING_SYMBOL: &str = "⚠";

    // Section formatting
    pub const SECTION_PREFIX: &str = "===== ";
    pub const SECTION_SUFFIX: &str = " =====";

    // Indentation
    pub const STATUS_INDENT: &str = "    "; // Level 4: 4 spaces for status lines
    pub const SUBSECTION_INDENT: &str = "  "; // Level 2: 2 spaces for subsections
    pub const SUB_ITEM_INDENT: &str = "    "; // Level 3-4: 4 spaces for sub-items

    // Progress bar
    pub const PROGRESS_FILL: &str = "#";
    pub const PROGRESS_EMPTY: &str = ".";

    // Empty line for vertical spacing
    pub const EMPTY_LINE: &str = "";

    // Vertical spacing - adjusted to match design guide
    pub const LINE_SPACING_BEFORE_SECTION: usize = 1; // Single line break before sections
    pub const LINE_SPACING_AFTER_SECTION: usize = 1; // Single line after section header
    pub const LINE_SPACING_BEFORE_PROCESSING: usize = 1; // Single line break between subsections
    pub const LINE_SPACING_AFTER_SUCCESS: usize = 0; // No extra lines after success message
    pub const LINE_SPACING_BETWEEN_SECTIONS: usize = 1; // Single line break between sections
}

// ============================================================================
// VERBOSITY CONTROL
// ============================================================================

// Verbosity is now handled by standard log levels (info, debug, trace)
// Use log::info! for normal output, log::debug! for verbose output

// Global color setting
static USE_COLOR: AtomicBool = AtomicBool::new(true);

// Track if we've printed the encoding section
static ENCODING_SECTION_PRINTED: AtomicBool = AtomicBool::new(false);

// Track if we've printed the first encoder message for spacing
static FIRST_ENCODER_MESSAGE_PRINTED: AtomicBool = AtomicBool::new(false);

/// Set whether to use color in terminal output
pub fn set_color(enable: bool) {
    USE_COLOR.store(enable, Ordering::Relaxed);
}

/// Check if color should be used
pub fn should_use_color() -> bool {
    USE_COLOR.load(Ordering::Relaxed)
}

// Output control is now handled by standard log levels
// Functions that were using should_print(VerbosityLevel::Normal) now use info!
// Functions that were using should_print(VerbosityLevel::Verbose) now use debug!

// ============================================================================
// TERMINAL COMPONENTS
// ============================================================================
//
// Terminal UI components follow a visual hierarchy:
//
// 1. Sections (===== Section =====)
//    - Used for major workflow phase transitions
//
// 2. Processing steps (» Step description)
//    - Used for major steps within a section
//
// 3. Sub-items (  Description...)
//    - Used for details under a processing step
//
// 4. Status items (  Label:     Value)
//    - Used for key-value information
//
// 5. Success messages (✓ Success message)
//    - Used to indicate completion of steps
//
// NOTE: All spacing between components is automatically managed by these
// functions. Other modules should NEVER manually add empty lines or spaces.
// If spacing needs to be adjusted, it should be changed here.

/// Print a section header for major workflow phases
///
/// # Arguments
///
/// * `title` - The title of the section
pub fn print_section(title: &str) {
    // Always format the plain header for logging
    let header_plain = format!(
        "{}{}{}",
        styling::SECTION_PREFIX,
        title.to_uppercase(),
        styling::SECTION_SUFFIX
    );
    
    // Add consistent empty lines before section for vertical spacing
    for _ in 0..styling::LINE_SPACING_BEFORE_SECTION {
        info!("{}", styling::EMPTY_LINE);
    }

    // Format section header with uppercase title and cyan color for the title only
    let header = if should_use_color() {
        format!(
            "{}{}{}",
            styling::SECTION_PREFIX,
            title.to_uppercase().cyan().bold(),
            styling::SECTION_SUFFIX
        )
    } else {
        header_plain.clone()
    };

    // Log the section header using the info! macro
    info!("{}", header);

    // Add consistent empty lines after section header for spacing
    for _ in 0..styling::LINE_SPACING_AFTER_SECTION {
        info!("{}", styling::EMPTY_LINE);
    }
}

/// Print a status line (key-value pair)
///
/// # Arguments
///
/// * `label` - The label for the status line
/// * `value` - The value to display
/// * `highlight` - Whether to emphasize the value
pub fn print_status(label: &str, value: &str, highlight: bool) {
    // Format for logging
    let padding = if label.len() < 15 {
        15 - label.len()
    } else {
        1
    };
    let formatted_label = format!("{}{}{}", label, ":", " ".repeat(padding));
    let _plain_text = format!("{}{} {}", styling::STATUS_INDENT, formatted_label, value);
    
    // Format the value with appropriate styling based on content and context
    let formatted_value = if should_use_color() {
        // Check for performance metrics (Speed)
        if label.contains("Speed") || label.contains("speed") {
            if let Some(speed_str) = value.strip_suffix('x') {
                if let Ok(speed) = speed_str.trim().parse::<f32>() {
                    if speed >= 2.0 {
                        // Good performance - green
                        value.green().to_string()
                    } else if speed < 1.0 {
                        // Poor performance - yellow
                        value.yellow().to_string()
                    } else {
                        // Acceptable performance - default
                        value.to_string()
                    }
                } else {
                    value.to_string()
                }
            } else {
                value.to_string()
            }
        } 
        // Check for hardware acceleration status
        else if label.contains("Acceleration") && value.contains("None available") {
            // No hardware acceleration - yellow warning
            value.yellow().to_string()
        }
        else if highlight {
            // Use bold for highlighted/important values
            value.bold().to_string()
        } else {
            value.to_string()
        }
    } else {
        value.to_string()
    };

    // Log the status line using the info! macro
    info!(
        "{}{} {}",
        styling::STATUS_INDENT,
        formatted_label,
        formatted_value
    );
}

/// Print a success message
///
/// # Arguments
///
/// * `message` - The success message to display
pub fn print_success(message: &str) {
    // Success symbol should not be colored - add proper indentation for Level 2
    // Color the success message text in green according to design guide
    let formatted_message = if should_use_color() {
        message.green().to_string()
    } else {
        message.to_string()
    };
    
    info!(
        "{}{} {}",
        styling::SUBSECTION_INDENT,
        styling::SUCCESS_SYMBOL,
        formatted_message
    );

    // No spacing after success messages per design guide (LINE_SPACING_AFTER_SUCCESS = 0)
}

/// Print a completion message with associated status
/// This function combines a success message with a status line
/// in a consistent format with proper spacing
///
/// # Arguments
///
/// * `success_message` - The main completion/success message  
/// * `status_label` - The label for the status line
/// * `status_value` - The value for the status line
pub fn print_completion_with_status(success_message: &str, status_label: &str, status_value: &str) {
    // Print the success message (without extra spacing)
    // Add a blank line before the success message for proper spacing between subsections
    info!("{}", styling::EMPTY_LINE);

    // Success symbol should not be colored, add proper indentation for Level 2
    // Color the success message text in green according to design guide
    let formatted_success = if should_use_color() {
        success_message.green().to_string()
    } else {
        success_message.to_string()
    };
    
    info!(
        "{}{} {}",
        styling::SUBSECTION_INDENT,
        styling::SUCCESS_SYMBOL,
        formatted_success
    );

    // Print the associated status line with Level 4 indentation (4 spaces)
    // Format it directly here instead of using print_status which only has 2-space indent
    let padding = if status_label.len() < 15 {
        15 - status_label.len()
    } else {
        1
    };

    let formatted_label = format!("{}{}{}", status_label, ":", " ".repeat(padding));

    // For grain detection results, color the selected grain level in green
    let formatted_value = if should_use_color() && status_label.contains("grain") && status_value.contains(" - applying") {
        // Extract the grain level part before " - applying"
        if let Some(dash_pos) = status_value.find(" - applying") {
            let grain_level = &status_value[..dash_pos];
            let rest = &status_value[dash_pos..];
            format!("{}{}", grain_level.green(), rest)
        } else {
            status_value.to_string()
        }
    } else {
        status_value.to_string()
    };

    // Use SUB_ITEM_INDENT (4 spaces) for proper Level 4 hierarchy
    info!(
        "{}{} {}",
        styling::SUB_ITEM_INDENT,
        formatted_label,
        formatted_value
    );
}

/// Print a processing step message
///
/// # Arguments
///
/// * `message` - The processing step message to display
pub fn print_processing(message: &str) {
    print_processing_internal(message, true);
}

/// Print a processing step message without preceding blank line
/// Used for the first processing step after a section header
///
/// # Arguments
///
/// * `message` - The processing step message to display
pub fn print_processing_no_spacing(message: &str) {
    print_processing_internal(message, false);
}

/// Internal function for printing processing steps
fn print_processing_internal(message: &str, add_spacing: bool) {
    // Add spacing before processing steps for visual grouping (if requested)
    if add_spacing {
        for _ in 0..styling::LINE_SPACING_BEFORE_PROCESSING {
            info!("{}", styling::EMPTY_LINE);
        }
    }

    // Processing symbol should not be colored; make message bold for Level 2 hierarchy
    if should_use_color() {
        info!(
            "{}{} {}",
            styling::SUBSECTION_INDENT,
            styling::PROCESSING_SYMBOL,
            message.bold()
        );
    } else {
        info!(
            "{}{} {}",
            styling::SUBSECTION_INDENT,
            styling::PROCESSING_SYMBOL,
            message
        );
    }
}

/// Print an error message with context
///
/// # Arguments
///
/// * `title` - The error title
/// * `message` - The error message
/// * `suggestion` - Optional suggestion for fixing the error
pub fn print_error(title: &str, message: &str, suggestion: Option<&str>) {
    // Always print errors regardless of verbosity
    if should_use_color() {
        // Only the title text should be red, not the symbol
        info!("{} {}", styling::ERROR_SYMBOL, title.red().bold());
    } else {
        info!("{} {}", styling::ERROR_SYMBOL, title);
    }

    info!("{}", styling::EMPTY_LINE);
    info!("  Message:  {}", message);

    if let Some(suggestion_text) = suggestion {
        info!("{}", styling::EMPTY_LINE);
        info!("  Suggestion: {}", suggestion_text);
    }

    // Add empty line after error
    info!("{}", styling::EMPTY_LINE);
}

/// Print a warning message
///
/// # Arguments
///
/// * `message` - The warning message to display
pub fn print_warning(message: &str) {
    // Warning symbol should not be colored; make message yellow for Level 2 hierarchy
    if should_use_color() {
        info!(
            "{}{} {}",
            styling::SUBSECTION_INDENT,
            styling::WARNING_SYMBOL,
            message.yellow()
        );
    } else {
        info!(
            "{}{} {}",
            styling::SUBSECTION_INDENT,
            styling::WARNING_SYMBOL,
            message
        );
    }
}

// Global progress bar instance
static CURRENT_PROGRESS_BAR: Lazy<Mutex<Option<ProgressBar>>> = Lazy::new(|| Mutex::new(None));

/// Initialize a progress bar with indicatif
fn init_progress_bar(total_secs: f64) -> ProgressBar {
    // Convert to milliseconds for smoother updates
    let pb = ProgressBar::new((total_secs * 1000.0) as u64);

    // Get terminal width for adaptive styling
    let term_width = term_size::dimensions().map(|(w, _)| w).unwrap_or(80);

    // Create style based on terminal width
    let style = if term_width >= 100 {
        // Wide terminal - full details on one line
        ProgressStyle::default_bar()
            .template(
                "{prefix} {msg}: {percent:>5.1}% [{bar:30}] ({elapsed_precise} / {eta_precise})",
            )
            .unwrap()
            .progress_chars("##.")
    } else if term_width >= 60 {
        // Medium terminal - compact format
        ProgressStyle::default_bar()
            .template("{prefix} {msg}: {percent:>5.1}% [{bar:20}]\n  {eta_precise}")
            .unwrap()
            .progress_chars("##.")
    } else {
        // Narrow terminal - minimal format
        ProgressStyle::default_bar()
            .template("{prefix} {percent:>5.1}% [{bar:10}]")
            .unwrap()
            .progress_chars("##.")
    };

    pb.set_style(style);
    pb.set_prefix(styling::PROGRESS_SYMBOL);
    pb.set_message("Encoding");

    // Only show in interactive terminals
    if !io::stderr().is_terminal() {
        pb.set_draw_target(ProgressDrawTarget::hidden());
    }

    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Print a progress bar using indicatif
///
/// # Arguments
///
/// * `percent` - Progress percentage (0.0 to 100.0)
/// * `elapsed_secs` - Elapsed time in seconds
/// * `total_secs` - Total duration in seconds
/// * `speed` - Optional encoding speed multiplier
/// * `fps` - Optional frames per second
/// * `eta` - Optional estimated time remaining
pub fn print_progress_bar(
    _percent: f32,
    elapsed_secs: f64,
    total_secs: f64,
    speed: Option<f32>,
    fps: Option<f32>,
    _eta: Option<Duration>,
) {
    if io::stderr().is_terminal() {
        let mut pb_guard = CURRENT_PROGRESS_BAR.lock().unwrap();

        // Initialize progress bar if needed
        if pb_guard.is_none() {
            *pb_guard = Some(init_progress_bar(total_secs));
        }

        if let Some(pb) = pb_guard.as_ref() {
            // Update position
            pb.set_position((elapsed_secs * 1000.0) as u64);

            // Update message with additional info if available
            if let (Some(speed_val), Some(fps_val)) = (speed, fps) {
                let term_width = term_size::dimensions().map(|(w, _)| w).unwrap_or(80);

                if term_width >= 100 {
                    // Wide terminal - show all details inline
                    let msg = format!(
                        "Encoding - Speed: {:.2}x, Avg FPS: {:.2}",
                        speed_val, fps_val
                    );
                    pb.set_message(msg);
                } else {
                    // Narrower terminal - just show basic encoding message
                    pb.set_message("Encoding");
                }
            }

            // Force a draw update
            pb.tick();
        }
    } else {
        // Non-interactive mode: Progress is logged via debug! in ffmpeg.rs
        // This ensures progress appears in log files for daemon mode
        // No need to duplicate logging here
    }
}

/// Clear the current progress bar
pub fn clear_progress_bar() {
    if let Ok(mut pb_guard) = CURRENT_PROGRESS_BAR.lock() {
        if let Some(pb) = pb_guard.take() {
            pb.finish_and_clear();
        }
    }
}

/// Print a subsection header
/// Automatically adds spacing before the subsection for proper visual separation
///
/// # Arguments
///
/// * `title` - The title of the subsection
pub fn print_subsection(title: &str) {
    let plain_text = format!("{}{}", styling::SUBSECTION_INDENT, title);
    
    // Subsections should always be bold for Level 2 hierarchy
    // Note: Spacing is handled by the section header or previous content
    // to avoid double-spacing when subsection follows a section header
    if should_use_color() {
        info!("{}{}", styling::SUBSECTION_INDENT, title.bold());
    } else {
        info!("{}", plain_text);
    }
}

/// Print empty lines to separate logical groups
/// This replaces the divider line with proper spacing according to the design guide
pub fn print_section_separator() {
    // Add empty lines for section separation based on design guide
    for _ in 0..styling::LINE_SPACING_BETWEEN_SECTIONS {
        info!("{}", styling::EMPTY_LINE);
    }
}

/// Print a sub-item under a processing step
///
/// # Arguments
///
/// * `message` - The sub-item message to display
pub fn print_sub_item(message: &str) {
    info!("{}{}", styling::SUB_ITEM_INDENT, message);
}

/// Print a progress indicator with a message for sub-steps
///
/// # Arguments
///
/// * `message` - The progress message to display
pub fn print_progress_indicator(message: &str) {
    // Progress symbol should not be colored - Level 3 with 4-space indentation
    info!(
        "{}{} {}",
        styling::SUB_ITEM_INDENT,
        styling::PROGRESS_SYMBOL,
        message
    );
}


// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Format time in seconds as HH:MM:SS
///
/// # Arguments
///
/// * `seconds` - Time in seconds
///
/// # Returns
///
/// * Formatted time string
fn format_time_hms(seconds: f64) -> String {
    let hours = (seconds / 3600.0) as u64;
    let minutes = ((seconds % 3600.0) / 60.0) as u64;
    let secs = (seconds % 60.0) as u64;

    format!("{:02}:{:02}:{:02}", hours, minutes, secs)
}


// ============================================================================
// CLI PROGRESS REPORTER IMPLEMENTATION
// ============================================================================

/// Implementation of the ProgressReporter trait for the CLI interface
/// This centralizes all formatting decisions in the terminal module
pub struct CliProgressReporter;

/// Register the CLI progress reporter with the core library
pub fn register_cli_reporter() {
    let reporter = Box::new(CliProgressReporter);
    drapto_core::progress_reporting::set_progress_reporter(reporter);
}

// ============================================================================
// FFMPEG COMMAND FORMATTING
// ============================================================================

/// Extract FFmpeg arguments from JSON array or debug output
fn extract_ffmpeg_args(cmd_data: &str) -> Option<Vec<String>> {
    // First try to parse as JSON array
    match serde_json::from_str::<Vec<String>>(cmd_data) {
        Ok(args) => {
            debug!("Successfully parsed JSON array with {} args", args.len());
            return Some(args);
        }
        Err(e) => {
            debug!("Failed to parse as JSON: {}", e);
        }
    }
    
    // Fallback to parsing debug format from FfmpegCommand
    let cmd_str = cmd_data
        .strip_prefix("FfmpegCommand { ")?
        .strip_suffix(" }")?;
    let mut args = vec!["ffmpeg".to_string()];
    let mut in_args = false;

    // Parse the debug output to extract arguments
    for part in cmd_str.split(", ") {
        if part.starts_with("args: [") {
            in_args = true;
            if let Some(args_start) = part.strip_prefix("args: [") {
                if !args_start.is_empty() {
                    args.push(args_start.trim_matches('"').to_string());
                }
            }
        } else if in_args {
            if part.ends_with("]") {
                in_args = false;
                if let Some(last_arg) = part.strip_suffix("]") {
                    if !last_arg.is_empty() {
                        args.push(last_arg.trim_matches('"').to_string());
                    }
                }
            } else {
                args.push(part.trim_matches('"').to_string());
            }
        }
    }

    Some(args)
}

/// Parse a quoted command string into individual arguments
fn parse_quoted_command(cmd_str: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut in_quotes = false;
    let mut escape_next = false;
    
    for ch in cmd_str.chars() {
        if escape_next {
            current_arg.push(ch);
            escape_next = false;
            continue;
        }
        
        match ch {
            '\\' => {
                escape_next = true;
            }
            '"' => {
                if in_quotes {
                    // End of quoted argument
                    if !current_arg.is_empty() {
                        args.push(current_arg.clone());
                        current_arg.clear();
                    }
                    in_quotes = false;
                } else {
                    // Start of quoted argument
                    in_quotes = true;
                }
            }
            ' ' | '\t' if !in_quotes => {
                // Whitespace outside quotes - end current argument
                if !current_arg.is_empty() {
                    args.push(current_arg.clone());
                    current_arg.clear();
                }
            }
            _ => {
                current_arg.push(ch);
            }
        }
    }
    
    // Don't forget the last argument
    if !current_arg.is_empty() {
        args.push(current_arg);
    }
    
    args
}

/// Format FFmpeg command with proper grouping according to CLI design guide
fn format_ffmpeg_command_pretty(args: &[String]) -> String {
    if args.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    let mut i = 0;

    // Start with ffmpeg
    output.push_str(&args[0]);
    i += 1;

    // Group arguments by category
    let mut current_line = String::new();
    let mut in_filter = false;
    let mut needs_newline = true;

    while i < args.len() {
        let arg = &args[i];
        
        // Determine if we need a new line based on argument type
        let start_new_line = match arg.as_str() {
            // Input/output files
            "-i" | "-y" => true,
            // Hardware acceleration
            "-hwaccel" => true,
            // Filters
            "-vf" | "-af" | "-filter_complex" => {
                in_filter = true;
                true
            },
            // Mappings
            "-map" | "-map_metadata" | "-map_chapters" => !current_line.contains("-map"),
            // Codecs
            "-c:v" | "-c:a" => true,
            // Video parameters
            "-pix_fmt" | "-crf" | "-preset" | "-g" => !current_line.contains("-c:v"),
            // Codec-specific parameters
            "-svtav1-params" | "-x264-params" | "-x265-params" => true,
            // Audio parameters  
            "-b:a" | "-ac" | "-ar" => !current_line.contains("-c:a"),
            // Other flags
            "-movflags" | "-flags" => true,
            _ => {
                // Continue current line for values and unknown args
                false
            }
        };

        if start_new_line && !current_line.is_empty() {
            output.push('\n');
            output.push_str("  ");
            output.push_str(&current_line);
            current_line.clear();
            needs_newline = true;
        }

        // Add the argument
        if !current_line.is_empty() {
            current_line.push(' ');
        }
        
        // Quote arguments that need it
        if arg.contains(' ') || arg.contains('=') || (in_filter && arg.contains('[')) {
            current_line.push_str(&format!("\"{}\"", arg));
        } else {
            current_line.push_str(arg);
        }

        // Reset filter flag after filter value
        if in_filter && i > 0 && !args[i-1].starts_with('-') {
            in_filter = false;
        }

        i += 1;
    }

    // Add the last line
    if !current_line.is_empty() {
        if needs_newline {
            output.push('\n');
            output.push_str("  ");
        }
        output.push_str(&current_line);
    }

    output
}

/// Implement the ProgressReporter trait for the CLI
impl drapto_core::progress_reporting::ProgressReporter for CliProgressReporter {
    fn section(&self, title: &str) {
        print_section(title);
    }

    fn subsection(&self, title: &str) {
        print_subsection(title);
    }

    fn processing_step(&self, message: &str) {
        print_processing(message);
    }

    fn status(&self, label: &str, value: &str, highlight: bool) {
        print_status(label, value, highlight);
    }

    fn success(&self, message: &str) {
        print_success(message);
    }

    fn sub_item(&self, message: &str) {
        print_sub_item(message);
    }

    fn completion_with_status(
        &self,
        success_message: &str,
        status_label: &str,
        status_value: &str,
    ) {
        print_completion_with_status(success_message, status_label, status_value);
    }

    fn analysis_step(&self, emoji: &str, message: &str) {
        print_analysis_step(emoji, message);
    }

    fn encoding_summary(
        &self,
        filename: &str,
        duration: std::time::Duration,
        input_size: u64,
        output_size: u64,
    ) {
        print_encoding_summary(filename, duration, input_size, output_size);
    }

    fn video_filters(&self, filters_str: &str, is_sample: bool) {
        if is_sample {
            return; // Skip for sample processing
        }

        if !filters_str.is_empty() {
            // Use sub-item formatting for Level 3 hierarchy
            print_sub_item(&format!("Applying video filters: {}", filters_str));
        } else {
            print_sub_item("No video filters applied.");
        }
    }

    fn film_grain(&self, level: Option<u8>, is_sample: bool) {
        if is_sample {
            return; // Skip for sample processing
        }

        if let Some(value) = level {
            // Use sub-item formatting for Level 3 hierarchy
            print_sub_item(&format!("Applying film grain synthesis: level={}", value));
        } else {
            print_sub_item("No film grain synthesis applied (denoise level is None or 0).");
        }
    }

    fn duration(&self, duration_secs: f64, is_sample: bool) {
        if is_sample {
            return; // Skip for sample processing
        }

        // Report as a status line
        print_status("Progress duration", &format_time_hms(duration_secs), false);
    }

    fn encoder_message(&self, message: &str, is_sample: bool) {
        // ENCODER MESSAGE DISPLAY POLICY:
        //
        // This function is called for Svt[info] messages from the encoder.
        // Per our CLI design guide, we minimize terminal clutter by:
        //
        // 1. NEVER showing messages from grain analysis samples (is_sample=true)
        //    These would add 3-6 sets of encoder config output before the actual encode
        //
        // 2. Only showing actual encode messages in --verbose mode
        //    This gives users detailed encoder configuration when explicitly requested
        //
        // 3. Preserving SVT's internal formatting
        //    The encoder outputs carefully aligned tables that shouldn't be indented
        
        if is_sample {
            return; // Always skip grain analysis sample messages
        }

        // Log encoder messages at debug level (shown with --verbose)
        // Add a blank line before the first encoder message for readability
        if !FIRST_ENCODER_MESSAGE_PRINTED.swap(true, Ordering::Relaxed) {
            debug!("");
        }
        // Display SVT messages exactly as formatted by the encoder
        debug!("{}", message);
    }

    fn section_separator(&self) {
        print_section_separator();
    }

    fn hardware_acceleration(&self, _available: bool, _acceleration_type: &str) {}

    fn encode_start(&self, input_path: &std::path::Path, output_path: &std::path::Path) {
        // Extract the filename for logging
        let filename = input_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| input_path.to_string_lossy().to_string());

        // Print the encoding section header only once for the first encode
        if !ENCODING_SECTION_PRINTED.swap(true, Ordering::Relaxed) {
            print_section("ENCODING PROGRESS");
            // Use no-spacing variant for first item after section
            print_processing_no_spacing(&format!("Encoding: {}", filename));
        } else {
            // Use regular spacing for subsequent files
            print_processing(&format!("Encoding: {}", filename));
        }

        // Only show output path in verbose mode
        debug!("{}Output: {}", styling::SUB_ITEM_INDENT, output_path.display());
    }

    fn encode_error(&self, input_path: &std::path::Path, message: &str) {
        // Extract the filename for logging
        let filename = input_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| input_path.to_string_lossy().to_string());

        error!("Error encoding {}: {}", filename, message);
    }

    fn log_message(
        &self,
        message: &str,
        level: drapto_core::progress_reporting::LogLevel,
    ) {
        // Log messages are now filtered by standard log levels
        match level {
            drapto_core::progress_reporting::LogLevel::Info => info!("{}", message),
            drapto_core::progress_reporting::LogLevel::Warning => warn!("{}", message),
            drapto_core::progress_reporting::LogLevel::Error => error!("{}", message),
            drapto_core::progress_reporting::LogLevel::Debug => debug!("{}", message),
        }
    }

    fn progress_bar(
        &self,
        percent: f32,
        elapsed_secs: f64,
        total_secs: f64,
        speed: Option<f32>,
        fps: Option<f32>,
        eta: Option<Duration>,
    ) {
        print_progress_bar(
            percent,
            elapsed_secs,
            total_secs,
            Some(speed.unwrap_or(0.0)),
            Some(fps.unwrap_or(0.0)),
            Some(eta.unwrap_or_else(|| Duration::from_secs(0))),
        );
    }

    fn clear_progress_bar(&self) {
        clear_progress_bar();
    }

    fn ffmpeg_command(&self, cmd_data: &str, is_sample: bool) {
        if is_sample {
            debug!("FFmpeg command (grain sample): {}", cmd_data);
            return;
        }

        // Parse and format the FFmpeg command according to CLI design guide
        if let Some(args) = extract_ffmpeg_args(cmd_data) {
            // Print as debug output (shown with --verbose)
            debug!("{}FFmpeg command:", styling::SUB_ITEM_INDENT);
            
            // Format the command with proper grouping and indentation
            let formatted = format_ffmpeg_command_pretty(&args);
            for line in formatted.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                // Use additional indentation for command lines
                debug!("      {}", line);
            }
        } else {
            // Fallback - the command data might be in a different format
            // Check if it looks like a command string (starts with quotes)
            print_sub_item("FFmpeg command:");
            if cmd_data.trim().starts_with('"') {
                // It's a quoted command string - let's parse it differently
                let parts = parse_quoted_command(cmd_data);
                if !parts.is_empty() {
                    let formatted = format_ffmpeg_command_pretty(&parts);
                    for line in formatted.lines() {
                        if line.trim().is_empty() {
                            continue;
                        }
                        info!("      {}", line);
                    }
                } else {
                    // Last resort - just show it as is
                    info!("      {}", cmd_data);
                }
            } else {
                info!("      {}", cmd_data);
            }
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_ffmpeg_args_from_json() {
        // Test parsing JSON array format (what should be sent from core)
        let json_cmd = r#"["ffmpeg", "-loglevel", "level+info", "-hwaccel", "videotoolbox", "-i", "/path/to/input.mkv", "-c:v", "libsvtav1", "-crf", "27", "/path/to/output.mkv"]"#;
        
        let args = extract_ffmpeg_args(json_cmd);
        assert!(args.is_some());
        let args = args.unwrap();
        println!("Extracted {} args from JSON: {:?}", args.len(), args);
        assert_eq!(args.len(), 12); // Fixed count
        assert_eq!(args[0], "ffmpeg");
        assert_eq!(args[6], "/path/to/input.mkv");
    }
    
    #[test]
    fn test_parse_quoted_command() {
        // Test parsing the quoted string format we're seeing in output
        let quoted_cmd = r#""ffmpeg" "-loglevel" "level+info" "-hwaccel" "videotoolbox" "-i" "/path/to/input.mkv" "-c:v" "libsvtav1" "-crf" "27" "/path/to/output.mkv""#;
        
        let args = parse_quoted_command(quoted_cmd);
        println!("Parsed {} args from quoted: {:?}", args.len(), args);
        assert_eq!(args.len(), 12); // Fixed count
        assert_eq!(args[0], "ffmpeg");
        assert_eq!(args[6], "/path/to/input.mkv");
        assert_eq!(args[8], "libsvtav1");
    }
    
    #[test]
    fn test_format_ffmpeg_command_pretty() {
        let args = vec![
            "ffmpeg".to_string(),
            "-loglevel".to_string(),
            "level+info".to_string(),
            "-hwaccel".to_string(),
            "videotoolbox".to_string(),
            "-i".to_string(),
            "/path/to/input.mkv".to_string(),
            "-filter_complex".to_string(),
            "[0:v:0]crop=1920:1036:0:22,hqdn3d=2:1.3:8:8[vout]".to_string(),
            "-map".to_string(),
            "[vout]".to_string(),
            "-map".to_string(),
            "0:a".to_string(),
            "-c:v".to_string(),
            "libsvtav1".to_string(),
            "-crf".to_string(),
            "27".to_string(),
            "-preset".to_string(),
            "6".to_string(),
            "-svtav1-params".to_string(),
            "tune=3:film-grain=16".to_string(),
            "-c:a".to_string(),
            "libopus".to_string(),
            "-b:a:0".to_string(),
            "256k".to_string(),
            "/path/to/output.mkv".to_string(),
        ];
        
        let formatted = format_ffmpeg_command_pretty(&args);
        println!("Formatted command:\n{}", formatted);
        
        // Check that it's multi-line
        let lines: Vec<&str> = formatted.lines().collect();
        assert!(lines.len() > 5, "Command should be formatted on multiple lines");
        
        // Check first line is ffmpeg
        assert_eq!(lines[0].trim(), "ffmpeg");
        
        // Check indentation
        assert!(lines[1].starts_with("  "), "Subsequent lines should be indented");
        
        // Check that related args are grouped
        let formatted_str = formatted.to_string();
        assert!(formatted_str.contains("-hwaccel videotoolbox"));
        assert!(formatted_str.contains("-c:v libsvtav1"));
    }
    
    #[test]
    fn test_real_world_command() {
        // Test with the actual command from the user's output
        let real_cmd = r#""ffmpeg" "-loglevel" "level+info" "-hwaccel" "videotoolbox" "-i" "/Users/ken/Videos/input/Adventures in Babysitting_clip1_821s.mkv" "-hide_banner" "-af" "aformat=channel_layouts=7.1|5.1|stereo|mono" "-filter_complex" "[0:v:0]crop=1920:1036:0:22,hqdn3d=2:1.3:8:8[vout]" "-map" "[vout]" "-map" "0:a" "-map_metadata" "0" "-map_chapters" "0" "-c:v" "libsvtav1" "-pix_fmt" "yuv420p10le" "-crf" "27" "-preset" "6" "-svtav1-params" "tune=3:film-grain=16:film-grain-denoise=0" "-c:a" "libopus" "-b:a:0" "256k" "/Users/ken/Videos/output/Adventures in Babysitting_clip1_821s.mkv""#;
        
        let args = parse_quoted_command(real_cmd);
        assert!(args.len() > 20, "Should parse all arguments");
        
        let formatted = format_ffmpeg_command_pretty(&args);
        println!("\nReal command formatted:\n{}", formatted);
        
        // Verify it contains key elements properly formatted
        assert!(formatted.contains("ffmpeg\n"));
        assert!(formatted.contains("  -hwaccel videotoolbox"));
        assert!(formatted.contains("  -i \"/Users/ken/Videos/input/Adventures in Babysitting_clip1_821s.mkv\""));
    }
}

// ============================================================================
// PRE-DAEMONIZATION OUTPUT
// ============================================================================

/// Print a list of files pre-daemonization with consistent formatting
/// This uses eprintln! directly instead of log macros since logging will be
/// redirected to the daemon log file.
///
/// # Arguments
///
/// * `files` - Vector of file paths to display
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

/// Print information about log file location pre-daemonization
///
/// # Arguments
///
/// * `log_path` - Path to the log file
pub fn print_daemon_log_info(log_path: &std::path::Path) {
    eprintln!("Log file: {}", log_path.display());
}

/// Print daemon startup message
pub fn print_daemon_starting() {
    eprintln!("Starting Drapto daemon in the background...");
}

/// Print a specialized analysis status message with emoji
///
/// # Arguments
///
/// * `emoji` - The emoji character to use (e.g., "🔬")
/// * `message` - The message to display
pub fn print_analysis_step(emoji: &str, message: &str) {
    // Emoji should not have special formatting - keep it simple
    // This is typically used for verbose mode analysis steps
    info!("{} {}", emoji, message);
}

/// Print a file list with consistent formatting
///
/// # Arguments
///
/// * `header` - Header message to display before the list
/// * `files` - Vector of file paths to display
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

/// Print an encoding summary with consistent formatting
///
/// # Arguments
///
/// * `filename` - Name of the encoded file
/// * `duration` - Encoding duration
/// * `input_size` - Size of input file in bytes
/// * `output_size` - Size of output file in bytes
pub fn print_encoding_summary(
    filename: &str,
    duration: std::time::Duration,
    input_size: u64,
    output_size: u64,
) {
    // Clear any active progress bar first
    clear_progress_bar();

    // Calculate size reduction percentage
    let reduction = if input_size > 0 {
        100 - ((output_size * 100) / input_size)
    } else {
        0
    };

    // Add extra spacing before the summary
    info!("{}", styling::EMPTY_LINE);

    // Print the summary with consistent formatting
    info!("{}", filename);
    info!(
        "  {:<13} {}",
        "Encode time:",
        format_time_hms(duration.as_secs_f64())
    );
    info!("  {:<13} {}", "Input size:", format_bytes(input_size));
    info!("  {:<13} {}", "Output size:", format_bytes(output_size));
    
    // Color significant reductions (>50%) in green according to design guide
    let reduction_str = format!("{}%", reduction);
    let reduction_display = if should_use_color() && reduction >= 50 {
        reduction_str.green().to_string()
    } else {
        reduction_str
    };
    info!("  {:<13} {}", "Reduced by:", reduction_display);

    // Add extra spacing after the summary
    info!("{}", styling::EMPTY_LINE);
}

/// Data structure for grain analysis table
#[derive(tabled::Tabled)]
pub struct GrainAnalysisRow {
    #[tabled(rename = "Sample")]
    pub sample: String,
    #[tabled(rename = "Time")]
    pub time: String,
    #[tabled(rename = "Size (MB)")]
    pub size_mb: String,
    #[tabled(rename = "Quality")]
    pub quality: String,
    #[tabled(rename = "Selection")]
    pub selection: String,
}

/// Print grain analysis results as a table
///
/// # Arguments
///
/// * `results` - Vector of grain analysis results to display
pub fn print_grain_analysis_table(results: &[GrainAnalysisRow]) {
    if results.is_empty() {
        return;
    }

    // Check terminal width
    let term_width = term_size::dimensions().map(|(w, _)| w).unwrap_or(80);

    if term_width < 60 {
        // Fall back to simple list for narrow terminals
        for (i, result) in results.iter().enumerate() {
            info!(
                "{}Sample {}: {} - {} MB ({})",
                styling::SUB_ITEM_INDENT,
                i + 1,
                result.time,
                result.size_mb,
                result.selection
            );
        }
        return;
    }

    // Create and configure table
    let mut table = Table::new(results);

    // Use ASCII style matching design guide
    table
        .with(tabled::settings::Style::ascii())
        .with(tabled::settings::Alignment::left())
        .with(tabled::settings::Padding::new(1, 1, 0, 0));

    // Print each line with proper indentation
    for line in table.to_string().lines() {
        info!("{}{}", styling::SUB_ITEM_INDENT, line);
    }
}

/// Data structure for encoding summary table
#[derive(tabled::Tabled)]
pub struct EncodingSummaryRow {
    #[tabled(rename = "File")]
    pub file: String,
    #[tabled(rename = "Input")]
    pub input: String,
    #[tabled(rename = "Output")]
    pub output: String,
    #[tabled(rename = "Reduction")]
    pub reduction: String,
    #[tabled(rename = "Time")]
    pub time: String,
}

/// Print encoding summary table for multiple files
///
/// # Arguments
///
/// * `summaries` - Vector of encoding summaries to display
pub fn print_encoding_summary_table(summaries: &[EncodingSummaryRow]) {
    if summaries.is_empty() {
        return;
    }

    print_section("ENCODING SUMMARY");

    // Check terminal width
    let term_width = term_size::dimensions().map(|(w, _)| w).unwrap_or(80);

    if term_width < 80 {
        // Simple list for narrow terminals
        for summary in summaries {
            info!(
                "{}{}: {} → {} ({})",
                styling::STATUS_INDENT,
                summary.file,
                summary.input,
                summary.output,
                summary.reduction
            );
        }
        return;
    }

    // Create table with minimal style
    let mut table = Table::new(summaries);
    table
        .with(tabled::settings::Style::blank())
        .with(tabled::settings::Alignment::left());

    // Print table
    for line in table.to_string().lines() {
        info!("{}{}", styling::STATUS_INDENT, line);
    }
}
