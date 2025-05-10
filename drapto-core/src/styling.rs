// ============================================================================
// drapto-core/src/styling.rs
// ============================================================================
//
// STYLING: Centralized Terminal Output Styling
//
// This module provides centralized styling functions and constants for
// consistent terminal output formatting throughout the drapto codebase.
// It defines color schemes, formatting patterns, and helper functions
// to ensure a uniform look and feel for all user-facing output.
//
// KEY COMPONENTS:
// - Color constants for different types of information
// - Styling functions for consistent formatting
// - Helper functions for common output patterns
//
// USAGE:
// Import this module and use its functions to format terminal output:
// ```
// use crate::styling;
// info!("{}", styling::format_header("Section Title"));
// info!("{}", styling::format_key_value("Label", "Value"));
// ```
//
// STYLING GUIDELINES:
// - Headers/Sections: Cyan, Bold
// - Labels/Keys: Cyan (not bold)
// - Values/Data: Green
// - Filenames/Paths: Yellow
// - Progress Indicators: Green, Bold for percentages and important metrics
// - Warnings: Yellow, Bold
// - Errors: Red, Bold
// - Dividers/Separators: Cyan
//
// AI-ASSISTANT-INFO: Centralized styling module for consistent terminal output

// ---- External crate imports ----
use colored::*;
// Removed unused import: std::io::Write
use std::process::{Command, Stdio};

// ============================================================================
// STYLING CONSTANTS
// ============================================================================

// Color scheme constants
pub const COLOR_HEADER: Color = Color::Cyan;
pub const COLOR_LABEL: Color = Color::Cyan;
pub const COLOR_VALUE: Color = Color::Green;
pub const COLOR_FILENAME: Color = Color::Yellow;
pub const COLOR_WARNING: Color = Color::Yellow;
pub const COLOR_ERROR: Color = Color::Red;
pub const COLOR_PROGRESS: Color = Color::Green;
pub const COLOR_TIME: Color = Color::Yellow;
pub const COLOR_DIVIDER: Color = Color::Cyan;
pub const COLOR_SUCCESS: Color = Color::Green;
pub const COLOR_METRIC: Color = Color::Magenta;
pub const COLOR_PHASE: Color = Color::Blue;
pub const COLOR_SECTION: Color = Color::BrightBlue;
pub const COLOR_SUBSECTION: Color = Color::Blue;
pub const COLOR_INFO: Color = Color::BrightWhite;
pub const COLOR_DETAIL: Color = Color::BrightBlack; // For less important details
pub const COLOR_HIGHLIGHT: Color = Color::BrightGreen; // For highlighting important values

// ============================================================================
// BASIC STYLING FUNCTIONS
// ============================================================================

/// Formats a section header with consistent styling (cyan, bold)
pub fn format_header(text: &str) -> ColoredString {
    text.color(COLOR_HEADER).bold()
}

/// Formats a label (key in key-value pair) with consistent styling (cyan)
pub fn format_label(text: &str) -> ColoredString {
    text.color(COLOR_LABEL)
}

/// Formats a value with consistent styling (green)
pub fn format_value(text: &str) -> ColoredString {
    text.color(COLOR_VALUE)
}

/// Formats a filename or path with consistent styling (yellow)
pub fn format_filename(text: &str) -> ColoredString {
    text.color(COLOR_FILENAME).bold()
}

/// Formats a warning prefix with consistent styling (yellow, bold)
pub fn format_warning_prefix() -> ColoredString {
    "Warning:".color(COLOR_WARNING).bold()
}

/// Formats a warning message with consistent styling (yellow)
pub fn format_warning(text: &str) -> String {
    format!("{} {}", format_warning_prefix(), text)
}

/// Formats an error prefix with consistent styling (red, bold)
pub fn format_error_prefix() -> ColoredString {
    "Error:".color(COLOR_ERROR).bold()
}

