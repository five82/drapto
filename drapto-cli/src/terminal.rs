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
use log::{debug, info, error, warn};
use std::time::Duration;
use std::io::{self, Write};
use colored::*;
use std::sync::atomic::{AtomicBool, Ordering};

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
    
    // Section formatting
    pub const SECTION_PREFIX: &str = "===== ";
    pub const SECTION_SUFFIX: &str = " =====";
    
    // Indentation
    pub const STATUS_INDENT: &str = "  ";
    pub const SUBSECTION_INDENT: &str = "  ";
    pub const SUB_ITEM_INDENT: &str = "    "; // For indenting items under a processing step
    
    // Progress bar
    pub const PROGRESS_FILL: &str = "#";
    pub const PROGRESS_EMPTY: &str = ".";
    
    // Empty line for vertical spacing
    pub const EMPTY_LINE: &str = "";
    
    // Vertical spacing - adjusted to match design guide
    pub const LINE_SPACING_BEFORE_SECTION: usize = 1;      // Single line break before sections
    pub const LINE_SPACING_AFTER_SECTION: usize = 1;       // Single line after section header
    pub const LINE_SPACING_BEFORE_PROCESSING: usize = 1;   // Single line break between subsections
    pub const LINE_SPACING_AFTER_SUCCESS: usize = 0;       // No extra lines after success message
    pub const LINE_SPACING_BETWEEN_SECTIONS: usize = 1;    // Single line break between sections
}

// ============================================================================
// VERBOSITY CONTROL
// ============================================================================

/// Verbosity levels for terminal output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerbosityLevel {
    /// Normal mode (quiet by default, only essential info)
    Normal,
    /// Verbose mode (detailed output for troubleshooting)
    Verbose,
}

// Global verbosity setting
static mut VERBOSITY: VerbosityLevel = VerbosityLevel::Normal;

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
fn should_use_color() -> bool {
    USE_COLOR.load(Ordering::Relaxed)
}

/// Set the verbosity level for terminal output
/// 
/// This also synchronizes the verbosity level with the core library
/// to maintain consistent output formatting.
pub fn set_verbosity(level: VerbosityLevel) {
    unsafe { VERBOSITY = level; }
    
    // Propagate verbosity to core library
    let core_verbosity = match level {
        VerbosityLevel::Normal => drapto_core::progress_reporting::VerbosityLevel::Normal,
        VerbosityLevel::Verbose => drapto_core::progress_reporting::VerbosityLevel::Verbose,
    };
    drapto_core::progress_reporting::set_verbosity(core_verbosity);
}

/// Check if output should be printed based on verbosity level
///
/// This function is used to determine if output at the specified verbosity level
/// should be printed based on the current verbosity setting.
///
/// # Arguments
///
/// * `level` - The verbosity level of the output to check
///
/// # Returns
///
/// * `true` if the output should be printed, `false` otherwise
pub fn should_print(level: VerbosityLevel) -> bool {
    let current = unsafe { VERBOSITY };
    match (current, level) {
        (VerbosityLevel::Normal, VerbosityLevel::Verbose) => false,
        _ => true,
    }
}

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
    if should_print(VerbosityLevel::Normal) {
        // Add consistent empty lines before section for vertical spacing
        for _ in 0..styling::LINE_SPACING_BEFORE_SECTION {
            info!("{}", styling::EMPTY_LINE);
        }
        
        // Format section header with uppercase title and cyan color for the title only
        let header = if should_use_color() {
            format!("{}{}{}", 
                styling::SECTION_PREFIX, 
                title.to_uppercase().cyan().bold(), 
                styling::SECTION_SUFFIX
            )
        } else {
            format!("{}{}{}", styling::SECTION_PREFIX, title.to_uppercase(), styling::SECTION_SUFFIX)
        };
        
        // Log the section header using the info! macro
        info!("{}", header);
        
        // Add consistent empty lines after section header for spacing
        for _ in 0..styling::LINE_SPACING_AFTER_SECTION {
            info!("{}", styling::EMPTY_LINE);
        }
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
    if should_print(VerbosityLevel::Normal) {
        // Use a shorter padding for more compact output
        let padding = if label.len() < 15 {
            15 - label.len()
        } else {
            1
        };
        
        let formatted_label = format!("{}{}{}", label, ":", " ".repeat(padding));
        
        // Format the value with appropriate styling - bold for important values (Level 4 hierarchy)
        let formatted_value = if should_use_color() && highlight {
            // Use bold for highlighted/important values, optionally with green for critical success values
            value.bold().to_string()
        } else if should_use_color() {
            // Regular values can still be bold if they're key information
            value.to_string()
        } else {
            value.to_string()
        };
        
        // Log the status line using the info! macro
        info!("{}{} {}", styling::STATUS_INDENT, formatted_label, formatted_value);
    }
}

