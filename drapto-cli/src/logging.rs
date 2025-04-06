// drapto-cli/src/logging.rs
//
// Handles logging setup and the combined console/file log callback.

use std::cell::Cell;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::{Duration, Instant}; // Added for throttling
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

// --- Helper Functions (Timestamp) ---
pub fn get_timestamp() -> String {
    chrono::Local::now().format("%Y%m%d_%H%M%S").to_string()
}

// --- Log Callback Creation ---
// Creates the logging closure that writes to both console (with color) and a file.
pub fn create_log_callback(
    log_file: File,
) -> Result<Box<dyn FnMut(&str)>, Box<dyn std::error::Error>> {
    let mut logger = Box::new(BufWriter::new(log_file)); // Using Box for simplicity with closure
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);

    // Use Cell to allow modifying state within FnMut closure
    let last_was_progress = Cell::new(false);
    let last_progress_file_log_time = Cell::new(None::<Instant>); // Track last file log time for progress
    let throttle_duration = Duration::from_secs(30); // Throttle interval

    let log_callback = move |msg: &str| {
        // --- File Logging (Throttled for progress) ---
        let is_progress = msg.contains('\r'); // Determine this early
        let should_log_to_file = if is_progress {
            let now = Instant::now();
            match last_progress_file_log_time.get() {
                Some(last_time) if now.duration_since(last_time) < throttle_duration => {
                    false // Throttle: Too soon since last progress log
                }
                _ => {
                    last_progress_file_log_time.set(Some(now)); // Update time and allow log
                    true
                }
            }
        } else {
            true // Always log non-progress messages
        };
        if should_log_to_file {
            // Write the raw message to the log file
            if is_progress {
                // Always add a newline for progress messages in the log file
                // This makes them visible on separate lines, effectively replacing the \r
                writeln!(logger, "{}", msg).ok();
            } else {
                // For non-progress messages, use the previous logic:
                // Write as-is if it already has a delimiter, otherwise add one.
                if msg.ends_with('\n') || msg.ends_with('\r') {
                    write!(logger, "{}", msg).ok();
                } else {
                    writeln!(logger, "{}", msg).ok();
                }
            }
            logger.flush().ok(); // Flush file buffer
        }

        // --- Console Logging (Colored) ---
        // Use the already determined is_progress
        // Note: Console logic below remains unchanged and uses the same `is_progress` value
        // File logging logic moved above

        // --- Console Logging (Colored) ---
        // `is_progress` is already defined above
        let msg_trimmed = msg.trim_end(); // Use trimmed for console logic

        if is_progress {
            // For progress, write directly, assuming HandBrake handles terminal control
            // Style HandBrakeCLI progress lines as Blue
            // Explicitly set Blue foreground, and ensure not bold/dimmed
            stdout
                .set_color(
                    ColorSpec::new()
                        .set_fg(Some(Color::Blue))
                        .set_bold(false)
                        .set_dimmed(false)
                        .set_intense(false), // Also ensure not intense
                )
                .ok();
            write!(&mut stdout, "{}", msg).ok(); // Print original message
            stdout.reset().ok(); // Reset color immediately after writing
            stdout.flush().ok(); // Flush console buffer
            last_was_progress.set(true);
        } else {
            // For normal messages, handle potential preceding progress line
            if last_was_progress.get() {
                writeln!(&mut stdout).ok(); // Move to the next line after progress
            }

            // --- Apply Enhanced Styling ---
            let mut handled = false; // Flag to check if we printed already

            // Define prefix arrays for styling rules
            let bold_label_prefixes = [
                "Input path:",
                "Output directory:",
                "Log directory:",
                "Main log file:",
                "Total encode execution time:",
                "Drapto Encode Run Finished:",
                "Drapto Encode Run Started:",
            ];
            let summary_value_prefixes = [
                // Moved definition here
                "  Encode time: ",
                "  Input size:  ",
                "  Output size: ",
                "  Reduced by:  ", // Note spaces for alignment
            ];

            // Style: Bold Labels, Normal Values (Initial Info & Final Timing)
            for prefix in bold_label_prefixes {
                if msg_trimmed.starts_with(prefix) {
                    if let Some(value) = msg_trimmed.strip_prefix(prefix) {
                        stdout.set_color(ColorSpec::new().set_bold(true)).ok(); // Bold label
                        write!(&mut stdout, "{}", prefix).ok();
                        stdout.reset().ok(); // Reset for value
                        writeln!(&mut stdout, "{}", value).ok();
                        handled = true;
                        break;
                    }
                }
            }

            // Style: Normal Labels, Bold Values (Summary Details)
            if !handled {
                // summary_value_prefixes is now defined above
                for prefix in summary_value_prefixes {
                    if msg_trimmed.starts_with(prefix) {
                        if let Some(value) = msg_trimmed.strip_prefix(prefix) {
                            write!(&mut stdout, "{}", prefix).ok(); // Normal label
                            stdout.set_color(ColorSpec::new().set_bold(true)).ok(); // Bold value
                            writeln!(&mut stdout, "{}", value).ok();
                            stdout.reset().ok(); // Reset after value
                            handled = true;
                            break;
                        }
                    }
                }
            }

            // Style: Success Count (Bold Green Number)
            if !handled && msg_trimmed.starts_with("Successfully encoded ") {
                if let Some(rest) = msg_trimmed.strip_prefix("Successfully encoded ") {
                    if let Some((count_str, _suffix)) = rest.split_once(" file(s).") {
                        // Prefix suffix with _
                        write!(&mut stdout, "Successfully encoded ").ok();
                        stdout
                            .set_color(
                                ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true),
                            )
                            .ok();
                        write!(&mut stdout, "{}", count_str).ok();
                        stdout.reset().ok();
                        stdout
                            .set_color(ColorSpec::new().set_fg(Some(Color::Green)))
                            .ok(); // Green for suffix
                        writeln!(&mut stdout, " file(s).").ok();
                        stdout.reset().ok();
                        handled = true;
                    }
                }
            }
            // Style: Status Prefixes ([OK], [INFO], etc.)
            if !handled {
                let status_prefixes = [
                    ("[OK]", ColorSpec::new().set_fg(Some(Color::Green)).clone()),
                    ("[INFO]", ColorSpec::new().set_fg(Some(Color::Cyan)).clone()),
                    (
                        "[WARN]",
                        ColorSpec::new().set_fg(Some(Color::Yellow)).clone(),
                    ),
                    (
                        "[ERROR]",
                        ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true).clone(),
                    ),
                    (
                        "[FAIL]",
                        ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true).clone(),
                    ),
                    (
                        "[DEBUG]",
                        ColorSpec::new().set_fg(Some(Color::Magenta)).clone(),
                    ),
                    (
                        "[TRACE]",
                        ColorSpec::new().set_fg(Some(Color::Blue)).clone(),
                    ),
                ];
                for (prefix, spec) in status_prefixes {
                    // Add space to prefix for matching to avoid partial matches like "[INFO]rmation"
                    let prefix_with_space = format!("{} ", prefix);
                    if msg_trimmed.starts_with(&prefix_with_space) {
                        if let Some(rest) = msg_trimmed.strip_prefix(&prefix_with_space) {
                            stdout.set_color(&spec).ok();
                            write!(&mut stdout, "{}", prefix).ok(); // Write only the prefix colored
                            stdout.reset().ok();
                            writeln!(&mut stdout, " {}", rest).ok(); // Write the rest uncolored (with space)
                            handled = true;
                            break;
                        }
                    }
                    // Handle cases where the prefix might be the entire message (less likely but possible)
                    else if msg_trimmed == prefix {
                        stdout.set_color(&spec).ok();
                        writeln!(&mut stdout, "{}", prefix).ok();
                        stdout.reset().ok();
                        handled = true;
                        break;
                    }
                }
            }

            // Style: Specific Lines ("Processing:", "External dependency check passed.")
            if !handled {
                if msg_trimmed == "External dependency check passed." {
                    stdout
                        .set_color(ColorSpec::new().set_fg(Some(Color::Green)))
                        .ok();
                    writeln!(&mut stdout, "{}", msg_trimmed).ok();
                    stdout.reset().ok();
                    handled = true;
                } else if let Some(filename) = msg_trimmed.strip_prefix("Processing: ") {
                    stdout.set_color(ColorSpec::new().set_bold(true)).ok(); // Bold "Processing:"
                    write!(&mut stdout, "Processing: ").ok();
                    stdout.reset().ok(); // Reset for filename
                    writeln!(&mut stdout, "{}", filename).ok();
                    handled = true;
                }
            }

            // --- Fallback to Previous Simpler Styling for remaining unhandled cases ---
            if !handled {
                let mut color_spec = ColorSpec::new();
                match msg_trimmed {
                    // Separators
                    m if m.starts_with("===") || m.starts_with("---") => {
                        color_spec.set_fg(Some(Color::Cyan)).set_bold(true);
                    }
                    // Headers
                    m if m.starts_with("Encoding Summary:") => {
                        color_spec.set_bold(true); // Bold White
                    }
                    // Warnings (already handled FATAL CORE ERROR, Success, specific labels/values)
                    m if m.starts_with("No processable .mkv files")
                        || m.starts_with("No files were successfully encoded.") =>
                    {
                        color_spec.set_fg(Some(Color::Yellow));
                    }
                    // Default case: Check for summary filename or just print default
                    _ => {
                        // Heuristic for summary filename: Not indented, not handled above
                        if !msg_trimmed.starts_with(' ')
                            && !msg_trimmed.starts_with("===")
                            && !msg_trimmed.starts_with("---")
                            && !msg_trimmed.starts_with("Encoding Summary:")
                        // Add other known non-filename prefixes if needed
                        {
                            // Assume it's a filename in the summary
                            color_spec.set_bold(true); // Bold White
                        }
                        // Otherwise, use default color spec (covers "Found X files...", etc.)
                    }
                }
                // Print lines handled by this fallback logic
                stdout.set_color(&color_spec).ok();
                writeln!(&mut stdout, "{}", msg_trimmed).ok();
                stdout.reset().ok(); // Reset to default colors
            }

            last_was_progress.set(false);
            stdout.flush().ok(); // Flush console buffer
        }
    };

    Ok(Box::new(log_callback))
}