/// Formats an error message with consistent styling (red)
pub fn format_error(text: &str) -> String {
    format!("{} {}", format_error_prefix(), text)
}

/// Formats a progress percentage with consistent styling (green, bold)
pub fn format_progress(percent: f64) -> ColoredString {
    format!("{:.2}%", percent).color(COLOR_PROGRESS).bold()
}

/// Formats a time value with consistent styling (yellow)
pub fn format_time_value(text: &str) -> ColoredString {
    text.color(COLOR_TIME)
}

/// Formats a divider line with consistent styling (cyan)
pub fn format_divider() -> ColoredString {
    "========================================".color(COLOR_DIVIDER)
}

/// Formats a short divider line with consistent styling (cyan)
pub fn format_short_divider() -> ColoredString {
    "----------------------------------------".color(COLOR_DIVIDER)
}

// ============================================================================
// COMPOSITE FORMATTING FUNCTIONS
// ============================================================================

/// Formats a key-value pair with consistent styling
///
/// # Arguments
///
/// * `key` - The label/key to display
/// * `value` - The value to display
///
/// # Returns
///
/// * A formatted string with the key-value pair
pub fn format_key_value(key: &str, value: &str) -> String {
    format!("  {:<25} {}", format_label(key), format_value(value))
}

/// Formats a progress message with consistent styling
///
/// # Arguments
///
/// * `label` - The progress label (e.g., "Encoding progress:")
/// * `percent` - The progress percentage (0-100)
/// * `current` - The current progress value (e.g., time elapsed)
/// * `total` - The total expected value (e.g., total duration)
/// * `speed` - The processing speed (e.g., 2.5x)
/// * `fps` - The frames per second
/// * `eta` - The estimated time remaining
///
/// # Returns
///
/// * A formatted progress message string
pub fn format_progress_message(
    label: &str,
    percent: f64,
    current: &str,
    total: &str,
    speed: f64,
    fps: f64,
    eta: &str,
) -> String {
    format!(
        "⏳ {} {} ({} / {}), Speed: {}, Avg FPS: {:.2}, ETA: {}",
        format_label(label),
        format_progress(percent),
        format_time_value(current),
        format_time_value(total),
        format!("{:.2}x", speed).color(COLOR_PROGRESS).bold(),
        fps,
        format_time_value(eta).bold()
    )
}

/// Formats a hardware acceleration status message
///
/// # Arguments
///
/// * `enabled` - Whether hardware acceleration is enabled
/// * `details` - Additional details about the hardware acceleration
///
/// # Returns
///
/// * A formatted hardware acceleration status message
pub fn format_hardware_status(enabled: bool, details: &str) -> String {
    if enabled {
        format!(
            "  {} {}",
            format_label("Hardware:"),
            format_value(details).bold()
        )
    } else {
        format!(
            "  {} {}",
            format_label("Hardware:"),
            "No hardware acceleration available".color(COLOR_WARNING)
        )
    }
}

/// Formats a success message with consistent styling (green, bold)
///
/// # Arguments
///
/// * `text` - The success message to display
///
/// # Returns
///
/// * A formatted success message string
pub fn format_success(text: &str) -> String {
    format!("✅ {}", text.color(COLOR_SUCCESS).bold())
}

/// Formats a phase header with consistent styling (blue, bold)
///
/// # Arguments
///
/// * `phase_number` - The phase number
/// * `description` - The phase description
///
/// # Returns
///
/// * A formatted phase header string
pub fn format_phase_header(phase_number: usize, description: &str) -> String {
    format!(
        "🔍 {} {}",
        format!("Phase {}:", phase_number).color(COLOR_PHASE).bold(),
        description.color(COLOR_PHASE)
    )
}

/// Formats a metric value with consistent styling (magenta, bold for important values)
///
/// # Arguments
///
/// * `value` - The metric value
/// * `important` - Whether this is an important metric that should be bold
///
/// # Returns
///
/// * A formatted metric value
pub fn format_metric(value: &str, important: bool) -> ColoredString {
    if important {
        value.color(COLOR_METRIC).bold()
    } else {
        value.color(COLOR_METRIC)
    }
}