/// Print a success message
///
/// # Arguments
///
/// * `message` - The success message to display
pub fn print_success(message: &str) {
    if should_print(VerbosityLevel::Normal) {
        // Success symbol should not be colored - add proper indentation for Level 2
        info!("{}{} {}", styling::SUBSECTION_INDENT, styling::SUCCESS_SYMBOL, message);
        
        // Add spacing after success messages for visual clarity
        for _ in 0..styling::LINE_SPACING_AFTER_SUCCESS {
            info!("{}", styling::EMPTY_LINE);
        }
    }
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
    if should_print(VerbosityLevel::Normal) {
        // Add a blank line before the success message for proper spacing between subsections
        info!("{}", styling::EMPTY_LINE);
        
        // Success symbol should not be colored, add proper indentation for Level 2
        info!("{}{} {}", styling::SUBSECTION_INDENT, styling::SUCCESS_SYMBOL, success_message);
    }
    
    // Print the associated status line with Level 4 indentation
    print_status(status_label, status_value, false);
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
    if should_print(VerbosityLevel::Normal) {
        // Add spacing before processing steps for visual grouping (if requested)
        if add_spacing {
            for _ in 0..styling::LINE_SPACING_BEFORE_PROCESSING {
                info!("{}", styling::EMPTY_LINE);
            }
        }
        
        // Processing symbol should not be colored; make message bold for Level 2 hierarchy
        if should_use_color() {
            info!("{}{} {}", styling::SUBSECTION_INDENT, styling::PROCESSING_SYMBOL, message.bold());
        } else {
            info!("{}{} {}", styling::SUBSECTION_INDENT, styling::PROCESSING_SYMBOL, message);
        }
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

/// Print a progress bar
///
/// # Arguments
///
/// * `percent` - Progress percentage (0.0 to 100.0)
/// * `elapsed` - Elapsed time
/// * `total` - Total duration
/// * `speed` - Optional encoding speed multiplier
/// * `fps` - Optional frames per second
/// * `eta` - Optional estimated time remaining
pub fn print_progress_bar(
    percent: f32, 
    elapsed_secs: f64, 
    total_secs: f64,
    speed: Option<f32>, 
    fps: Option<f32>, 
    eta: Option<Duration>
) {
    if should_print(VerbosityLevel::Normal) {
        // Get terminal width
        let term_width = match term_size::dimensions() {
            Some((width, _)) => width,
            None => 80, // Default width if terminal size can't be determined
        };
        
        // Calculate bar width based on available space
        let bar_width = (term_width.saturating_sub(50)).max(20); // Min 20 chars, max term_width-50
        
        // Format times
        let elapsed_str = format_time_hms(elapsed_secs);
        let total_str = format_time_hms(total_secs);
        
        // Create progress bar
        let filled_width = ((percent / 100.0) * (bar_width as f32)).round() as usize;
        let empty_width = bar_width.saturating_sub(filled_width);
        
        let filled = styling::PROGRESS_FILL.repeat(filled_width);
        let empty = styling::PROGRESS_EMPTY.repeat(empty_width);
        
        // Format ETA
        let eta_str = match eta {
            Some(eta_duration) if eta_duration.as_secs() > 0 => {
                format!("ETA: {}", format_time_hms(eta_duration.as_secs_f64()))
            },
            _ => "ETA: < 1s".to_string(),
        };
        
        // Create progress line
        let progress_symbol = styling::PROGRESS_SYMBOL;
        
        let progress_bar = format!("[{}{}]", filled, empty);
        
        let mut progress_line = format!(
            "{} Encoding: {:.1}% {} ({} / {})", 
            progress_symbol, 
            percent, 
            progress_bar, 
            elapsed_str, 
            total_str
        );
        
        // Add extra info if available
        if let (Some(speed_val), Some(fps_val)) = (speed, fps) {
            let extra_info = format!(", Speed: {:.2}x, Avg FPS: {:.2}, {}", speed_val, fps_val, eta_str);
            progress_line.push_str(&extra_info);
        }
        
        // Use info! macro to log the progress bar
        info!("{}", progress_line);
        
        // Also log to debug level for potential file logging
        debug!("{}", progress_line);
        
        // Ensure the output is flushed to make real-time progress visible
        let _ = io::stdout().flush();
    }
}

/// Print a subsection header
///
/// # Arguments
///
/// * `title` - The title of the subsection
pub fn print_subsection(title: &str) {
    if should_print(VerbosityLevel::Normal) {
        // Subsections should always be bold for Level 2 hierarchy
        if should_use_color() {
            info!("{}{}", styling::SUBSECTION_INDENT, title.bold());
        } else {
            info!("{}{}", styling::SUBSECTION_INDENT, title);
        }
    }
}

/// Print empty lines to separate logical groups
/// This replaces the divider line with proper spacing according to the design guide
pub fn print_section_separator() {
    if should_print(VerbosityLevel::Normal) {
        // Add empty lines for section separation based on design guide
        for _ in 0..styling::LINE_SPACING_BETWEEN_SECTIONS {
            info!("{}", styling::EMPTY_LINE);
        }
    }
}

/// Print a sub-item under a processing step
///
/// # Arguments
///
/// * `message` - The sub-item message to display
pub fn print_sub_item(message: &str) {
    if should_print(VerbosityLevel::Normal) {
        info!("{}{}", styling::SUB_ITEM_INDENT, message);
    }
}

/// Print a progress indicator with a message for sub-steps
///
/// # Arguments
///
/// * `message` - The progress message to display
pub fn print_progress_indicator(message: &str) {
    if should_print(VerbosityLevel::Normal) {
        // Progress symbol should not be colored - Level 3 with 4-space indentation
        info!("{}{} {}", styling::SUB_ITEM_INDENT, styling::PROGRESS_SYMBOL, message);
    }
}

/// Print an empty line for vertical spacing
pub fn print_empty_line() {
    if should_print(VerbosityLevel::Normal) {
        info!("{}", styling::EMPTY_LINE);
    }
}

/// Format command output details
///
/// # Arguments
///
/// * `command` - The command to format
/// * `verbose` - Whether to include full details
pub fn format_command(command: &str, verbose: bool) -> String {
    if verbose {
        // In verbose mode, return the formatted command with line breaks for readability
        format_ffmpeg_command(command)
    } else {
        // In normal mode, simplify the command significantly
        // Extract just the important parts
        let simplified = simplify_ffmpeg_command(command);
        if simplified.len() > 120 {
            format!("{}...", &simplified[0..120])
        } else {
            simplified
        }
    }
}

/// Format an FFmpeg command with proper line breaks and grouping
/// 
/// This formats an FFmpeg command string into a more readable format
/// by splitting it into logical groups with line breaks.
///
/// # Arguments
///
/// * `command` - The FFmpeg command string to format
///
/// # Returns
///
/// * The formatted command with line breaks
fn format_ffmpeg_command(command: &str) -> String {
    // Remove quotes for better parsing
    let cmd = command.replace("\"", "");
    
    // Split the command into tokens
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    
    let mut result = String::new();
    let mut current_group = String::new();
    let mut i = 0;
    
    while i < parts.len() {
        let part = parts[i];
        
        // Start a new line for major option groups
        if part.starts_with('-') && !part.starts_with("--") && part != "-i" && part != "-y" && part != "-f" &&
           !current_group.is_empty() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(&format!("  {}", current_group.trim()));
            current_group.clear();
        }
        
        // Add the current part to the group
        current_group.push_str(&format!(" {}", part));
        
        // Special handling for input files
        if part == "-i" && i + 1 < parts.len() {
            current_group.push_str(&format!(" {}", parts[i + 1]));
            i += 1;
        }
        
        i += 1;
    }
    
    // Add the last group
    if !current_group.is_empty() {
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(&format!("  {}", current_group.trim()));
    }
    
    // Clean up special characters and wrap lines
    result = result.replace("\\\"", "\"");
    
    format!("ffmpeg\n{}", result)
}

/// Simplify an FFmpeg command by extracting just the important parts
///
/// This removes verbose FFmpeg options and keeps only the most important
/// parts for clarity in the normal mode output.
///
/// # Arguments
///
/// * `command` - The FFmpeg command string to simplify
///
/// # Returns
///
/// * The simplified command
fn simplify_ffmpeg_command(command: &str) -> String {
    // For a real implementation, extract only the critical parameters
    // This is a simplified version that just shows major components
    
    let mut simplified = String::from("ffmpeg");
    
    // Extract input file
    if let Some(input_index) = command.find(" -i ") {
        let input_start = input_index + 4;
        let input_end = command[input_start..].find(' ').map_or(command.len(), |pos| input_start + pos);
        let input = &command[input_start..input_end];
        simplified.push_str(&format!(" -i {}", input));
    }
    
    // Extract video codec
    if let Some(codec_index) = command.find(" -c:v ") {
        let codec_start = codec_index + 6;
        let codec_end = command[codec_start..].find(' ').map_or(command.len(), |pos| codec_start + pos);
        let codec = &command[codec_start..codec_end];
        simplified.push_str(&format!(" -c:v {}", codec));
    }
    
    // Extract CRF value
    if let Some(crf_index) = command.find(" -crf ") {
        let crf_start = crf_index + 6;
        let crf_end = command[crf_start..].find(' ').map_or(command.len(), |pos| crf_start + pos);
        let crf = &command[crf_start..crf_end];
        simplified.push_str(&format!(" -crf {}", crf));
    }
    
    // Extract output file (last argument)
    if let Some(last_space) = command.rfind(' ') {
        let output = &command[last_space + 1..];
        simplified.push_str(&format!(" {}", output));
    }
    
    simplified
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

/// Format bytes as human-readable size (KB, MB, GB)
///
/// # Arguments
///
/// * `bytes` - Size in bytes
///
/// # Returns
///
/// * Formatted size string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if bytes < KB {
        format!("{} bytes", bytes)
    } else if bytes < MB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    }
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
    
    fn completion_with_status(&self, success_message: &str, status_label: &str, status_value: &str) {
        print_completion_with_status(success_message, status_label, status_value);
    }
    
    fn analysis_step(&self, emoji: &str, message: &str) {
        print_analysis_step(emoji, message);
    }
    
    fn encoding_summary(&self, filename: &str, duration: std::time::Duration, input_size: u64, output_size: u64) {
        print_encoding_summary(filename, duration, input_size, output_size);
    }
    
    fn video_filters(&self, filters_str: &str, is_sample: bool) {
        if is_sample {
            return; // Skip for sample processing
        }
        
        if !filters_str.is_empty() {
            if should_print(VerbosityLevel::Normal) {
                // Use sub-item formatting for Level 3 hierarchy
                print_sub_item(&format!("Applying video filters: {}", filters_str));
            }
        } else {
            if should_print(VerbosityLevel::Normal) {
                print_sub_item("No video filters applied.");
            }
        }
    }
    
    fn film_grain(&self, level: Option<u8>, is_sample: bool) {
        if is_sample {
            return; // Skip for sample processing
        }
        
        if let Some(value) = level {
            if should_print(VerbosityLevel::Normal) {
                // Use sub-item formatting for Level 3 hierarchy
                print_sub_item(&format!("Applying film grain synthesis: level={}", value));
            }
        } else {
            if should_print(VerbosityLevel::Normal) {
                print_sub_item("No film grain synthesis applied (denoise level is None or 0).");
            }
        }
    }
    
    fn duration(&self, duration_secs: f64, is_sample: bool) {
        if is_sample {
            return; // Skip for sample processing
        }
        
        if should_print(VerbosityLevel::Normal) {
            // Use sub-item formatting for Level 3 hierarchy
            print_sub_item(&format!("Using provided duration for progress: {}", format_time_hms(duration_secs)));
        }
    }
    
    fn encoder_message(&self, message: &str, is_sample: bool) {
        if is_sample {
            return; // Skip for sample processing
        }
        
        if should_print(VerbosityLevel::Normal) {
            // Add a blank line before the first encoder message for readability
            if !FIRST_ENCODER_MESSAGE_PRINTED.swap(true, Ordering::Relaxed) {
                print_empty_line();
            }
            info!("{}", message);
        }
    }
    
    fn section_separator(&self) {
        print_section_separator();
    }
    
    fn hardware_acceleration(&self, _available: bool, _acceleration_type: &str) {
    }
    
    fn encode_start(&self, input_path: &std::path::Path, output_path: &std::path::Path) {
        if should_print(VerbosityLevel::Normal) {
            // Extract the filename for logging
            let filename = input_path.file_name()
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
            if should_print(VerbosityLevel::Verbose) {
                print_sub_item(&format!("Output: {}", output_path.display()));
            }
        }
    }
    
    fn encode_error(&self, input_path: &std::path::Path, message: &str) {
        // Extract the filename for logging
        let filename = input_path.file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| input_path.to_string_lossy().to_string());
            
        error!("Error encoding {}: {}", filename, message);
    }
    
    fn log_message(&self, message: &str, level: drapto_core::progress_reporting::LogLevel, verbosity: Option<drapto_core::progress_reporting::VerbosityLevel>) {
        // Convert the core verbosity to CLI verbosity
        let should_log = if let Some(v) = verbosity {
            match v {
                drapto_core::progress_reporting::VerbosityLevel::Normal => {
                    should_print(VerbosityLevel::Normal)
                },
                drapto_core::progress_reporting::VerbosityLevel::Verbose => {
                    should_print(VerbosityLevel::Verbose)
                }
            }
        } else {
            true
        };
        
        if should_log {
            match level {
                drapto_core::progress_reporting::LogLevel::Info => info!("{}", message),
                drapto_core::progress_reporting::LogLevel::Warning => warn!("{}", message),
                drapto_core::progress_reporting::LogLevel::Error => error!("{}", message),
                drapto_core::progress_reporting::LogLevel::Debug => debug!("{}", message),
            }
        }
    }
    
    fn progress_bar(&self, percent: f32, elapsed_secs: f64, total_secs: f64, speed: Option<f32>, fps: Option<f32>, eta: Option<Duration>) {
        print_progress_bar(
            percent,
            elapsed_secs,
            total_secs,
            Some(speed.unwrap_or(0.0)),
            Some(fps.unwrap_or(0.0)),
            Some(eta.unwrap_or_else(|| Duration::from_secs(0)))
        );
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
    if should_print(VerbosityLevel::Normal) {
        // Emoji should not have special formatting - keep it simple
        // This is typically used for verbose mode analysis steps
        info!("{} {}", emoji, message);
    }
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
pub fn print_encoding_summary(filename: &str, duration: std::time::Duration, input_size: u64, output_size: u64) {
    if should_print(VerbosityLevel::Normal) {
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
        info!("  {:<13} {}", "Encode time:", format_time_hms(duration.as_secs_f64()));
        info!("  {:<13} {}", "Input size:", format_bytes(input_size));
        info!("  {:<13} {}", "Output size:", format_bytes(output_size));
        info!("  {:<13} {}", "Reduced by:", format!("{}%", reduction));
        
        // Add extra spacing after the summary
        info!("{}", styling::EMPTY_LINE);
    }
}