/// Formats a processing step with consistent styling
///
/// # Arguments
///
/// * `step` - The processing step description
///
/// # Returns
///
/// * A formatted processing step string
pub fn format_processing_step(step: &str) -> String {
    format!("⚙️ {}", step.color(COLOR_HEADER))
}

/// Formats a result with consistent styling
///
/// # Arguments
///
/// * `label` - The result label
/// * `value` - The result value
/// * `important` - Whether this is an important result that should be bold
///
/// # Returns
///
/// * A formatted result string
pub fn format_result(label: &str, value: &str, important: bool) -> String {
    if important {
        format!("  {} {}", format_label(label), format_value(value).bold())
    } else {
        format_key_value(label, value)
    }
}

/// Formats a sample processing message with consistent styling
///
/// # Arguments
///
/// * `sample_number` - The sample number
/// * `total_samples` - The total number of samples
/// * `description` - Additional description
///
/// # Returns
///
/// * A formatted sample processing message
pub fn format_sample_processing(sample_number: usize, total_samples: usize, description: &str) -> String {
    format!(
        "📊 {} {}/{}{}{}",
        format_label("Sample"),
        format_value(&sample_number.to_string()).bold(),
        format_value(&total_samples.to_string()),
        if description.is_empty() { "".to_string() } else { ": ".to_string() },
        format_value(description)
    )
}

// ============================================================================
// ENHANCED VISUAL HIERARCHY FUNCTIONS
// ============================================================================

/// Formats a major section header with distinct styling and a full-width divider
///
/// # Arguments
///
/// * `text` - The section title
///
/// # Returns
///
/// * A formatted section header string with dividers
pub fn format_section(text: &str) -> String {
    format!(
        "{}\n{}\n{}",
        format_divider(),
        format!("  {} ", text).color(COLOR_SECTION).bold().on_black(),
        format_divider()
    )
}

/// Formats a subsection header with distinct styling
///
/// # Arguments
///
/// * `text` - The subsection title
///
/// # Returns
///
/// * A formatted subsection header string
pub fn format_subsection(text: &str) -> String {
    format!(
        "{}\n  {}",
        format_short_divider(),
        text.color(COLOR_SUBSECTION).bold()
    )
}

/// Formats an indented group of related information
///
/// # Arguments
///
/// * `title` - The group title
/// * `content` - The content lines (each will be indented)
///
/// # Returns
///
/// * A formatted group with indented content
pub fn format_group(title: &str, content: &[String]) -> String {
    let title_line = format!("  {} ", title).color(COLOR_LABEL).bold();
    let indented_content: Vec<String> = content
        .iter()
        .map(|line| format!("    {}", line))
        .collect();

    format!(
        "{}\n{}",
        title_line,
        indented_content.join("\n")
    )
}

/// Formats a spinner for operations without percentage progress
///
/// # Arguments
///
/// * `message` - The message to display
/// * `operation` - The current operation (optional)
///
/// # Returns
///
/// * A formatted spinner message
pub fn format_spinner(message: &str, operation: Option<&str>) -> String {
    match operation {
        Some(op) => format!(
            "⏳ {} {}",
            format_label(message),
            format_value(op).bold()
        ),
        None => format!("⏳ {}", format_label(message))
    }
}

/// Formats a detailed error message with context and potential solution
///
/// # Arguments
///
/// * `message` - The error message
/// * `context` - Additional context about where/why the error occurred
/// * `solution` - Potential solution or next steps (optional)
///
/// # Returns
///
/// * A formatted detailed error message
pub fn format_detailed_error(message: &str, context: &str, solution: Option<&str>) -> String {
    let solution_text = match solution {
        Some(sol) => format!("\n  {} {}", "Suggestion:".color(COLOR_LABEL).bold(), sol.color(COLOR_VALUE)),
        None => String::new()
    };

    format!(
        "{}\n  {} {}\n  {} {}{}",
        format_error_prefix(),
        "Message:".color(COLOR_LABEL).bold(),
        message,
        "Context:".color(COLOR_LABEL).bold(),
        context,
        solution_text
    )
}

/// Formats a command with syntax highlighting for better readability
///
/// # Arguments
///
/// * `command` - The command name
/// * `args` - The command arguments
///
/// # Returns
///
/// * A formatted command string with syntax highlighting
pub fn format_command(command: &str, args: &[&str]) -> String {
    let formatted_command = command.color(COLOR_HEADER).bold();
    let formatted_args = args
        .iter()
        .map(|arg| {
            if arg.starts_with('-') {
                arg.color(COLOR_LABEL).to_string()
            } else {
                arg.color(COLOR_VALUE).to_string()
            }
        })
        .collect::<Vec<String>>()
        .join(" ");

    format!("{} {}", formatted_command, formatted_args)
}

/// Formats an enhanced progress bar with additional context
///
/// # Arguments
///
/// * `percent` - The progress percentage (0-100)
/// * `message` - The progress message
/// * `context` - Additional context about the current operation
/// * `elapsed` - Elapsed time (optional)
///
/// # Returns
///
/// * A formatted progress bar with context
pub fn format_enhanced_progress(percent: f64, message: &str, context: &str, elapsed: Option<&str>) -> String {
    let elapsed_text = match elapsed {
        Some(time) => format!(" (elapsed: {})", time.color(COLOR_TIME)),
        None => String::new()
    };

    let progress_bar = create_progress_bar(percent, 20);

    format!(
        "⏳ {} {} [{}] {}{}",
        format_label(message),
        format_progress(percent),
        progress_bar,
        context.color(COLOR_INFO),
        elapsed_text
    )
}

/// Creates a text-based progress bar
///
/// # Arguments
///
/// * `percent` - The progress percentage (0-100)
/// * `width` - The width of the progress bar in characters
///
/// # Returns
///
/// * A string representing a text-based progress bar
fn create_progress_bar(percent: f64, width: usize) -> ColoredString {
    let filled_width = ((percent / 100.0) * width as f64).round() as usize;
    let empty_width = width.saturating_sub(filled_width);

    let filled = "=".repeat(filled_width);
    let empty = " ".repeat(empty_width);
    let progress_bar = format!("{}{}", filled, empty);

    progress_bar.color(COLOR_PROGRESS).bold()
}

// ============================================================================
// TERMINAL WIDTH DETECTION AND ADAPTIVE OUTPUT
// ============================================================================

/// Default terminal width to use when detection fails
const DEFAULT_TERMINAL_WIDTH: usize = 80;

/// Minimum terminal width to consider for formatting
const MIN_TERMINAL_WIDTH: usize = 40;

/// Detects the current terminal width
///
/// This function attempts to detect the width of the terminal using
/// platform-specific commands. If detection fails, it returns a default width.
///
/// # Returns
///
/// * The detected terminal width, or DEFAULT_TERMINAL_WIDTH if detection fails
pub fn get_terminal_width() -> usize {
    // Try to get terminal width using stty
    if let Some(width) = get_terminal_width_stty() {
        return width;
    }

    // Try to get terminal width using tput
    if let Some(width) = get_terminal_width_tput() {
        return width;
    }

    // Fall back to default width
    DEFAULT_TERMINAL_WIDTH
}

/// Attempts to get terminal width using stty
///
/// # Returns
///
/// * Some(width) if successful, None otherwise
fn get_terminal_width_stty() -> Option<usize> {
    let output = Command::new("stty")
        .arg("size")
        .stdin(Stdio::inherit())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = output_str.trim().split_whitespace().collect();

    if parts.len() >= 2 {
        if let Ok(width) = parts[1].parse::<usize>() {
            return Some(width.max(MIN_TERMINAL_WIDTH));
        }
    }

    None
}

/// Attempts to get terminal width using tput
///
/// # Returns
///
/// * Some(width) if successful, None otherwise
fn get_terminal_width_tput() -> Option<usize> {
    let output = Command::new("tput")
        .arg("cols")
        .stdin(Stdio::inherit())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    if let Ok(width) = output_str.trim().parse::<usize>() {
        return Some(width.max(MIN_TERMINAL_WIDTH));
    }

    None
}

/// Truncates a string to fit within the specified width, adding an ellipsis if needed
///
/// # Arguments
///
/// * `text` - The text to truncate
/// * `max_width` - The maximum width
///
/// # Returns
///
/// * The truncated string
pub fn truncate_to_width(text: &str, max_width: usize) -> String {
    if text.len() <= max_width {
        return text.to_string();
    }

    if max_width <= 3 {
        return ".".repeat(max_width);
    }

    format!("{}...", &text[0..max_width - 3])
}

/// Creates a divider line that fits the terminal width
///
/// # Arguments
///
/// * `char` - The character to use for the divider (default: '=')
///
/// # Returns
///
/// * A divider string that fits the terminal width
pub fn create_adaptive_divider(char: Option<char>) -> ColoredString {
    let width = get_terminal_width();
    let divider_char = char.unwrap_or('=');
    divider_char.to_string().repeat(width).color(COLOR_DIVIDER)
}

/// Formats a table row with columns that adapt to terminal width
///
/// # Arguments
///
/// * `columns` - The column values
/// * `widths` - The desired column widths (percentages of terminal width)
///
/// # Returns
///
/// * A formatted table row
pub fn format_adaptive_table_row(columns: &[&str], widths: &[usize]) -> String {
    if columns.is_empty() || widths.is_empty() || columns.len() != widths.len() {
        return String::new();
    }

    let terminal_width = get_terminal_width();
    let mut result = String::new();
    let mut remaining_width = terminal_width;

    // Calculate actual column widths based on percentages
    let mut actual_widths = Vec::with_capacity(widths.len());
    for &width_percent in widths.iter().take(widths.len() - 1) {
        let width = (terminal_width * width_percent) / 100;
        actual_widths.push(width);
        remaining_width = remaining_width.saturating_sub(width);
    }

    // Last column gets remaining width
    actual_widths.push(remaining_width);

    // Format each column
    for (i, &column) in columns.iter().enumerate() {
        let width = *actual_widths.get(i).unwrap_or(&0);
        if width == 0 {
            continue;
        }

        let truncated = truncate_to_width(column, width);
        let padding = width.saturating_sub(truncated.len());
        result.push_str(&truncated);
        result.push_str(&" ".repeat(padding));
    }

    result
}

// ============================================================================
// FFMPEG COMMAND FORMATTING
// ============================================================================

/// Formats an FFmpeg command with improved visual structure
///
/// # Arguments
///
/// * `command` - The FFmpeg command as a vector of strings
/// * `is_sample` - Whether this is a sample command (less verbose output)
///
/// # Returns
///
/// * A formatted FFmpeg command string
pub fn format_ffmpeg_command(command: &[String], is_sample: bool) -> String {
    if command.is_empty() {
        return String::new();
    }

    if is_sample {
        // For sample commands, use simpler formatting
        return format!(
            "🔧 {} {}",
            "FFmpeg command (sample):".color(COLOR_LABEL).bold(),
            command.join(" ").color(COLOR_VALUE)
        );
    }

    // Group related flags for better readability
    let mut grouped_command = Vec::new();
    let mut current_group = Vec::new();
    let mut current_group_type = "";

    // Define group types for common FFmpeg flags
    let group_types = [
        ("input", vec!["-i", "-f"]),
        ("video", vec!["-c:v", "-crf", "-preset", "-tune", "-profile:v", "-pix_fmt", "-vf", "-filter:v"]),
        ("audio", vec!["-c:a", "-b:a", "-ac", "-ar", "-filter:a"]),
        ("output", vec!["-y", "-movflags"]),
        ("hwaccel", vec!["-hwaccel", "-hwaccel_output_format"]),
    ];

    // Helper function to determine group type
    let get_group_type = |arg: &str| -> &str {
        for (group_type, prefixes) in &group_types {
            if prefixes.iter().any(|&prefix| arg.starts_with(prefix)) {
                return group_type;
            }
        }
        "other"
    };

    // Group related flags
    for arg in command {
        if arg.starts_with('-') {
            let arg_group_type = get_group_type(arg);

            if !current_group.is_empty() && current_group_type != arg_group_type {
                grouped_command.push(current_group.clone());
                current_group.clear();
            }

            current_group_type = arg_group_type;
        }

        current_group.push(arg.clone());

        // If this is a file path (not starting with '-'), add it and start a new group
        if !arg.starts_with('-') && !current_group.is_empty() {
            grouped_command.push(current_group.clone());
            current_group.clear();
            current_group_type = "";
        }
    }

    // Add any remaining items
    if !current_group.is_empty() {
        grouped_command.push(current_group);
    }

    // Format the grouped command
    let mut formatted = String::new();
    formatted.push_str(&format!("🎬 {}\n", "FFmpeg Command:".color(COLOR_HEADER).bold()));

    for (i, group) in grouped_command.iter().enumerate() {
        if i > 0 {
            formatted.push_str("\n");
        }

        formatted.push_str("  ");

        for (j, arg) in group.iter().enumerate() {
            if j > 0 {
                formatted.push_str(" ");
            }

            if arg.starts_with('-') {
                formatted.push_str(&arg.color(COLOR_LABEL).to_string());
            } else {
                formatted.push_str(&arg.color(COLOR_VALUE).to_string());
            }
        }
    }

    formatted
}

/// Formats a configuration summary with visual grouping
///
/// # Arguments
///
/// * `config_items` - A vector of (section, key, value, is_default) tuples
///
/// # Returns
///
/// * A formatted configuration summary
pub fn format_config_summary(config_items: &[(&str, &str, &str, bool)]) -> String {
    if config_items.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    result.push_str(&format!("{}\n", "Configuration:".color(COLOR_HEADER).bold()));

    // Group by section
    let mut current_section = "";

    for &(section, key, value, is_default) in config_items {
        if section != current_section {
            if !current_section.is_empty() {
                result.push_str("\n");
            }

            result.push_str(&format!("  {}\n", section.color(COLOR_SUBSECTION).bold()));
            current_section = section;
        }

        let value_str = if is_default {
            value.color(COLOR_VALUE).to_string()
        } else {
            value.color(COLOR_HIGHLIGHT).bold().to_string()
        };

        result.push_str(&format!("    {:<20} {}\n", key.color(COLOR_LABEL), value_str));
    }

    result
}

// ============================================================================
// GRAIN ANALYSIS OUTPUT FORMATTING
// ============================================================================

/// Formats a grain analysis phase header with improved visual structure
///
/// # Arguments
///
/// * `phase_number` - The phase number
/// * `description` - The phase description
///
/// # Returns
///
/// * A formatted grain analysis phase header
pub fn format_grain_analysis_phase(phase_number: usize, description: &str) -> String {
    format!(
        "{}\n🔍 {} {}\n{}",
        format_short_divider(),
        format!("Phase {}:", phase_number).color(COLOR_PHASE).bold(),
        description.color(COLOR_PHASE),
        format_short_divider()
    )
}

/// Formats a grain level result with visual indicators
///
/// # Arguments
///
/// * `level` - The grain level name
/// * `size_mb` - The size in MB
/// * `is_baseline` - Whether this is the baseline (no denoising) level
/// * `is_selected` - Whether this level was selected as optimal
///
/// # Returns
///
/// * A formatted grain level result
pub fn format_grain_level_result(level: &str, size_mb: f64, is_baseline: bool, is_selected: bool) -> String {
    let indicator = if is_selected { "✓ " } else { "  " };
    let level_color = if is_baseline {
        COLOR_INFO
    } else if is_selected {
        COLOR_HIGHLIGHT
    } else {
        COLOR_VALUE
    };

    format!(
        "{}{}  {:>8.2} MB",
        indicator,
        format!("{:<15}", level).color(level_color).bold(),
        size_mb
    )
}

/// Creates a simple ASCII chart showing relative file sizes for grain levels
///
/// # Arguments
///
/// * `sizes` - A vector of (level_name, size_mb) pairs
/// * `selected_level` - The name of the selected level
///
/// # Returns
///
/// * A formatted ASCII chart
pub fn format_grain_level_chart(sizes: &[(&str, f64)], selected_level: &str) -> String {
    if sizes.is_empty() {
        return String::new();
    }

    // Find the maximum size for scaling
    let max_size = sizes.iter().map(|(_, size)| *size).fold(0.0, f64::max);
    if max_size <= 0.0 {
        return String::new();
    }

    let terminal_width = get_terminal_width();
    let max_bar_width = terminal_width.saturating_sub(30);

    let mut result = String::new();
    result.push_str(&format!("{}\n", "Grain Level Comparison:".color(COLOR_HEADER).bold()));

    for &(level, size) in sizes {
        let is_selected = level == selected_level;
        let relative_size = size / max_size;
        let bar_width = (relative_size * max_bar_width as f64).round() as usize;

        let bar = "█".repeat(bar_width);
        let bar_color = if is_selected {
            COLOR_HIGHLIGHT
        } else if level == "Baseline" {
            COLOR_INFO
        } else {
            COLOR_VALUE
        };

        let indicator = if is_selected { "✓ " } else { "  " };

        result.push_str(&format!(
            "{}{}  {:>8.2} MB  {}\n",
            indicator,
            format!("{:<15}", level).color(bar_color).bold(),
            size,
            bar.color(bar_color)
        ));
    }

    result
}

/// Formats a grain analysis summary with improved visual structure
///
/// # Arguments
///
/// * `detected_level` - The detected grain level
/// * `level_sizes` - A vector of (level_name, size_mb) pairs for all tested levels
///
/// # Returns
///
/// * A formatted grain analysis summary
pub fn format_grain_analysis_summary(detected_level: &str, level_sizes: &[(&str, f64)]) -> String {
    let mut result = String::new();

    // Add header
    result.push_str(&format_section("Grain Analysis Results"));

    // Add detected level
    result.push_str(&format!(
        "\n  {} {}\n",
        "Detected Grain Level:".color(COLOR_LABEL).bold(),
        detected_level.color(COLOR_HIGHLIGHT).bold()
    ));

    // Add chart
    result.push_str("\n");
    result.push_str(&format_grain_level_chart(level_sizes, detected_level));

    // Add explanation
    result.push_str("\n");
    result.push_str(&format!(
        "  {} {}\n",
        "Explanation:".color(COLOR_LABEL).bold(),
        "The optimal grain level provides the best balance between file size reduction and video quality.".color(COLOR_INFO)
    ));

    result
}

/// Formats a grain level description with detailed explanation
///
/// # Arguments
///
/// * `level` - The grain level name
/// * `description` - The description of what this level does
/// * `technical_details` - Technical details about the denoising parameters
///
/// # Returns
///
/// * A formatted grain level description
pub fn format_grain_level_description(level: &str, description: &str, technical_details: &str) -> String {
    format!(
        "  {} {}\n    {}\n    {}\n",
        level.color(COLOR_LABEL).bold(),
        "Grain Level".color(COLOR_LABEL),
        description.color(COLOR_INFO),
        format!("Technical: {}", technical_details).color(COLOR_DETAIL)
    )
}
